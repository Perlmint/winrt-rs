use super::*;
use crate::TypeReader;

#[derive(Copy, Clone)]
pub struct MemberRef {
    pub row: Row,
}

impl MemberRef {
    pub fn parent(&self) -> MemberRefParent {
        TypeReader::get().decode(self.row, 0)
    }

    pub fn name(&self) -> &str {
        TypeReader::get().str(self.row, 1)
    }
}

impl std::fmt::Debug for MemberRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemberRef").field("row", &self.row).finish()
    }
}
