use std::collections::BTreeSet;

/// The set of relevant namespaces and types
pub struct TypeLimits {
    pub(crate) reader: &'static winmd::TypeReader,
    pub(crate) namespaces: BTreeSet<NamespaceTypes>,
}

impl TypeLimits {
    pub fn new() -> Self {
        Self {
            reader: winmd::TypeReader::get(),
            namespaces: BTreeSet::new(),
        }
    }

    /// Insert a namespace into the set of relevant namespaces
    ///
    /// expects the namespace in the form: `parent::namespace::*`s
    pub fn insert(&mut self, mut limit: NamespaceTypes) -> Result<(), &'static str> {
        if let Some(namespace) = self.reader.find_lowercase_namespace(&limit.namespace.to_lowercase()) {
            limit.namespace = namespace;
            self.namespaces.insert(limit);
            Ok(())
        } else {
            Err(limit.namespace)
        }
    }

    pub fn namespaces(&self) -> impl Iterator<Item = &NamespaceTypes> {
        self.namespaces.iter()
    }
}

/// A namespace's relevant types
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct NamespaceTypes {
    pub namespace: &'static str, // &'static str since it should come from static TypeReader
    pub limit: TypeLimit,
}

/// A limit on the types in a namespace.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum TypeLimit {
    /// All the types in a namespace
    All,
    /// Some types in the namespace
    Some(Vec<String>), // &'static str since it should come from static TypeReader
}
