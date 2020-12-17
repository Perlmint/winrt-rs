use super::*;
use crate::TypeReader;

#[derive(Copy, Clone)]
pub struct TypeRef {
    pub row: Row,
}

impl TypeRef {
    pub fn name(&self) -> (&'static str, &'static str) {
        (TypeReader::get().str(self.row, 2), TypeReader::get().str(self.row, 1))
    }

    pub fn resolve(&self) -> TypeDef {
        TypeReader::get().expect_type_def(self.name())
    }
}

impl std::fmt::Debug for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeRef").field("row", &self.row).finish()
    }
}
