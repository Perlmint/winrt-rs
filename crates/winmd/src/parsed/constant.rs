use crate::*;

#[derive(Copy, Clone)]
pub struct Constant {
    pub row: Row,
}

impl Constant {
    pub fn value_type(&self) -> ElementType {
        ElementType::from_code(TypeReader::get().u32(self.row, 0))
    }

    pub fn value(&self) -> Blob {
        TypeReader::get().blob(self.row, 2)
    }
}

impl std::fmt::Debug for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Constant").field("row", &self.row).finish()
    }
}
