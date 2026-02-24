use std::path::PathBuf;

use radicle_oid::Oid;
use thiserror::Error;

use crate::storage::refs::canonical;
use crate::storage::refs::sigrefs::git::{object, reference};

// TODO: use commit NID (and RID?) for traceability
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Write {
    #[error(transparent)]
    Head(Head),
    #[error(transparent)]
    Commit(Commit),
    #[error(transparent)]
    Reference(reference::error::WriteReference),
}

// TODO: use commit OID for traceability
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Commit {
    #[error(transparent)]
    Tree(Tree),
    #[error(transparent)]
    Write(object::error::WriteCommit),
}

// TODO: use commit OID for traceability
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Tree {
    #[error("failed to sign references payload")]
    Sign(crypto::signature::Error),
    #[error(transparent)]
    Write(object::error::WriteTree),
}

// TODO: use commit OID for traceability
#[derive(Debug, Error)]
#[non_exhaustive]
#[error(transparent)]
pub enum Head {
    #[error(transparent)]
    Reference(reference::error::FindReference),
    #[error(transparent)]
    Blob(object::error::ReadBlob),
    #[error(transparent)]
    Refs(canonical::Error),
    #[error("failed to parse refs signature in commit {commit}")]
    Signature { commit: Oid, source: crypto::Error },
    #[error(
        "could not find the references blob, within the commit '{commit}', under the path {path:?}"
    )]
    MissingPath { commit: Oid, path: PathBuf },
}
