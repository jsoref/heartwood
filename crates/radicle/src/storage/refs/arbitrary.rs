#![allow(clippy::unwrap_used)]

use qcheck::Arbitrary;

use crate::node::device::Device;

use super::*;

pub fn signed_refs_at<S>(g: &mut qcheck::Gen, root: Oid, signer: &Device<S>) -> SignedRefsAt
where
    S: crypto::signature::Signer<crypto::Signature>,
{
    let mut refs = Refs::arbitrary(g);
    refs.insert(IDENTITY_ROOT.to_ref_string(), root);
    let signature = crypto::signature::Signer::sign(signer, &refs.canonical());
    let sigrefs = SignedRefs {
        refs,
        signature,
        id: *signer.node_id(),
        _verified: PhantomData,
    };
    SignedRefsAt {
        sigrefs,
        at: Oid::from_sha1(Arbitrary::arbitrary(g)),
    }
}

impl Arbitrary for Refs {
    fn arbitrary(g: &mut qcheck::Gen) -> Self {
        let mut refs: BTreeMap<git::fmt::RefString, storage::Oid> = BTreeMap::new();
        let mut bytes: [u8; 20] = [0; 20];
        let names = &[
            "heads/master",
            "heads/feature/1",
            "heads/feature/2",
            "heads/feature/3",
            "rad/id",
            "tags/v1.0",
            "tags/v2.0",
            "notes/1",
        ];

        for _ in 0..g.size().min(names.len()) {
            if let Some(name) = g.choose(names) {
                for byte in &mut bytes {
                    *byte = u8::arbitrary(g);
                }
                let oid = storage::Oid::from_sha1(bytes);
                let name = git::fmt::RefString::try_from(*name).unwrap();

                refs.insert(name, oid);
            }
        }
        Self::from(refs)
    }
}

impl Arbitrary for RefsAt {
    fn arbitrary(g: &mut qcheck::Gen) -> Self {
        Self {
            remote: PublicKey::arbitrary(g),
            at: Oid::from_sha1(Arbitrary::arbitrary(g)),
        }
    }
}
