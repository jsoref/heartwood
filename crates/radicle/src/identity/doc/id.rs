use std::ops::Deref;
use std::{ffi::OsString, fmt, str::FromStr};

use crate::git::fmt::{Component, RefString};
use thiserror::Error;

use crate::git;
use crate::serde_ext;

/// Radicle identifier prefix.
pub const RAD_PREFIX: &str = "rad:";

#[derive(Error, Debug)]
pub enum IdError {
    #[error(transparent)]
    Multibase(#[from] multibase::Error),
    #[error("invalid length: expected {expected} bytes, got {actual} bytes")]
    Length { expected: usize, actual: usize },
}

/// A repository identifier.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RepoId(
    #[cfg_attr(feature = "schemars", schemars(
        with = "String",
        description = "A repository identifier. Starts with \"rad:\", followed by a multibase Base58 encoded Git object identifier.",
        regex(pattern = r"rad:z[1-9a-km-zA-HJ-NP-Z]+"),
        length(min = 5),
        example = &"rad:z3gqcJUoA1n9HaHKufZs5FCSGazv5",
    ))]
    git::Oid,
);

impl fmt::Display for RepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.urn().as_str())
    }
}

impl fmt::Debug for RepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RepoId({self})")
    }
}

impl RepoId {
    /// Format the identifier as a human-readable URN.
    ///
    /// Eg. `rad:z3XncAdkZjeK9mQS5Sdc4qhw98BUX`.
    ///
    pub fn urn(&self) -> String {
        format!("{RAD_PREFIX}{}", self.canonical())
    }

    /// Parse an identifier from the human-readable URN format.
    /// Accepts strings without the radicle prefix as well,
    /// for convenience.
    pub fn from_urn(s: &str) -> Result<Self, IdError> {
        let s = s.strip_prefix(RAD_PREFIX).unwrap_or(s);
        let id = Self::from_canonical(s)?;

        Ok(id)
    }

    /// Format the identifier as a multibase string.
    ///
    /// Eg. `z3XncAdkZjeK9mQS5Sdc4qhw98BUX`.
    ///
    pub fn canonical(&self) -> String {
        multibase::encode(multibase::Base::Base58Btc, AsRef::<[u8]>::as_ref(&self.0))
    }

    pub fn from_canonical(input: &str) -> Result<Self, IdError> {
        const EXPECTED_LEN: usize = 20;
        let (_, bytes) = multibase::decode(input)?;
        let bytes: [u8; EXPECTED_LEN] =
            bytes.try_into().map_err(|bytes: Vec<u8>| IdError::Length {
                expected: EXPECTED_LEN,
                actual: bytes.len(),
            })?;
        Ok(Self(crate::git::Oid::from_sha1(bytes)))
    }
}

impl FromStr for RepoId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_urn(s)
    }
}

impl TryFrom<OsString> for RepoId {
    type Error = IdError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        let string = value.to_string_lossy();
        Self::from_canonical(&string)
    }
}

impl From<git::Oid> for RepoId {
    fn from(oid: git::Oid) -> Self {
        Self(oid)
    }
}

impl From<git::raw::Oid> for RepoId {
    fn from(oid: git::raw::Oid) -> Self {
        Self(oid.into())
    }
}

impl Deref for RepoId {
    type Target = git::Oid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for RepoId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_ext::string::serialize(&self.urn(), serializer)
    }
}

impl<'de> serde::Deserialize<'de> for RepoId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_ext::string::deserialize(deserializer)
    }
}

impl From<&RepoId> for Component<'_> {
    fn from(id: &RepoId) -> Self {
        let refstr =
            RefString::try_from(id.0.to_string()).expect("repository id's are valid ref strings");
        Component::from_refstr(refstr).expect("repository id's are valid refname components")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;
    use qcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_from_str(input: RepoId) {
        let encoded = input.to_string();
        let decoded = RepoId::from_str(&encoded).unwrap();

        assert_eq!(input, decoded);
    }
}
