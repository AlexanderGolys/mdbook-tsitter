use dashmap::DashMap;
use lsp_types::{Location, Range, Url};
use std::{path::PathBuf, sync::{Arc, RwLock}};

#[derive(Debug, Clone)]
struct DefSite {
    uri: Url,
    range: Range,
    kind: TokenKind,
}

trait DefLookup {
    fn find(&self, name: &str, skip: &Url)
        -> Vec<Location>;
    fn has(&self, name: &str) -> bool;
    fn kind(&self, name: &str, skip: &Url)
        -> Option<TokenKind>;
}

impl<T: DefLookup + ?Sized> DefLookup for Arc<T> {
    fn find(
        &self,
        name: &str,
        skip: &Url,
    ) -> Vec<Location> {
        self.as_ref().find(name, skip)
    }

    fn has(&self, name: &str) -> bool {
        self.as_ref().has(name)
    }

    fn kind(
        &self,
        name: &str,
        skip: &Url,
    ) -> Option<TokenKind> {
        self.as_ref().kind(name, skip)
    }
}

/// A symbol index kept in sync with editor changes.
#[derive(Debug, Default)]
struct SymbolIndex {
    defs: DashMap<Symbol, Vec<DefSite>>,
    files: DashMap<Url, Vec<Symbol>>,
    roots: RwLock<Vec<PathBuf>>,
}

impl SymbolIndex {
    fn set_roots(&self, roots: Vec<PathBuf>) {
        *self.roots.write().expect("roots lock") = roots;
    }

    fn roots(&self) -> Vec<PathBuf> {
        self.roots.read().expect("roots lock").clone()
    }

    fn index<K>(&self, uri: &Url, text: &str, p: &K)
    where
        K: TypeProvider + ?Sized,
        for<'a> K::View<'a>: TokenKnowledge,
    {
        self.remove(uri);
        let defs = top_level_defs(text, p);
        if defs.is_empty() {
            return;
        }

        let mut names = Vec::with_capacity(defs.len());
        for (name, range, kind) in defs {
            let name = Symbol::new(&name);
            self.defs
                .entry(name.clone())
                .or_default()
                .push(DefSite {
                    uri: uri.clone(),
                    range,
                    kind,
                });
            names.push(name);
        }
        self.files.insert(uri.clone(), names);
    }

    fn remove(&self, uri: &Url) {
        let Some((_, names)) = self.files.remove(uri)
        else {
            return;
        };

        for name in names {
            let empty = self.defs
                .get_mut(&name)
                .is_some_and(|mut sites| {
                    sites.retain(|site| &site.uri != uri);
                    sites.is_empty()
                });
            if empty {
                self.defs.remove(&name);
            }
        }
    }
}

impl DefLookup for SymbolIndex {
    fn find(
        &self,
        name: &str,
        skip: &Url,
    ) -> Vec<Location> {
        self.defs.get(name)
            .into_iter()
            .flat_map(|sites| sites.iter())
            .filter(|site| &site.uri != skip)
            .map(|site| Location {
                uri: site.uri.clone(),
                range: site.range,
            })
            .collect()
    }

    fn has(&self, name: &str) -> bool {
        self.defs.contains_key(name)
    }

    fn kind(
        &self,
        name: &str,
        skip: &Url,
    ) -> Option<TokenKind> {
        self.defs.get(name)?
            .iter()
            .find(|site| &site.uri != skip)
            .map(|site| site.kind)
    }
}
