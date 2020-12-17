use crate::*;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::PathBuf;

type TypeMap = BTreeMap<&'static str, BTreeMap<&'static str, TypeRow>>;

/// A reader of type information from Windows Metadata
pub struct TypeReader {
    pub(crate) files: Vec<File>,
    pub(crate) types: TypeMap,
}

#[derive(Copy, Clone)]
pub enum TypeRow {
    TypeDef(TypeDef),
    MethodDef(MethodDef),
    Field(Field),
}

impl TypeReader {
    pub fn get() -> &'static Self {
        use std::sync::atomic::{AtomicPtr, Ordering};

        static mut SHARED: AtomicPtr<TypeReader> = 
            AtomicPtr::new(std::ptr::null_mut());

        let mut ptr = unsafe { SHARED.load(Ordering::Relaxed) };

        if ptr.is_null() {
            ptr = Box::into_raw(Box::new(Self { files: winmd_files(), types: BTreeMap::new() }));
            unsafe { *SHARED.get_mut() = ptr };

            let reader: &mut Self = unsafe { &mut *ptr };
            let mut types = BTreeMap::new();

            fn insert(types: &mut TypeMap, (namespace, name): (&'static str, &'static str), row: TypeRow) {
                types
                    .entry(namespace)
                    .or_default()
                    .entry(name)
                    .or_insert(row);
            }

            fn remove(types: &mut TypeMap, (namespace, name): (&str, &str)) {
                if let Some(value) = types.get_mut(namespace) {
                    value.remove(name);
                }
            }

            for (file_index, file) in reader.files.iter().enumerate() {
                let row_count = file.type_def_table().row_count;
    
                for row in 0..row_count {
                    let row = Row::new(row, TableIndex::TypeDef, file_index as u16);
                    let def = TypeDef { row };
                    let name = def.name();

                    if name == ("", "<Module>") {
                        continue;
                    }
    
                    match def.category() {
                        TypeCategory::Interface
                        | TypeCategory::Enum
                        | TypeCategory::Delegate
                        | TypeCategory::Struct => insert(&mut types, name, TypeRow::TypeDef(def)),
                        TypeCategory::Class => {
                            insert(&mut types, name, TypeRow::TypeDef(def));
                            if !def.is_winrt() {
                                for field in def.fields() {
                                    insert(&mut types, name, TypeRow::Field(field));
                                }
                                for method in def.methods() {
                                    insert(&mut types, name, TypeRow::MethodDef(method));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            remove(&mut types, ("Windows.Foundation", "HResult"));
            remove(&mut types, ("Windows.Win32", "IUnknown"));
    
            // TODO: remove once this is fixed: https://github.com/microsoft/win32metadata/issues/30
            remove(&mut types, ("Windows.Win32", "CFunctionDiscoveryNotificationWrapper"));

            reader.types = types;
        }

        unsafe { &*ptr }
    }




    pub fn find_lowercase_namespace(&self, lowercase: &str) -> Option<&str> {
        self.types.keys().find(|namespace|namespace.to_lowercase() == lowercase).map(|namespace|*namespace)
    }

    pub fn find_type(&self, (namespace, name): (&str, &str)) -> Option<TypeRow> { // return Option<&TypeRow> to avoid copy
        self.types.get(namespace).and_then(|types|types.get(name)).map(|row|*row)
    }

    pub fn expect_type_def(&self, (namespace, name): (&str, &str)) -> TypeDef {
        if let Some(TypeRow::TypeDef(def)) = self.find_type((namespace, name)) {
            return def
        }

        panic!("Could not find type `{}.{}`", namespace, name);
    }

    /// Read a [`u32`] value from a specific [`Row`] and column
    pub fn u32(&self, row: Row, column: u32) -> u32 {
        let file = &self.files[row.file_index as usize];
        let table = &file.tables[row.table_index as usize];
        let offset = table.data + row.index * table.row_size + table.columns[column as usize].0;
        match table.columns[column as usize].1 {
            1 => file.bytes.copy_as::<u8>(offset) as u32,
            2 => file.bytes.copy_as::<u16>(offset) as u32,
            4 => file.bytes.copy_as::<u32>(offset) as u32,
            _ => file.bytes.copy_as::<u64>(offset) as u32,
        }
    }

    /// Read a [`&str`] value from a specific [`Row`] and column
    pub fn str(&self, row: Row, column: u32) -> &str {
        let file = &self.files[row.file_index as usize];
        let offset = (file.strings + self.u32(row, column)) as usize;
        let last = file.bytes[offset..]
            .iter()
            .position(|c| *c == b'\0')
            .unwrap();
        std::str::from_utf8(&file.bytes[offset..offset + last]).unwrap()
    }

    /// Read a `T: Decode` value from a specific [`Row`] and column
    pub(crate) fn decode<T: Decode>(&'static self, row: Row, column: u32) -> T {
        T::decode(self.u32(row, column), row.file_index)
    }

    pub(crate) fn list(
        &self,
        row: Row,
        table: TableIndex,
        column: u32,
    ) -> impl Iterator<Item = Row> {
        let file = &self.files[row.file_index as usize];
        let first = self.u32(row, column) - 1;

        let last = if row.index + 1 < file.tables[row.table_index as usize].row_count {
            self.u32(row.next(), column) - 1
        } else {
            file.tables[table as usize].row_count
        };

        (first..last).map(move |value| Row::new(value, table, row.file_index))
    }

    /// Read a blob for a given row and column
    pub fn blob(&'static self, row: Row, column: u32) -> Blob {
        let file = &self.files[row.file_index as usize];
        let offset = (file.blobs + self.u32(row, column)) as usize;
        let initial_byte = file.bytes[offset];
        let (mut blob_size, blob_size_bytes) = match initial_byte >> 5 {
            0..=3 => (initial_byte & 0x7f, 1),
            4..=5 => (initial_byte & 0x3f, 2),
            6 => (initial_byte & 0x1f, 4),
            _ => panic!("Invalid blob size"),
        };
        for byte in &file.bytes[offset + 1..offset + blob_size_bytes] {
            blob_size = blob_size.checked_shl(8).unwrap_or(0) + byte;
        }
        Blob {
            file_index: row.file_index,
            offset: offset + blob_size_bytes,
        }
    }

    pub(crate) fn equal_range(
        &self,
        file: u16,
        table: TableIndex,
        column: u32,
        value: u32,
    ) -> impl Iterator<Item = Row> {
        let (first, last) = self.equal_range_of(
            table,
            file,
            0,
            self.files[file as usize].tables[table as usize].row_count,
            column,
            value,
        );

        (first..last).map(move |row| Row::new(row, table, file))
    }

    fn lower_bound_of(
        &self,
        table: TableIndex,
        file: u16,
        mut first: u32,
        last: u32,
        column: u32,
        value: u32,
    ) -> u32 {
        let mut count = last - first;
        while count > 0 {
            let count2 = count / 2;
            let middle = first + count2;
            if self.u32(Row::new(middle, table, file), column) < value {
                first = middle + 1;
                count -= count2 + 1;
            } else {
                count = count2;
            }
        }
        first
    }

    pub(crate) fn upper_bound(&self, file: u16, table: TableIndex, column: u32, value: u32) -> Row {
        Row::new(
            self.upper_bound_of(
                table,
                file,
                0,
                self.files[file as usize].tables[table as usize].row_count,
                column,
                value,
            ),
            table,
            file,
        )
    }

    fn upper_bound_of(
        &self,
        table: TableIndex,
        file: u16,
        mut first: u32,
        last: u32,
        column: u32,
        value: u32,
    ) -> u32 {
        let mut count = last - first;

        while count > 0 {
            let count2 = count / 2;
            let middle = first + count2;
            if value < self.u32(Row::new(middle, table, file), column) {
                count = count2
            } else {
                first = middle + 1;
                count -= count2 + 1;
            }
        }

        first
    }

    fn equal_range_of(
        &self,
        table: TableIndex,
        file: u16,
        mut first: u32,
        mut last: u32,
        column: u32,
        value: u32,
    ) -> (u32, u32) {
        let mut count = last - first;
        loop {
            if count == 0 {
                last = first;
                break;
            }
            let count2 = count / 2;
            let middle = first + count2;
            let middle_value = self.u32(Row::new(middle, table, file), column);
            match middle_value.cmp(&value) {
                Ordering::Less => {
                    first = middle + 1;
                    count -= count2 + 1;
                }
                Ordering::Greater => count = count2,
                Ordering::Equal => {
                    let first2 = self.lower_bound_of(table, file, first, middle, column, value);
                    first += count;
                    last = self.upper_bound_of(table, file, middle + 1, first, column, value);
                    first = first2;
                    break;
                }
            }
        }
        (first, last)
    }
}

fn winmd_files() -> Vec<File> {
        let mut windows_path = workspace_windows_dir();
        windows_path.push("winmd");
    
        let mut files = vec![];
        push_winmd_files(windows_path, &mut files);
    
        // If at this point the files vector is still empty then go and grab the winmd files from WinMetadata
        // to make it easy for developers to get started without having to figure out where to get metadata.
    
        if files.is_empty() {
            if let Ok(dir) = std::env::var("windir") {
                let mut dir = std::path::PathBuf::from(dir);
                dir.push(SYSTEM32);
                dir.push("winmetadata");
    
                push_winmd_files(dir, &mut files);
            }
        }

        files
}

fn push_winmd_files(dir: std::path::PathBuf, paths: &mut Vec<File>) {
    if let Ok(files) = std::fs::read_dir(dir) {
        for file in files.filter_map(|file| file.ok()) {
            if let Ok(file_type) = file.file_type() {
                if file_type.is_file() {
                    let path = file.path();
                    if let Some("winmd") = path.extension().and_then(|extension| extension.to_str())
                    {
                        paths.push(File::new(file.path()));
                    }
                }
            }
        }
    }
}

#[cfg(target_pointer_width = "64")]
const SYSTEM32: &str = "System32";

#[cfg(target_pointer_width = "32")]
const SYSTEM32: &str = "SysNative";
