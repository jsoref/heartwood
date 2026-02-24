use radicle_git_ref_format::Component;

use super::mock::{self, MockRepository};
use crate::storage::refs::sigrefs::write::{error, Head, HeadReader};
use crate::storage::refs::{Refs, SIGREFS_BRANCH};

/// Drive `HeadReader` directly via the sigrefs reference for `mock::node_id()`.
fn read(repo: &MockRepository) -> Result<Option<Head>, error::Head> {
    let namespace = mock::node_id();
    let reference = SIGREFS_BRANCH.with_namespace(Component::from(&namespace));
    HeadReader::new(&reference, repo).read()
}

#[test]
fn reference_error() {
    let repo = MockRepository::new().with_rad_sigrefs_error(&mock::node_id());
    assert!(matches!(read(&repo), Err(error::Head::Reference(_))));
}

#[test]
fn no_head() {
    let repo = MockRepository::new().with_missing_rad_sigrefs(&mock::node_id());
    assert!(matches!(read(&repo), Ok(None)));
}

#[test]
fn refs_blob_error() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_refs_error(head);
    assert!(matches!(read(&repo), Err(error::Head::Blob(_))));
}

#[test]
fn refs_blob_missing() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_missing_refs(head)
        .with_signature(head, 1);
    assert!(matches!(read(&repo), Err(error::Head::MissingPath { .. })));
}

#[test]
fn refs_parse_error() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_invalid_refs(head)
        .with_signature(head, 1);
    assert!(matches!(read(&repo), Err(error::Head::Refs(_))));
}

#[test]
fn signature_blob_error() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_refs(head, [(mock::refs_heads_main(), mock::oid(10))])
        .with_signature_error(head);
    assert!(matches!(read(&repo), Err(error::Head::Blob(_))));
}

#[test]
fn signature_blob_missing() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_refs(head, [(mock::refs_heads_main(), mock::oid(10))])
        .with_missing_signature(head);
    assert!(matches!(read(&repo), Err(error::Head::MissingPath { .. })));
}

#[test]
fn signature_parse_error() {
    let head = mock::oid(1);
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_refs(head, [(mock::refs_heads_main(), mock::oid(10))])
        .with_invalid_signature(head);
    assert!(matches!(read(&repo), Err(error::Head::Signature { .. })));
}

#[test]
fn read_ok() {
    let head = mock::oid(1);
    let refs = [(mock::refs_heads_main(), mock::oid(10))];
    let repo = MockRepository::new()
        .with_rad_sigrefs(&mock::node_id(), head)
        .with_refs(head, refs.clone())
        .with_signature(head, 1);
    assert_eq!(
        read(&repo).unwrap(),
        Some(Head {
            commit: head,
            refs: Refs::from(refs.into_iter()),
            signature: crypto::Signature::from([1; 64]),
        })
    );
}
