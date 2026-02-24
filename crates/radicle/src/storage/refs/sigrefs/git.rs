//! The transparency log of Radicle signed references is encoded in the Git
//! commit graph. This module provides traits for interacting with a Git
//! repository to read and write data for the transparency log process.

pub mod object;
pub mod reference;

pub use git2_impls::committer;

use crate::profile::env;
use crypto::PublicKey;
use radicle_git_metadata::author;
use radicle_git_metadata::author::Author;

/// Convenience type that corresponds to an [`Author`].
///
/// If [`env::GIT_COMMITTER_DATE`] is set, then [`Committer::from_env`] can be
/// used to construct a stable [`Author`].
///
/// Otherwise, an [`Author`] can be provided via [`Committer::new`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Committer {
    pub author: Author,
}

impl Committer {
    /// Construct a [`Committer`] using [`Committer::from_env`], if possible,
    /// using `default` if not.
    pub fn from_env_or_else<F>(public_key: &PublicKey, default: F) -> Self
    where
        F: FnOnce() -> Author,
    {
        Self::from_env(public_key).unwrap_or_else(|| Self::new(default()))
    }

    /// Construct a [`Committer`] with the provided [`Author`].
    pub fn new(author: Author) -> Self {
        Self { author }
    }

    /// Construct a [`Committer`] using the timestamp found at
    /// [`env::GIT_COMMITTER_DATE`], and the given [`PublicKey`] for the email.
    pub fn from_env(public_key: &PublicKey) -> Option<Self> {
        let s = env::var(env::GIT_COMMITTER_DATE).ok()?;
        let Ok(timestamp) = s.trim().parse::<i64>() else {
            panic!(
                "Invalid timestamp value {s:?} for `{}`",
                env::GIT_COMMITTER_DATE
            );
        };
        let time = author::Time::new(timestamp, 0);
        let author = Author {
            name: "radicle".to_string(),
            email: public_key.to_human(),
            time,
        };
        Some(Self::new(author))
    }

    pub fn into_inner(self) -> Author {
        self.author
    }
}

mod git2_impls {
    //! [`git2::Repository`] implementations of the [`object`] and [`reference`] traits.
    //!
    //! [`object`]: super::object
    //! [`reference`]: super::reference

    use std::path::Path;

    use radicle_core::NodeId;
    use radicle_git_metadata::author::{Author, Time};
    use radicle_oid::Oid;

    use crate::git;

    use super::object;
    use super::object::{RefsEntry, SignatureEntry};
    use super::reference;
    use super::Committer;

    pub fn committer(node: &NodeId, signature: &git2::Signature) -> Result<Committer, git2::Error> {
        let default = {
            let name = signature
                .name()
                .map(|name| name.to_string())
                .ok_or(git2::Error::new(
                    git2::ErrorCode::Invalid,
                    git2::ErrorClass::Invalid,
                    "Invalid UTF-8 of Git signature name",
                ))?;
            let email =
                signature
                    .email()
                    .map(|email| email.to_string())
                    .ok_or(git2::Error::new(
                        git2::ErrorCode::Invalid,
                        git2::ErrorClass::Invalid,
                        "Invalid UTF-8 of Git signature email",
                    ))?;
            Author {
                name,
                email,
                time: Time::new(
                    signature.when().seconds(),
                    signature.when().offset_minutes(),
                ),
            }
        };
        Ok(Committer::from_env_or_else(node, || default))
    }

    impl object::Reader for git2::Repository {
        fn read_commit(&self, oid: &Oid) -> Result<Option<Vec<u8>>, object::error::ReadCommit> {
            use object::error::ReadCommit;

            let odb = self.odb().map_err(ReadCommit::other)?;
            let object = odb.read(git2::Oid::from(*oid));
            match object {
                Ok(object) => {
                    if object.kind() != git2::ObjectType::Commit {
                        return Err(ReadCommit::incorrect_object_error(*oid, object.kind()));
                    }
                    Ok(Some(object.data().to_vec()))
                }
                Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
                Err(e) => Err(ReadCommit::other(e)),
            }
        }

