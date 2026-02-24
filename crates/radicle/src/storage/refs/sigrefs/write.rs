pub mod error;

#[cfg(test)]
mod test;

use std::marker::PhantomData;
use std::path::Path;

use crypto::signature::Signer;
use crypto::PublicKey;
use radicle_core::NodeId;
use radicle_git_metadata::author::Author;
use radicle_git_metadata::commit::{headers::Headers, trailers::OwnedTrailer, CommitData};
use radicle_oid::Oid;

use crate::git;
use crate::storage::refs::sigrefs::git::{object, reference, Committer};
use crate::storage::refs::{
    Refs, IDENTITY_ROOT, REFS_BLOB_PATH, SIGNATURE_BLOB_PATH, SIGREFS_BRANCH, SIGREFS_PARENT,
};
use crate::storage::refs::{SignedRefs, SignedRefsAt};

/// The result of calling [`SignedRefsWriter::write`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Update {
    /// The new signed references commit was written to the Git repository.
    Changed { entry: Box<Commit> },
    /// The provided [`Refs`] were equal to the current [`Refs`], so the process
    /// exited early.
    Unchanged {
        commit: Oid,
        refs: Refs,
        signature: crypto::Signature,
    },
}

impl From<Commit> for Update {
    fn from(commit: Commit) -> Self {
        Self::Changed {
            entry: Box::new(commit),
        }
    }
}

impl From<Head> for Update {
    fn from(
        Head {
            commit,
            refs,
            signature,
        }: Head,
    ) -> Self {
        Self::Unchanged {
            commit,
            refs,
            signature,
        }
    }
}

/// A [`SignedRefsWriter`] write a commit to the `rad/sigrefs` reference of a
/// namespace.
///
/// To create a new reader, use [`SignedRefsWriter::new`].
///
/// The construction expects:
/// - A [`Refs`] to write to the commit.
/// - A [`NodeId`] which identifies the namespace for which `rad/sigrefs`
///   reference should be read and written to.
/// - A `repository` which is the Git repository being used for reading and
///   writing.
/// - A `signer` which is the entity that produces the cryptographic signature
///   over the [`Refs`].
pub struct SignedRefsWriter<'a, R, S> {
    refs: Refs,
    namespace: NodeId,
    repository: &'a R,
    signer: &'a S,
}

impl<'a, R, S> SignedRefsWriter<'a, R, S>
where
    R: object::Writer + object::Reader + reference::Writer + reference::Reader,
    S: Signer<crypto::Signature>,
{
    /// Construct a new [`SignedRefsWriter`].
    ///
    /// The construction removes the ref [`SIGREFS_BRANCH`] from [`Refs`]
    /// (if present).
    ///
    /// When calling [`SignedRefsWriter::write`], if the process is successful,
    /// the given [`Refs`] will be written to the provided `namespace`.
    pub fn new(mut refs: Refs, namespace: NodeId, repository: &'a R, signer: &'a S) -> Self {
        debug_assert!(refs.get(&IDENTITY_ROOT).is_some());
        debug_assert!(refs.get(&SIGREFS_PARENT).is_none());
        refs.remove_sigrefs();
        Self {
            refs,
            namespace,
            repository,
            signer,
        }
    }

    /// Write a commit using the [`SignedRefsWriter`].
    ///
    /// The commit written will be composed of:
    /// - The parent commit of the previous entry, unless it is the root commit.
    /// - The [`Refs`] under the `/refs` blob. The [`Refs`] must include:
    ///   - The [`SIGREFS_PARENT`] entry.
    ///   - The [`IDENTITY_ROOT`] entry.
    /// - The [`crypto::Signature`] of the [`Refs`] bytes, under the
    ///   `/signature` blob.
    ///
    /// Note that the [`SIGREFS_PARENT`] is not never included in the [`Refs`]
    /// outside of this process.
    ///
    /// This commit is then written to the reference:
    /// ```text,no_run
    /// refs/namespaces/<namespace>/refs/rad/sigrefs
    /// ```
    pub fn write(
        self,
        committer: Committer,
        message: String,
        reflog: String,
    ) -> Result<Update, error::Write> {
        let author = committer.into_inner();
        let Self {
            refs,
            namespace,
            repository,
            signer,
        } = self;
        let reference = SIGREFS_BRANCH.with_namespace(git::fmt::Component::from(&namespace));
        let head = HeadReader::new(&reference, repository)
            .read()
            .map_err(error::Write::Head)?;
        let commit_writer = match head {
            Some(head) if head.is_unchanged(&refs) => return Ok(Update::from(head)),
            Some(head) => {
                CommitWriter::with_parent(refs, head.commit, author, message, repository, signer)
            }
            None => CommitWriter::root(refs, author, message, repository, signer),
        };
        let commit = commit_writer.write().map_err(error::Write::Commit)?;
        repository
            .write_reference(&reference, commit.oid, commit.parent, reflog)
            .map_err(error::Write::Reference)?;
        Ok(Update::from(commit))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The [`Oid`] of the parent commit.
    parent: Option<Oid>,
    /// The [`Oid`] of this commit.
    oid: Oid,
    /// The [`Refs`] that were committed.
    refs: Refs,
    /// The [`Signature`] of the [`Refs`] and the [`CommitData`].
    signature: crypto::Signature,
}

impl Commit {
    #[cfg(test)]
    pub(super) fn into_refs(self) -> Refs {
        self.refs
    }

    pub(crate) fn into_sigrefs_at(self, id: PublicKey) -> SignedRefsAt {
        SignedRefsAt {
            at: self.oid,
            sigrefs: SignedRefs {
                id,
                signature: self.signature,
                refs: self.refs,
                _verified: PhantomData,
            },
        }
    }
}

struct CommitWriter<'a, R, S> {
    refs: Refs,
    parent: Option<Oid>,
    author: Author,
    message: String,
    repository: &'a R,
    signer: &'a S,
}

