use super::*;
use crate::{TableIndex, TypeReader};

#[derive(Copy, Clone)]
pub struct Field {
    pub row: Row,
}

impl Field {
    pub fn name(&self) -> &'static str {
        TypeReader::get().str(self.row, 1)
    }

    pub fn sig(&self) -> Blob {
        TypeReader::get().blob(self.row, 2)
    }

    pub fn flags(&self) -> FieldFlags {
        FieldFlags(TypeReader::get().u32(self.row, 0))
    }

    pub fn constants(&self) -> impl Iterator<Item = Constant> + '_ {
        TypeReader::get()
            .equal_range(
                self.row.file_index,
                TableIndex::Constant,
                1,
                HasConstant::Field(*self).encode(),
            )
            .map(move |row| Constant {
                row,
            })
    }
}

impl std::fmt::Debug for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Field").field("row", &self.row).finish()
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row
    }
}

impl Eq for Field {}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.row.cmp(&other.row)
    }
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
