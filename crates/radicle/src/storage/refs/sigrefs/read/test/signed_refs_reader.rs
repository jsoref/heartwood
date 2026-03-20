use radicle_oid::Oid;

use crate::storage::refs::sigrefs::read::error::{Read, Verify};
use crate::storage::refs::sigrefs::read::{error, Commit, SignedRefsReader, Tip};
use crate::storage::refs::sigrefs::VerifiedCommit;
use crate::storage::refs::{IDENTITY_ROOT, SIGREFS_PARENT};
use crate::{assert_matches, git};

use super::mock;
use super::mock::{AlwaysVerify, MockRepository};

fn refs_without_parent(head_oid: Oid) -> Vec<(git::fmt::RefString, Oid)> {
    vec![
        (mock::refs_heads_main(), head_oid),
        (
            IDENTITY_ROOT.to_ref_string(),
            mock::oid(mock::MOCKED_IDENTITY),
        ),
    ]
}

fn refs(head_oid: Oid, parent_oid: Oid) -> Vec<(git::fmt::RefString, Oid)> {
    vec![
        (mock::refs_heads_main(), head_oid),
        (
            IDENTITY_ROOT.to_ref_string(),
            mock::oid(mock::MOCKED_IDENTITY),
        ),
        (SIGREFS_PARENT.to_ref_string(), parent_oid),
    ]
}

fn read(tip: Oid, repo: MockRepository) -> Result<VerifiedCommit, error::Read> {
    SignedRefsReader::new(mock::rid(99), Tip::Commit(tip), &repo, &AlwaysVerify).read()
}

#[test]
fn head_commit_error() {
    let head = mock::oid(1);
    let repo = MockRepository::new().with_commit_error(head);

    let err = read(head, repo).unwrap_err();
    assert!(matches!(err, error::Read::Commit(_)));
}

#[test]
fn walk_commit_error() {
    let root = mock::oid(1);
    let head = mock::oid(2);
    let r2 = refs_without_parent(head);

    let repo = MockRepository::new()
        .with_commit(head, mock::commit_data([root]))
        .with_refs(head, r2)
        .with_signature(head, 1)
        .with_commit_error(root);

    let err = read(head, repo).unwrap_err();
    assert!(matches!(err, error::Read::Commit(_)));
}

#[test]
fn head_verify_signature_error() {
    // The verifier always rejects the signature → `error::Verify::Signature`.
    let head = mock::oid(1);
    let repo = mock::setup_chain([(head, 1, refs_without_parent(head))]);

    let err = SignedRefsReader::new(mock::rid(99), Tip::Commit(head), &repo, &mock::NeverVerify)
        .read()
        .unwrap_err();

    assert!(matches!(
        err,
        error::Read::Verify(error::Verify::Signature(_))
    ));
}

#[test]
fn head_verify_mismatched_identity_error() {
    let head = mock::oid(1);
    // RepoId in test scenario is rid(99), so not equal to rid(50)
    let mismatched_identity_root = mock::oid(50);
    let refs = [
        (mock::refs_heads_main(), mock::oid(10)),
        (IDENTITY_ROOT.to_ref_string(), mismatched_identity_root),
    ];

    let repo = MockRepository::new()
        .with_commit(head, mock::commit_data([]))
        .with_refs(head, refs)
        .with_signature(head, 1)
        .with_identity(mismatched_identity_root);

    let err = read(head, repo).unwrap_err();
    assert!(matches!(
        err,
        error::Read::Verify(error::Verify::MismatchedIdentity { .. })
    ));
}

#[test]
fn walk_verify_error() {
    let root = mock::oid(1);
    let commit1 = mock::oid(2);
    let commit2 = mock::oid(3);
    let identity_root_mismatch = mock::oid(50);

    let r1 = [
        (mock::refs_heads_main(), mock::oid(10)),
        (
            IDENTITY_ROOT.to_ref_string(),
            mock::oid(mock::MOCKED_IDENTITY),
        ),
    ];
    let r2 = [
        (mock::refs_heads_main(), mock::oid(10)),
        (IDENTITY_ROOT.to_ref_string(), identity_root_mismatch),
    ];
    let r3 = [
        (mock::refs_heads_main(), mock::oid(10)),
        (
            IDENTITY_ROOT.to_ref_string(),
            mock::oid(mock::MOCKED_IDENTITY),
        ),
    ];

    let repo = MockRepository::new()
        .with_commit(root, mock::commit_data([]))
        .with_refs(root, r1)
        .with_signature(root, 1)
        .with_commit(commit1, mock::commit_data([root]))
        .with_refs(commit1, r2)
        .with_signature(commit1, 1)
        .with_identity(identity_root_mismatch)
        .with_commit(commit2, mock::commit_data([commit1]))
        .with_refs(commit2, r3)
        .with_signature(commit2, 1);

    let err = read(commit2, repo).unwrap_err();
    assert!(matches!(
        err,
        error::Read::Verify(error::Verify::MismatchedIdentity { .. })
    ));
}