impl<'a, R, S> CommitWriter<'a, R, S>
where
    R: object::Writer,
    S: Signer<crypto::Signature>,
{
    fn root(refs: Refs, author: Author, message: String, repository: &'a R, signer: &'a S) -> Self {
        Self {
            refs,
            parent: None,
            author,
            message,
            repository,
            signer,
        }
    }

    fn with_parent(
        refs: Refs,
        parent: Oid,
        author: Author,
        message: String,
        repository: &'a R,
        signer: &'a S,
    ) -> Self {
        Self {
            refs,
            parent: Some(parent),
            author,
            message,
            repository,
            signer,
        }
    }

    fn write(mut self) -> Result<Commit, error::Commit> {
        if let Some(parent) = self.parent {
            let prev = self.refs.add_parent(parent);
            debug_assert!(prev.is_none());
        }

        let mut tree = TreeWriter::new(self.refs, self.repository, self.signer)
            .write()
            .map_err(error::Commit::Tree)?;

        let commit = CommitData::new::<_, _, OwnedTrailer>(
            tree.oid,
            self.parent,
            self.author.clone(),
            self.author,
            Headers::new(),
            self.message,
            vec![],
        );

        let oid = self
            .repository
            .write_commit(commit.to_string().as_bytes())
            .map_err(error::Commit::Write)?;

        tree.refs.remove_parent();

        Ok(Commit {
            parent: self.parent,
            oid,
            refs: tree.refs,
            signature: tree.signature,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Tree {
    oid: Oid,
    refs: Refs,
    signature: crypto::Signature,
}

struct TreeWriter<'a, R, S> {
    refs: Refs,
    repository: &'a R,
    signer: &'a S,
}

impl<'a, R, S> TreeWriter<'a, R, S>
where
    R: object::Writer,
    S: Signer<crypto::Signature>,
{
    fn new(refs: Refs, repository: &'a R, signer: &'a S) -> Self {
        Self {
            refs,
            repository,
            signer,
        }
    }

    fn write(self) -> Result<Tree, error::Tree> {
        let canonical = self.refs.canonical();
        let signature = self
            .signer
            .try_sign(&canonical)
            .map_err(error::Tree::Sign)?;
        let refs = object::RefsEntry {
            path: Path::new(REFS_BLOB_PATH).to_path_buf(),
            content: canonical,
        };
        let sig = object::SignatureEntry {
            path: Path::new(SIGNATURE_BLOB_PATH).to_path_buf(),
            content: signature.to_vec(),
        };
        let oid = self
            .repository
            .write_tree(refs, sig)
            .map_err(error::Tree::Write)?;
        Ok(Tree {
            oid,
            refs: self.refs,
            signature,
        })
    }
}

/// The current head commit of the reference that points to the signed
/// references payload.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Head {
    /// The commit [`Oid`] at the head of the reference.
    commit: Oid,
    /// The [`Refs`] found within the head commit.
    refs: Refs,
    /// The [`crypto::Signature`] over the [`Refs`] blob.
    signature: crypto::Signature,
}

impl Head {
    /// Returns `true` if the `proposed` [`Refs`] are equal to the [`Refs`]
    /// of the [`Head`].
    fn is_unchanged(&self, proposed: &Refs) -> bool {
        self.refs == *proposed
    }
}

struct HeadReader<'a, 'b, R> {
    reference: &'a git::fmt::Namespaced<'a>,
    repository: &'b R,
}

impl<'a, 'b, R> HeadReader<'a, 'b, R>
where
    R: object::Reader + reference::Reader,
{
    /// Construct a [`HeadReader`] with the `reference` that is being read from
    /// the `repository.`
    fn new(reference: &'a git::fmt::Namespaced<'a>, repository: &'b R) -> Self {
        Self {
            reference,
            repository,
        }
    }

    /// Read the [`Head`] that is found in the repository under the given
    /// reference.
    ///
    /// Returns `None` if no such reference exists.
    ///
    /// The returned [`Refs`] do not contain the [`SIGREFS_PARENT`] reference.
    fn read(self) -> Result<Option<Head>, error::Head> {
        self.repository
            .find_reference(self.reference)
            .map_err(error::Head::Reference)?
            .map(|commit| self.with_refs(commit))
            .transpose()
    }

    fn with_refs(&self, commit: Oid) -> Result<Head, error::Head> {
        let refs = self.refs(commit)?;
        let signature = self.refs_signature(commit)?;
        Ok(Head {
            commit,
            refs,
            signature,
        })
    }

    fn refs(&self, commit: Oid) -> Result<Refs, error::Head> {
        let path = Path::new(REFS_BLOB_PATH);
        let object::Blob { bytes, .. } = self
            .repository
            .read_blob(&commit, path)
            .map_err(error::Head::Blob)?
            .ok_or(error::Head::MissingPath {
                commit,
                path: path.to_path_buf(),
            })?;

        let mut refs = Refs::from_canonical(&bytes).map_err(error::Head::Refs)?;
        refs.remove_parent();
        Ok(refs)
    }

    fn refs_signature(&self, commit: Oid) -> Result<crypto::Signature, error::Head> {
        let path = Path::new(SIGNATURE_BLOB_PATH);
        let object::Blob {
            bytes: sig_bytes, ..
        } = self
            .repository
            .read_blob(&commit, path)
            .map_err(error::Head::Blob)?
            .ok_or(error::Head::MissingPath {
                commit,
                path: path.to_path_buf(),
            })?;
        crypto::Signature::try_from(sig_bytes.as_slice()).map_err(|err| error::Head::Signature {
            commit,
            source: err,
        })
    }
}
