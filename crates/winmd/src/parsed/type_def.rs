use super::*;
use crate::{TableIndex, TypeReader};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeDef {
    pub row: Row,
}

impl TypeDef {
    pub fn flags(&self) -> TypeFlags {
        TypeFlags(TypeReader::get().u32(self.row, 0))
    }

    pub fn name(&self) -> (&'static str, &'static str) {
        let reader = TypeReader::get();
        (reader.str(self.row, 2), reader.str(self.row, 1))
    }

    pub fn extends(&self) -> TypeDefOrRef {
        TypeReader::get().decode(self.row, 3)
    }

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        TypeReader::get()
            .list(self.row, TableIndex::Field, 4)
            .map(move |row| Field {
                row,
            })
    }

    pub fn methods(&self) -> impl Iterator<Item = MethodDef> + '_ {
        TypeReader::get()
            .list(self.row, TableIndex::MethodDef, 5)
            .map(move |row| MethodDef {
                row,
            })
    }

    pub fn generics(&self) -> impl Iterator<Item = GenericParam> + '_ {
        TypeReader::get()
            .equal_range(
                self.row.file_index,
                TableIndex::GenericParam,
                2,
                TypeOrMethodDef::TypeDef(*self).encode(),
            )
            .map(move |row| GenericParam {
                row,
            })
    }

    pub fn interfaces(&self) -> impl Iterator<Item = InterfaceImpl> + '_ {
        TypeReader::get()
            .equal_range(
                self.row.file_index,
                TableIndex::InterfaceImpl,
                0,
                self.row.index + 1,
            )
            .map(move |row| InterfaceImpl {
                row,
            })
    }

    pub fn attributes(&self) -> impl Iterator<Item = Attribute> + '_ {
        TypeReader::get()
            .equal_range(
                self.row.file_index,
                TableIndex::CustomAttribute,
                0,
                HasAttribute::TypeDef(*self).encode(),
            )
            .map(move |row| Attribute {
                row,
            })
    }

    pub fn has_attribute(&self, name: (&str, &str)) -> bool {
        self.attributes().any(|attribute| attribute.name() == name)
    }

    pub fn is_winrt(&self) -> bool {
        self.flags().windows_runtime()
    }

    pub fn category(&self) -> TypeCategory {
        if self.flags().interface() {
            TypeCategory::Interface
        } else {
            match self.extends().name() {
                ("System", "Enum") => TypeCategory::Enum,
                ("System", "MulticastDelegate") => TypeCategory::Delegate,
                ("System", "Attribute") => TypeCategory::Attribute,
                ("System", "ValueType") => {
                    if self.has_attribute(("Windows.Foundation.Metadata", "ApiContractAttribute")) {
                        TypeCategory::Contract
                    } else {
                        TypeCategory::Struct
                    }
                }
                _ => TypeCategory::Class,
            }
        }
    }

    pub fn underlying_type(&self) -> ElementType {
        for field in self.fields() {
            if let Some(constant) = field.constants().next() {
                return constant.value_type();
            } else {
                let blob = &mut field.sig();
                blob.read_unsigned();
                blob.read_modifiers();

                blob.read_expected(0x1D);
                blob.read_modifiers();

                return ElementType::from_blob(blob);
            }
        }

        panic!("TypeDef::underlying_type {:?}", self.name());
    }
}

impl std::fmt::Debug for TypeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: add name to debug output
        f.debug_struct("TypeDef").field("row", &self.row).finish()
    }
}