        fn read_blob(
            &self,
            oid: &Oid,
            path: &Path,
        ) -> Result<Option<object::Blob>, object::error::ReadBlob> {
            use object::error::ReadBlob;

            let commit = match self.find_commit(git2::Oid::from(*oid)) {
                Ok(c) => c,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {
                    return Err(ReadBlob::commit_not_found_error(*oid))
                }
                Err(e) => return Err(ReadBlob::other(e)),
            };

            let tree = commit.tree().map_err(ReadBlob::other)?;

            let entry = match tree.get_path(path) {
                Ok(e) => e,
                Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
                Err(e) => return Err(ReadBlob::other(e)),
            };

            let object = entry.to_object(self).map_err(ReadBlob::other)?;
            let blob = object.as_blob().ok_or(ReadBlob::incorrect_object_error(
                *oid,
                path.to_path_buf(),
                object.kind().unwrap_or(git2::ObjectType::Any),
            ))?;

            Ok(Some(object::Blob {
                oid: blob.id().into(),
                bytes: blob.content().to_vec(),
            }))
        }
    }

    impl object::Writer for git2::Repository {
        fn write_tree(
            &self,
            refs: RefsEntry,
            signature: SignatureEntry,
        ) -> Result<Oid, object::error::WriteTree> {
            use object::error::WriteTree;

            let odb = self.odb().map_err(WriteTree::write_error)?;

            let refs_oid = odb
                .write(git2::ObjectType::Blob, &refs.content)
                .map_err(WriteTree::refs_error)?;

            let sig_oid = odb
                .write(git2::ObjectType::Blob, &signature.content)
                .map_err(WriteTree::signature_error)?;

            let mut builder = self.treebuilder(None).map_err(WriteTree::write_error)?;

            builder
                .insert(&refs.path, refs_oid, git2::FileMode::Blob.into())
                .map_err(WriteTree::refs_error)?;

            builder
                .insert(&signature.path, sig_oid, git2::FileMode::Blob.into())
                .map_err(WriteTree::signature_error)?;

            let tree_oid = builder.write().map_err(WriteTree::write_error)?;

            Ok(Oid::from(tree_oid))
        }

        fn write_commit(&self, bytes: &[u8]) -> Result<Oid, object::error::WriteCommit> {
            use object::error::WriteCommit;

            let odb = self.odb().map_err(WriteCommit::other)?;

            let oid = odb
                .write(git2::ObjectType::Commit, bytes)
                .map_err(WriteCommit::other)?;

            Ok(Oid::from(oid))
        }
    }

    impl reference::Reader for git2::Repository {
        fn find_reference(
            &self,
            reference: &git::fmt::Namespaced,
        ) -> Result<Option<Oid>, reference::error::FindReference> {
            match self.refname_to_id(reference.as_str()) {
                Ok(oid) => Ok(Some(Oid::from(oid))),
                Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
                Err(e) => Err(reference::error::FindReference::other(e)),
            }
        }
    }

    impl reference::Writer for git2::Repository {
        fn write_reference(
            &self,
            reference: &git::fmt::Namespaced,
            commit: Oid,
            parent: Option<Oid>,
            reflog: String,
        ) -> Result<(), reference::error::WriteReference> {
            let new = git2::Oid::from(commit);

            match parent {
                Some(parent) => {
                    let old = git2::Oid::from(parent);
                    // The old OID provides a guard, which gives us a compare-and-swap —
                    // the write will fail if the ref has moved since we read it.
                    self.reference_matching(reference.as_str(), new, true, old, &reflog)
                        .map_err(reference::error::WriteReference::other)?;
                }
                None => {
                    self.reference(reference.as_str(), new, false, &reflog)
                        .map_err(reference::error::WriteReference::other)?;
                }
            }

            Ok(())
        }
    }
}
