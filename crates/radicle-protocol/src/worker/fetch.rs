pub mod error;

use std::collections::HashSet;

use radicle::crypto::PublicKey;
use radicle::{identity::DocAt, storage::RefUpdate};

#[derive(Debug, Clone)]
pub struct FetchResult {
    /// The set of updated references.
    pub updated: Vec<RefUpdate>,
    /// The set of remote namespaces that were updated.
    pub namespaces: HashSet<PublicKey>,
    /// The fetch was a full clone.
    pub clone: bool,
    /// Identity doc of fetched repo.
    pub doc: DocAt,
}

impl FetchResult {
    pub fn new(doc: DocAt) -> Self {
        Self {
            updated: vec![],
            namespaces: HashSet::new(),
            clone: false,
            doc,
        }
    }
}

#[cfg(any(test, feature = "test"))]
impl qcheck::Arbitrary for FetchResult {
    fn arbitrary(g: &mut qcheck::Gen) -> Self {
        FetchResult {
            updated: vec![],
            namespaces: HashSet::arbitrary(g),
            clone: bool::arbitrary(g),
            doc: DocAt::arbitrary(g),
        }
    }
}