#[test]
fn single_commit() {
    let head = mock::oid(1);
    let repo = mock::setup_chain([(head, 1, refs_without_parent(head))]);

    let vc = read(head, repo).unwrap();
    assert_eq!(*vc.id(), head);
}

#[test]
fn two_commits() {
    let root = mock::oid(1);
    let head = mock::oid(2);
    let repo = mock::setup_chain([
        (root, 1, refs_without_parent(root)),
        (head, 2, refs_without_parent(head)),
    ]);

    let vc = read(head, repo).unwrap();
    assert_eq!(*vc.id(), head);
}

/// We test a handful scenarios with replayed commits (or rather, references
/// and signatures within commits).
///
/// For every test we define:
///  - A history, which is a linear history of commits,
///    where the earliest and leftmost commit is a root commit.
///  - Which commit we expect to be loaded, as a zero based index in the
///    history.
mod replay {
    use super::*;

    /// Mocks a chain of commits, where their OID is their zero-based index
    /// in `chain` (note that since this is only mocked, it is not an issue
    /// that the first commit in the chain, at index zero, is identified by
    /// the zero OID).
    ///
    /// Asserts that the result of [`read`] on the chain is `expected`.
    fn replay(chain: impl IntoIterator<Item = u8>, expected: u8) {
        let refs = refs_without_parent(mock::oid(10));

        let chain: Vec<_> = chain.into_iter().collect();
        let mut repo = MockRepository::new();
        let mut parent = None;
        for (i, signature) in chain.iter().enumerate() {
            let i = mock::oid(i as u8);
            repo = repo
                .with_commit(i, mock::commit_data(parent))
                .with_refs(i, refs.clone())
                .with_signature(i, *signature);
            parent = Some(i);
        }

        assert_eq!(
            *read(mock::oid((chain.len() - 1) as u8), repo).unwrap().id(),
            mock::oid(expected)
        )
    }

    #[test]
    fn root_at_head() {
        replay([1, 2, 1], 1)
    }

    #[test]
    fn chain() {
        replay([1, 1, 1], 0)
    }

    #[test]
    fn multiple() {
        replay([1, 1, 2, 3, 3], 3)
    }

    #[test]
    fn alternating() {
        replay([1, 2, 1, 2], 1)
    }
}

#[test]
fn read_ok_no_parent() {
    const SIGNATURE_1: u8 = 1;
    const SIGNATURE_2: u8 = 2;

    let c1 = mock::oid(1);
    let c2 = mock::oid(2);

    let r = refs_without_parent(mock::oid(10));
    let repo = mock::setup_chain([(c2, SIGNATURE_2, r.clone()), (c1, SIGNATURE_1, r)]);

    let vc = read(c1, repo).unwrap();
    assert_eq!(*vc.id(), c1);

    assert_matches!(
        vc,
        VerifiedCommit {
            commit: Commit {
                oid: _,
                parent: Some(_),
                refs: _,
                signature: _,
                identity_root: Some(_)
            },
            parent: false
        }
    );
}

#[test]
fn read_ok_parent() {
    const SIGNATURE_1: u8 = 1;
    const SIGNATURE_2: u8 = 2;

    let c1 = mock::oid(1);
    let c2 = mock::oid(2);

    let repo = mock::setup_chain([
        (c2, SIGNATURE_2, refs_without_parent(mock::oid(10))),
        (c1, SIGNATURE_1, refs(mock::oid(20), c2)),
    ]);

    let vc = read(c1, repo).unwrap();
    assert_eq!(*vc.id(), c1);

    assert_matches!(vc, VerifiedCommit { commit: Commit {
        oid,
        parent: Some(parent),
        refs: _,
        signature: _,
        identity_root: Some(_)
    }, parent: true } if parent == c2 && oid == c1);
}

#[test]
fn invalid_parent() {
    const SIGNATURE_1: u8 = 1;
    const SIGNATURE_2: u8 = 2;

    let c1 = mock::oid(1);
    let c2 = mock::oid(2);

    let wrong = mock::oid(42);

    let repo = mock::setup_chain([
        (c2, SIGNATURE_2, refs_without_parent(mock::oid(10))),
        (c1, SIGNATURE_1, refs(mock::oid(20), wrong)),
    ]);

    assert_matches!(read(c1, repo), Err(Read::Verify(Verify::MismatchedParent {
        sigrefs_commit,
        expected,
        actual,
    })) if sigrefs_commit == c1 && expected == c2 && actual == wrong);
}
