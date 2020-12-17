use crate::*;
use squote::{quote, TokenStream};

#[derive(Debug)]
pub struct ComClass {
    pub name: TypeName,
}

impl ComClass {
    pub fn from_type_name(name: TypeName) -> Self {
        Self { name }
    }

    pub fn gen(&self) -> TokenStream {
        //let name = self.name.gen();

        // TODO: generate constant for CLSID

        quote! {
           
        }
    }

    pub fn dependencies(&self) -> Vec<winmd::TypeDef> {
        Vec::new()
    }
}
