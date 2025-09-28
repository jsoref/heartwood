use clap::Parser;
use thiserror::Error;

use radicle::prelude::{NodeId, RepoId};

pub(crate) const ABOUT: &str = "Block repositories or nodes from being seeded or followed";

#[derive(Clone, Debug)]
pub(super) enum Target {
    Node(NodeId),
    Repo(RepoId),
}

#[derive(Debug, Error)]
#[error("invalid repository or node specified (RID parsing failed with: '{repo}', NID parsing failed with: '{node}'))")]
pub(super) struct ParseTargetError {
    repo: radicle::identity::IdError,
    node: radicle::crypto::PublicKeyError,
}

impl std::str::FromStr for Target {
    type Err = ParseTargetError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        val.parse::<RepoId>().map(Target::Repo).or_else(|repo| {
            val.parse::<NodeId>()
                .map(Target::Node)
                .map_err(|node| ParseTargetError { repo, node })
        })
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Node(nid) => nid.fmt(f),
            Self::Repo(rid) => rid.fmt(f),
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = ABOUT, disable_version_flag = true)]
pub struct Args {
    /// A Repository ID or Node ID to block from seeding or following (respectively)
    ///
    /// Example values:
    /// - z6MkiswaKJ85vafhffCGBu2gdBsYoDAyHVBWRxL3j297fwS9 (Node ID)
    /// - rad:z3Tr6bC7ctEg2EHmLvknUr29mEDLH (Repository ID)
    #[arg(value_name = "RID|NID", verbatim_doc_comment)]
    pub(super) target: Target,
}

#[cfg(test)]
mod test {
    use clap::error::ErrorKind;
    use clap::Parser;

    use super::Args;

    #[test]
    fn should_parse_nid() {
        let args =
            Args::try_parse_from(["block", "z6MkiswaKJ85vafhffCGBu2gdBsYoDAyHVBWRxL3j297fwS9"]);
        assert!(args.is_ok())
    }

    #[test]
    fn should_parse_rid() {
        let args = Args::try_parse_from(["block", "rad:z3Tr6bC7ctEg2EHmLvknUr29mEDLH"]);
        assert!(args.is_ok())
    }

    #[test]
    fn should_not_parse() {
        let err = Args::try_parse_from(["block", "bee"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }
}
