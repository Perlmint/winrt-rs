use super::*;
use crate::{TableIndex, TypeReader};

#[derive(Copy, Clone)]
pub struct MethodDef {
    pub row: Row,
}

impl MethodDef {
    pub fn flags(&self) -> MethodFlags {
        MethodFlags(TypeReader::get().u32(self.row, 2))
    }

    pub fn parent(&self) -> TypeDef {
        TypeDef {
            row: TypeReader::get().upper_bound(
                self.row.file_index,
                TableIndex::TypeDef,
                6,
                self.row.index,
            ),
        }
    }

    pub fn params(&self) -> impl Iterator<Item = Param> + '_ {
        TypeReader::get()
            .list(self.row, TableIndex::Param, 5)
            .map(move |row| Param {
                row,
            })
    }

    pub fn name(&self) -> &str {
        TypeReader::get().str(self.row, 3)
    }

    pub fn sig(&self) -> Blob {
        TypeReader::get().blob(self.row, 4)
    }

    pub fn category(&self) -> MethodCategory {
        if self.flags().special() {
            let name = self.name();

            if name.starts_with("get") {
                MethodCategory::Get
            } else if name.starts_with("put") {
                MethodCategory::Set
            } else if name.starts_with("add") {
                MethodCategory::Add
            } else if name.starts_with("remove") {
                MethodCategory::Remove
            } else {
                // A delegate's 'Invoke' method is "special" but lacks a preamble.
                MethodCategory::Normal
            }
        } else {
            MethodCategory::Normal
        }
    }

    pub fn attributes(&self) -> impl Iterator<Item = Attribute> + '_ {
        TypeReader::get()
            .equal_range(
                self.row.file_index,
                TableIndex::CustomAttribute,
                0,
                HasAttribute::MethodDef(*self).encode(),
            )
            .map(move |row| Attribute {
                row,
            })
    }
}

impl std::fmt::Debug for MethodDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MethodDef").field("row", &self.row).finish()
    }
}

impl PartialEq for MethodDef {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row
    }
}

impl Eq for MethodDef {}

impl Ord for MethodDef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.row.cmp(&other.row)
    }
}

impl PartialOrd for MethodDef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}