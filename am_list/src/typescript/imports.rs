use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Relative source of an import
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source(String);

impl<T: Into<String>> From<T> for Source {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl ToString for Source {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Source {
    pub fn into_canonical(self, import_statement_location: Option<&Path>) -> CanonicalSource {
        if import_statement_location.map_or(true, |path| path.to_string_lossy().is_empty()) {
            // This base case is reached when we called `import_statement_location.parent()` too
            // many times, which means the import is a sibling of the import_statement_location given in the beginning.
            return CanonicalSource::from(format!("sibling://{}", self.0));
        }

        let import_location = import_statement_location.unwrap();

        let relative_path = PathBuf::from(self.0);
        if let Ok(sibling) = relative_path.strip_prefix("..") {
            return Source::from(sibling.to_string_lossy())
                .into_canonical(import_location.parent());
        }

        if let Ok(sub_dir) = relative_path.strip_prefix(".") {
            let mut combined_path = import_location.to_path_buf();
            combined_path.push(sub_dir);
            CanonicalSource::from(combined_path.as_os_str().to_string_lossy())
        } else {
            CanonicalSource::from(format!("ext://{}", relative_path.display()))
        }
    }
}

/// Canonical source of an import
///
/// The import will begin with `ext://` if the import is detected to come from
/// outside the current project.
///
/// The import will begin with `sibling://` if the import is detected to come
/// from a sibling folder in the same repository as the current project.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CanonicalSource(String);

impl<T: Into<String>> From<T> for CanonicalSource {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl ToString for CanonicalSource {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

/// New type for Identifiers to create type safe interfaces.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct Identifier(String);

impl<T: Into<String>> From<T> for Identifier {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

/// Structure containing the map of imports valid in a given source file.
#[derive(Clone, Debug, Default)]
pub struct ImportsMap {
    namespaced_imports: HashMap<Identifier, CanonicalSource>,
    /// Maps:
    /// - (real_name) to (real_name, source), and
    /// - (aliased_name) to (aliased_name, source)
    named_imports: HashMap<Identifier, (Identifier, CanonicalSource)>,
}

impl ImportsMap {
    pub fn find_namespace(&self, namespace: &Identifier) -> Option<CanonicalSource> {
        self.namespaced_imports.get(namespace).cloned()
    }

    pub fn find_identifier(&self, ident: &Identifier) -> Option<(Identifier, CanonicalSource)> {
        self.named_imports.get(ident).cloned()
    }

    pub fn add_namespace(
        &mut self,
        namespace: Identifier,
        source: CanonicalSource,
    ) -> Option<CanonicalSource> {
        self.namespaced_imports.insert(namespace, source)
    }

    pub fn add_named_import(
        &mut self,
        import: Identifier,
        source: CanonicalSource,
    ) -> Option<(Identifier, CanonicalSource)> {
        self.named_imports.insert(import.clone(), (import, source))
    }

    pub fn add_aliased_import(
        &mut self,
        alias: Identifier,
        name_in_source: Identifier,
        source: CanonicalSource,
    ) -> Option<(Identifier, CanonicalSource)> {
        self.named_imports.insert(alias, (name_in_source, source))
    }

    /// Return the original name and the source of the given identifier.
    pub fn resolve_ident(&self, ident: Identifier) -> Option<(Identifier, CanonicalSource)> {
        let ident_str = ident.to_string();

        if let Some((namespace, sub_ident)) = ident_str.split_once('.') {
            self.find_namespace(&Identifier::from(namespace))
                .map(|canon| (Identifier::from(sub_ident), canon))
        } else {
            self.find_identifier(&ident)
        }
    }
}
