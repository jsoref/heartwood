use std::collections::{BTreeMap, BTreeSet};
use std::ops::Not as _;

use radicle::storage::git::Repository;
pub use radicle::storage::refs::SignedRefsAt;
pub use radicle::storage::{git::Validation, Validations};
use radicle::{crypto::PublicKey, storage::ValidateRepository};

use crate::state::Cached;

pub mod error {
    use radicle::crypto::PublicKey;
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[non_exhaustive]
    pub enum RemoteRefs {
        #[error("required sigrefs of {0} not found")]
        NotFound(PublicKey),
        #[error(transparent)]
        Load(#[from] Load),
    }

    pub type Load = radicle::storage::refs::sigrefs::read::error::Read;
}

pub(crate) fn validate(
    repo: &impl ValidateRepository,
    SignedRefsAt { sigrefs, .. }: SignedRefsAt,
) -> Result<Option<Validations>, radicle::storage::Error> {
    let remote = radicle::storage::Remote::new(sigrefs);
    let validations = repo.validate_remote(&remote)?;
    Ok(validations.is_empty().not().then_some(validations))
}

/// The sigrefs found for each remote.
///
/// Construct using [`RemoteRefs::load`].
#[derive(Debug, Default)]
pub struct RemoteRefs(
    pub(super)  BTreeMap<
        PublicKey,
        Result<Option<SignedRefsAt>, radicle::storage::refs::sigrefs::read::error::Read>,
    >,
);

impl RemoteRefs {
    /// Load the sigrefs for each remote in `remotes`.
    pub(crate) fn load<'a, R, S>(
        cached: &Cached<R, S>,
        remotes: impl Iterator<Item = &'a PublicKey>,
    ) -> Self
    where
        R: AsRef<Repository>,
    {
        Self(
            remotes
                .map(|remote| (*remote, cached.load(remote)))
                .collect(),
        )
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn into_inner(
        self,
    ) -> BTreeMap<
        PublicKey,
        Result<Option<SignedRefsAt>, radicle::storage::refs::sigrefs::read::error::Read>,
    > {
        self.0
    }
}

impl<'a> IntoIterator for &'a RemoteRefs {
    type Item = <&'a BTreeMap<
        PublicKey,
        Result<Option<SignedRefsAt>, radicle::storage::refs::sigrefs::read::error::Read>,
    > as IntoIterator>::Item;
    type IntoIter = <&'a BTreeMap<
        PublicKey,
        Result<Option<SignedRefsAt>, radicle::storage::refs::sigrefs::read::error::Read>,
    > as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
