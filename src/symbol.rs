use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};

/// A compact identifier for an interned string.
///
/// `SymbolId` is intentionally small, copyable, and cheap to compare.
/// It is only meaningful within the [`Interner`] instance that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(pub usize);

/// A string interner.
///
/// The interner assigns a unique [`SymbolId`] to each distinct string.
/// If the same string is interned multiple times, the same [`SymbolId`]
/// is returned.
///
/// This is useful for identifiers, keywords, and other repeated strings
/// where comparing or storing integers is cheaper than repeatedly working
/// with full string values.
#[derive(Debug)]
pub struct Interner {
    /// Maps interned text to its symbol.
    map: BTreeMap<String, SymbolId>,

    /// Stores interned text in insertion order so symbols can be resolved
    /// back into their original string form.
    strings: Vec<String>,
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    /// Creates an empty interner.
    #[inline]
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            strings: Vec::new(),
        }
    }

    /// Returns the symbol for `text`, interning it first if necessary.
    ///
    /// This operation is idempotent: repeated calls with equal text
    /// return the same [`SymbolId`].
    pub fn intern(&mut self, text: &str) -> SymbolId {
        if let Some(existing_id) = self.map.get(text) {
            return *existing_id;
        }

        self.insert_new(text)
    }

    /// Resolves a symbol back to its interned text.
    ///
    /// # Panics
    ///
    /// Panics if `id` was not produced by this interner.
    ///
    /// Prefer [`Interner::get`] when an invalid symbol is possible and
    /// should be handled without panicking.
    #[inline]
    #[track_caller]
    pub fn resolve(&self, id: SymbolId) -> &str {
        self.get(id)
            .expect("attempted to resolve an invalid SymbolId")
    }

    /// Resolves a symbol back to its interned text, if the symbol is valid.
    ///
    /// This is the non-panicking alternative to [`Interner::resolve`].
    #[inline]
    pub fn get(&self, id: SymbolId) -> Option<&str> {
        self.strings.get(id.0).map(String::as_str)
    }

    /// Inserts a string that is known not to be present yet.
    ///
    /// This keeps the `intern` method easier to read by moving the
    /// insertion details into a dedicated helper.
    fn insert_new(&mut self, text: &str) -> SymbolId {
        let id = SymbolId(self.strings.len());

        // The current implementation stores two owned copies of each new
        // string: one as the map key and one in the vector used for reverse
        // lookup. This keeps the public API simple and avoids unsafe code.
        let owned = text.to_owned();
        self.strings.push(owned.clone());
        self.map.insert(owned, id);

        id
    }
}
