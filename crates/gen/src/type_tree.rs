use crate::*;
use rayon::prelude::*;
use squote::TokenStream;
use std::collections::BTreeSet;

/// A namespaced tree of types
#[derive(Default)]
pub struct TypeTree {
    pub types: Vec<TypeDefinition>,
    pub namespaces: TypeNamespaces,
    pub include_foundation: bool,
}

impl From<TypeLimits> for TypeTree {
    fn from(limits: TypeLimits) -> Self {
        let mut tree = Self::default();
        let mut set = BTreeSet::new();

        for limit in limits.namespaces() {
            match &limit.limit {
                TypeLimit::All => {
                    limits.cache.0.get::<str>(&limit.namespace).unwrap().values().for_each(|row|{
                        tree.insert_if(&mut set, row);
                    });
                }
                TypeLimit::Some(types) => {

                }
            }
        }

        tree
    }
}

impl TypeTree {
    // pub fn from_limits(reader: &'static winmd::TypeReader, limits: &TypeLimits) -> Self {
    //     let mut tree = TypeTree::default();
    //     let mut set = std::collections::BTreeSet::new();

    //     for limit in limits.limits() {
    //         match &limit.limit {
    //             TypeLimit::All => {
    //                 for def in reader.types[&limit.namespace]
    //                     .values()
    //                     .map(|row| winmd::TypeDef { reader, row: *row })
    //                 {
    //                     match def.category() {
    //                         winmd::TypeCategory::Attribute | winmd::TypeCategory::Contract => {}
    //                         _ => tree.insert2(reader, &mut set, &def),
    //                     };
    //                 }
    //             }
    //             TypeLimit::Some(types) => {
    //                 let namespace = &reader.types[&limit.namespace];
    //                 // TODO: if 'name' isn't a type but a constant/function then look it up
    //                 // inside the Apis class for that namespace and add it to a `Class32`?

    //                 //let mut apis = None;

    //                 for name in types {
    //                     if let Some(row) = namespace.get(name) {
    //                         tree.insert2(reader, &mut set, &winmd::TypeDef { reader, row: *row });
    //                     }
    //                     //  else {
    //                     //     if apis.is_none() {
    //                     //         apis = Some(winmd::TypeDef{ reader, row: namespace["Apis"] });
    //                     //     }

    //                     //     if let Some(apis) = apis {

    //                     //     } else {
    //                     //         panic!("Type name not found");
    //                     //     }
    //                     // }
    //                 }
    //             }
    //         }
    //     }

    //     tree
    // }

    fn insert_if(
        &mut self,
        set: &mut std::collections::BTreeSet<winmd::CacheRow>,
        row: &winmd::CacheRow,
    ) {
        if set.insert(*row) {
            let t = TypeDefinition::from_cache_row(row);

            for def in t.dependencies() {
                self.insert_if(set, &winmd::CacheRow::TypeDef(def));
            }

            self.insert(t.name().namespace, t);
        }
    }

    /// Insert a [`TypeDefinition`] into [`TypeTree`]
    ///
    /// This recursively searchs the tree for an entry corresponding to the namespace
    pub fn insert(&mut self, namespace: &'static str, t: TypeDefinition) {
        if let Some(pos) = namespace.find('.') {
            self.namespaces
                .0
                .entry(&namespace[..pos])
                .or_default()
                .insert(&namespace[pos + 1..], t);
        } else {
            self.namespaces
                .0
                .entry(namespace)
                .or_default()
                .types
                .push(t);
        }
    }

    pub fn remove(&mut self, namespace: &str) {
        if let Some(pos) = namespace.find('.') {
            if let Some(tree) = self.namespaces.0.get_mut(&namespace[..pos]) {
                tree.remove(&namespace[pos + 1..])
            }
        } else {
            self.namespaces.0.remove(namespace);
        }
    }

    pub fn reexport(&mut self) {
        self.namespaces
            .0
            .entry("Windows")
            .or_default()
            .include_foundation = true;
    }

    /// Turn the tree into a token stream for code generation
    pub fn gen<'a>(&'a self) -> impl ParallelIterator<Item = TokenStream> + 'a {
        self.types
            .par_iter()
            .map(|t| t.gen())
            .chain(self.namespaces.gen())
    }
}

#[cfg(test)]
mod tests {
    use crate::{NamespaceTypes, TypeLimit, TypeLimits, TypeTree};

    #[test]
    fn test_dependency_inclusion() {
        let reader = winmd::TypeReader::from_build();
        let mut limits = TypeLimits::new(reader);
        limits
            .insert(NamespaceTypes {
                namespace: "windows.foundation".to_owned(),
                limit: TypeLimit::All,
            })
            .unwrap();
        limits
            .insert(NamespaceTypes {
                namespace: "windows.ui".to_owned(),
                limit: TypeLimit::All,
            })
            .unwrap();

        // Since Windows.Foundation depends on Windows.Foundation.Collections and
        // Windows.UI doesn't have dependencies, we should only see those namespaces.
        let root = TypeTree::from_limits(reader, &limits);

        // There is one root namespace.
        assert!(root.namespaces.0.len() == 1);
        let windows = &root.namespaces.0["Windows"];

        // The Windows namespace will only contain Foundation and UI.
        assert!(windows.namespaces.0.len() == 2);
        let foundation = &windows.namespaces.0["Foundation"];
        let ui = &windows.namespaces.0["UI"];

        // The UI namespace will not contain any further namespaces.
        assert!(ui.namespaces.0.is_empty());

        // The Foundation namespace will contain the Collections namespace.
        assert!(foundation.namespaces.0.len() == 1);
        let collections = &foundation.namespaces.0["Collections"];

        // The Collections namespace will not contain any further namespaces.
        assert!(collections.namespaces.0.is_empty());

        // The root never has any types.
        assert!(root.types.is_empty());

        // The Windows namespace has no types.
        assert!(windows.types.is_empty());

        // The UI namespace has all of its types.
        assert!(ui.types.iter().any(|t| t.name().name == "Colors"));
        assert!(ui.types.iter().any(|t| t.name().name == "IColorsStatics"));

        // The Foundation namespace has all of its types.
        assert!(foundation.types.iter().any(|t| t.name().name == "Uri"));
        assert!(foundation
            .types
            .iter()
            .any(|t| t.name().name == "IStringable"));

        // The Collections namespace only has the needed types.
        assert!(collections
            .types
            .iter()
            .any(|t| t.name().name == "IVectorView`1"));
        assert!(
            collections
                .types
                .iter()
                .any(|t| t.name().name == "PropertySet")
                == false
        );
    }
}
