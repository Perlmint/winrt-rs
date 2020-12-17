use crate::*;
use squote::{quote, TokenStream};

#[derive(Debug)]
pub struct Function {
    pub name: TypeName,
}

impl Function {
    pub fn from_type_name(name: TypeName) -> Self {
        Self { name }
    }

    pub fn gen(&self) -> TokenStream {
        quote! {
           
        }
    }

    pub fn dependencies(&self) -> Vec<winmd::TypeDef> {
        Vec::new()
    }
}
