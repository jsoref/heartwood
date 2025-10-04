//! This module re-exports selected items from the [`git2`] crate and provides
//! an extension trait for its [`git2::Error`] type to more conveniently handle
//! errors associated with the code [`git2::ErrorCode::NotFound`].
//!
// Re-exports created by manually scanning the `heartwood` workspace on 2025-10-04.

// Re-exports that are only used within this crate.
pub(crate) use git2::{
    message_trailers_strs, AutotagOption, Blob, Config, FetchOptions, FetchPrune, Object,
    RemoteCallbacks, Revwalk, Sort,
};

// Re-exports that are used by other crates in the workspace, including this crate.
pub use git2::{
    Branch, BranchType, Commit, Direction, Error, ErrorClass, ErrorCode, FileMode, ObjectType, Oid,
    Reference, Remote, Repository, RepositoryInitOptions, RepositoryOpenFlags, Signature, Time,
    Tree,
};

// Re-exports that are used by other crates in the workspace, but *not* this crate.
pub use git2::{
    AnnotatedCommit, Diff, DiffFindOptions, DiffOptions, DiffStats, MergeAnalysis, MergeOptions,
};

// Re-exports for `radicle-cli`.
pub mod build {
    pub use git2::build::CheckoutBuilder;
}

pub(crate) mod transport {
    pub use git2::transport::{
        register, Service, SmartSubtransport, SmartSubtransportStream, Transport,
    };
}
