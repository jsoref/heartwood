use std::collections::HashSet;
use std::ops::Deref;
use std::str::FromStr;
use std::{fmt, net};

use cyphernet::addr::PeerAddr;
use localtime::LocalDuration;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::node;
use crate::node::policy::{Scope, SeedingPolicy};
use crate::node::{Address, Alias, NodeId};

/// Peer-to-peer protocol version.
pub type ProtocolVersion = u8;

/// Configured public seeds.
pub mod seeds {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        str::FromStr,
        sync::LazyLock,
    };

    use cyphernet::addr::{tor::OnionAddrV3, HostName, NetAddr};

    use super::{ConnectAddress, NodeId, PeerAddr};

    /// A helper to generate many connect addresses for a node, using port 8776.
    fn to_connect_addresses(id: NodeId, hostnames: Vec<HostName>) -> Vec<ConnectAddress> {
        hostnames
            .into_iter()
            .map(|hostname| PeerAddr::new(id, NetAddr::new(hostname, 8776).into()).into())
            .collect()
    }

    /// A public Radicle seed node for the community.
    pub static RADICLE_NODE_BOOTSTRAP_IRIS: LazyLock<Vec<ConnectAddress>> = LazyLock::new(|| {
        to_connect_addresses(
            #[allow(clippy::unwrap_used)] // Value is manually verified.
            NodeId::from_str("z6MkrLMMsiPWUcNPHcRajuMi9mDfYckSoJyPwwnknocNYPm7").unwrap(),
            vec![
                HostName::Dns("iris.radicle.xyz".to_owned()),
                Ipv6Addr::new(0x2a01, 0x4f9, 0xc010, 0xdfaa, 0, 0, 0, 1).into(),
                Ipv4Addr::new(95, 217, 156, 6).into(),
                #[allow(clippy::unwrap_used)] // Value is manually verified.
                OnionAddrV3::from_str(
                    "irisradizskwweumpydlj4oammoshkxxjur3ztcmo7cou5emc6s5lfid.onion",
                )
                .unwrap()
                .into(),
            ],
        )
    });

    /// A public Radicle seed node for the community.
    pub static RADICLE_NODE_BOOTSTRAP_ROSA: LazyLock<Vec<ConnectAddress>> = LazyLock::new(|| {
        to_connect_addresses(
            #[allow(clippy::unwrap_used)] // Value is manually verified.
            NodeId::from_str("z6Mkmqogy2qEM2ummccUthFEaaHvyYmYBYh3dbe9W4ebScxo").unwrap(),
            vec![
                HostName::Dns("rosa.radicle.xyz".to_owned()),
                Ipv6Addr::new(0x2a01, 0x4ff, 0xf0, 0xabd3, 0, 0, 0, 1).into(),
                Ipv4Addr::new(5, 161, 85, 124).into(),
                #[allow(clippy::unwrap_used)] // Value is manually verified.
                OnionAddrV3::from_str(
                    "rosarad5bxgdlgjnzzjygnsxrwxmoaj4vn7xinlstwglxvyt64jlnhyd.onion",
                )
                .unwrap()
                .into(),
            ],
        )
    });
}

/// Peer-to-peer network.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Network {
    #[default]
    Main,
    Test,
}

impl Network {
    /// Bootstrap nodes for this network.
    pub fn bootstrap(&self) -> Vec<(Alias, ProtocolVersion, Vec<ConnectAddress>)> {
        match self {
            Self::Main => [
                (
                    "iris.radicle.xyz",
                    seeds::RADICLE_NODE_BOOTSTRAP_IRIS.clone(),
                ),
                (
                    "rosa.radicle.xyz",
                    seeds::RADICLE_NODE_BOOTSTRAP_ROSA.clone(),
                ),
            ]
            .into_iter()
            .map(|(a, s)| (Alias::new(a), 1, s))
            .collect(),

            Self::Test => vec![],
        }
    }

    /// Public seeds for this network.
    pub fn public_seeds(&self) -> Vec<ConnectAddress> {
        match self {
            Self::Main => {
                let mut result = seeds::RADICLE_NODE_BOOTSTRAP_IRIS.clone();
                result.extend(seeds::RADICLE_NODE_BOOTSTRAP_ROSA.clone());
                result
            }
            Self::Test => vec![],
        }
    }
}

/// Configuration parameters defining attributes of minima and maxima.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Limits {
    /// Number of routing table entries before we start pruning.
    #[serde(default = "defaults::limit_routing_max_size")]
    pub routing_max_size: usize,

    /// How long to keep a routing table entry before being pruned.
    #[serde(
        default = "defaults::limit_routing_max_age",
        with = "crate::serde_ext::localtime::duration"
    )]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "crate::schemars_ext::localtime::LocalDuration")
    )]
    pub routing_max_age: LocalDuration,

    /// How long to keep a gossip message entry before pruning it.
    #[serde(
        default = "defaults::limit_gossip_max_age",
        with = "crate::serde_ext::localtime::duration"
    )]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "crate::schemars_ext::localtime::LocalDuration")
    )]
    pub gossip_max_age: LocalDuration,

    /// Maximum number of concurrent fetches per peer connection.
    #[serde(default = "defaults::limit_fetch_concurrency")]
    pub fetch_concurrency: usize,

    /// Maximum number of open files.
    #[serde(default = "defaults::limit_max_open_files")]
    pub max_open_files: usize,

    /// Rate limitter settings.
    #[serde(default)]
    pub rate: RateLimits,

    /// Connection limits.
    #[serde(default)]
    pub connection: ConnectionLimits,

    /// Channel limits.
    #[serde(default)]
    pub fetch_pack_receive: FetchPackSizeLimit,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            routing_max_size: defaults::limit_routing_max_size(),
            routing_max_age: defaults::limit_routing_max_age(),
            gossip_max_age: defaults::limit_gossip_max_age(),
            fetch_concurrency: defaults::limit_fetch_concurrency(),
            max_open_files: defaults::limit_max_open_files(),
            rate: RateLimits::default(),
            connection: ConnectionLimits::default(),
            fetch_pack_receive: FetchPackSizeLimit::default(),
        }
    }
}

/// Limiter for byte streams.
///
/// Default: 500MiB
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(into = "String", try_from = "String")]
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(transparent),
    // serde's transparent and try_from/into will conflict, so we tell schemars
    // to ignore them for its generation.
    schemars(!try_from),
    schemars(!into),
)]
pub struct FetchPackSizeLimit {
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "crate::schemars_ext::bytesize::ByteSize")
    )]
    limit: bytesize::ByteSize,
}

impl From<bytesize::ByteSize> for FetchPackSizeLimit {
    fn from(limit: bytesize::ByteSize) -> Self {
        Self { limit }
    }
}

impl From<FetchPackSizeLimit> for String {
    fn from(limit: FetchPackSizeLimit) -> Self {
        limit.to_string()
    }
}

impl TryFrom<String> for FetchPackSizeLimit {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl FromStr for FetchPackSizeLimit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(FetchPackSizeLimit { limit: s.parse()? })
    }
}

impl fmt::Display for FetchPackSizeLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.limit)
    }
}

impl FetchPackSizeLimit {
    /// New `FetchPackSizeLimit` in bytes.
    pub fn bytes(size: u64) -> Self {
        bytesize::ByteSize::b(size).into()
    }

    /// New `FetchPackSizeLimit` in kibibytes.
    pub fn kibibytes(size: u64) -> Self {
        bytesize::ByteSize::kib(size).into()
    }

    /// New `FetchPackSizeLimit` in mebibytes.
    pub fn mebibytes(size: u64) -> Self {
        bytesize::ByteSize::mib(size).into()
    }

    /// New `FetchPackSizeLimit` in gibibytes.
    pub fn gibibytes(size: u64) -> Self {
        bytesize::ByteSize::gib(size).into()
    }

    /// Check if this limit is exceeded by the number of `bytes` provided.
    pub fn exceeded_by(&self, bytes: usize) -> bool {
        bytes >= self.limit.as_u64() as usize
    }
}

impl Default for FetchPackSizeLimit {
    fn default() -> Self {
        Self::mebibytes(500)
    }
}

/// Connection limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ConnectionLimits {
    /// Max inbound connections.
    #[serde(default = "defaults::limit_connections_inbound")]
    pub inbound: usize,

    /// Max outbound connections. Note that this can be higher than the *target* number.
    #[serde(default = "defaults::limit_connections_outbound")]
    pub outbound: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self {
            inbound: defaults::limit_connections_inbound(),
            outbound: defaults::limit_connections_outbound(),
        }
    }
}

/// Rate limts for a single connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RateLimit {
    pub fill_rate: f64,
    pub capacity: usize,
}

/// Rate limits for inbound and outbound connections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RateLimits {
    #[serde(default = "defaults::limit_rate_inbound")]
    pub inbound: RateLimit,

    #[serde(default = "defaults::limit_rate_outbound")]
    pub outbound: RateLimit,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            inbound: defaults::limit_rate_inbound(),
            outbound: defaults::limit_rate_outbound(),
        }
    }
}

/// Full address used to connect to a remote node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(description = "\
    A node address to connect to. Format: An Ed25519 public key in multibase encoding, \
    followed by the symbol '@', followed by an IP address, or a DNS name, or a Tor onion \
    name, followed by the symbol ':', followed by a TCP port number.\
")
)]
pub struct ConnectAddress(
    #[serde(with = "crate::serde_ext::string")]
    #[cfg_attr(feature = "schemars", schemars(
        with = "String",
        regex(pattern = r"^.+@.+:((6553[0-5])|(655[0-2][0-9])|(65[0-4][0-9]{2})|(6[0-4][0-9]{3})|([1-5][0-9]{4})|([0-5]{0,5})|([0-9]{1,4}))$"),
        extend("examples" = [
            "z6MkrLMMsiPWUcNPHcRajuMi9mDfYckSoJyPwwnknocNYPm7@rosa.radicle.xyz:8776",
            "z6MkvUJtYD9dHDJfpevWRT98mzDDpdAtmUjwyDSkyqksUr7C@xmrhfasfg5suueegrnc4gsgyi2tyclcy5oz7f5drnrodmdtob6t2ioyd.onion:8776",
            "z6MknSLrJoTcukLrE435hVNQT4JUhbvWLX4kUzqkEStBU8Vi@seed.example.com:8776",
            "z6MkkfM3tPXNPrPevKr3uSiQtHPuwnNhu2yUVjgd2jXVsVz5@192.0.2.0:31337",
        ]),
    ))]
    PeerAddr<NodeId, Address>,
);

impl From<PeerAddr<NodeId, Address>> for ConnectAddress {
    fn from(value: PeerAddr<NodeId, Address>) -> Self {
        Self(value)
    }
}

impl From<ConnectAddress> for (NodeId, Address) {
    fn from(value: ConnectAddress) -> Self {
        (value.0.id, value.0.addr)
    }
}

impl From<(NodeId, Address)> for ConnectAddress {
    fn from((id, addr): (NodeId, Address)) -> Self {
        Self(PeerAddr { id, addr })
    }
}

impl From<ConnectAddress> for Address {
    fn from(value: ConnectAddress) -> Self {
        value.0.addr
    }
}

impl Deref for ConnectAddress {
    type Target = PeerAddr<NodeId, Address>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Peer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum PeerConfig {
    /// Static peer set. Connect to the configured peers and maintain the connections.
    Static,
    /// Dynamic peer set.
    Dynamic,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self::Dynamic
    }
}

/// Relay configuration.
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Relay {
    /// Always relay messages.
    Always,
    /// Never relay messages.
    Never,
    /// Relay messages when applicable.
    #[default]
    Auto,
}

/// Proxy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "mode")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AddressConfig {
    /// Proxy connections to this address type.
    Proxy {
        /// Proxy address.
        address: net::SocketAddr,
    },
    /// Forward address to the next layer. Either this is the global proxy,
    /// or the operating system, via DNS.
    Forward,
}

/// Default seeding policy. Applies when no repository policies for the given repo are found.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "default")]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum DefaultSeedingPolicy {
    /// Allow seeding.
    Allow {
        /// Seeding scope.
        #[serde(default)]
        scope: Scope,
    },
    /// Block seeding.
    #[default]
    Block,
}

impl DefaultSeedingPolicy {
    /// Is this an "allow" policy.
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    /// Seed everything from anyone.
    pub fn permissive() -> Self {
        Self::Allow { scope: Scope::All }
    }
}

impl From<DefaultSeedingPolicy> for SeedingPolicy {
    fn from(policy: DefaultSeedingPolicy) -> Self {
        match policy {
            DefaultSeedingPolicy::Block => Self::Block,
            DefaultSeedingPolicy::Allow { scope } => Self::Allow { scope },
        }
    }
}

/// Service configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "NodeConfig")
)]
pub struct Config {
    /// Node alias.
    pub alias: Alias,
    /// Socket address (a combination of IPv4 or IPv6 address and TCP port) to listen on.
    #[serde(default)]
    #[cfg_attr(feature = "schemars", schemars(example = &"127.0.0.1:8776"))]
    pub listen: Vec<net::SocketAddr>,
    /// Peer configuration.
    #[serde(default)]
    pub peers: PeerConfig,
    /// Peers to connect to on startup.
    /// Connections to these peers will be maintained.
    #[serde(default)]
    pub connect: HashSet<ConnectAddress>,
    /// Specify the node's public addresses
    #[serde(default)]
    pub external_addresses: Vec<Address>,
    /// Global proxy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<net::SocketAddr>,
    /// Onion address config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onion: Option<AddressConfig>,
    /// Peer-to-peer network.
    #[serde(default)]
    pub network: Network,
    /// Log level.
    #[serde(default = "defaults::log")]
    #[serde(with = "crate::serde_ext::string")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "crate::schemars_ext::log::Level")
    )]
    pub log: log::Level,
    /// Whether or not our node should relay messages.
    #[serde(default, deserialize_with = "crate::serde_ext::ok_or_default")]
    pub relay: Relay,
    /// Configured service limits.
    #[serde(default)]
    pub limits: Limits,
    /// Number of worker threads to spawn.
    #[serde(default = "defaults::workers")]
    pub workers: usize,
    /// Default seeding policy.
    #[serde(default)]
    pub seeding_policy: DefaultSeedingPolicy,
    /// Extra fields that aren't supported.
    #[serde(flatten, skip_serializing)]
    pub extra: json::Map<String, json::Value>,
}

impl Config {
    pub fn test(alias: Alias) -> Self {
        Self {
            network: Network::Test,
            ..Self::new(alias)
        }
    }

    pub fn new(alias: Alias) -> Self {
        Self {
            alias,
            peers: PeerConfig::default(),
            listen: vec![],
            connect: HashSet::default(),
            external_addresses: vec![],
            network: Network::default(),
            proxy: None,
            onion: None,
            relay: Relay::default(),
            limits: Limits::default(),
            workers: defaults::workers(),
            log: defaults::log(),
            seeding_policy: DefaultSeedingPolicy::default(),
            extra: json::Map::default(),
        }
    }

    pub fn peer(&self, id: &NodeId) -> Option<&Address> {
        self.connect
            .iter()
            .find(|ca| &ca.id == id)
            .map(|ca| &ca.addr)
    }

    pub fn peers(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.connect.iter().cloned().map(|p| p.id)
    }

    pub fn is_persistent(&self, id: &NodeId) -> bool {
        self.peer(id).is_some()
    }

    /// Are we a relay node? This determines what we do with gossip messages from other peers.
    pub fn is_relay(&self) -> bool {
        match self.relay {
            // In "auto" mode, we only relay if we're a public seed node.
            // This reduces traffic for private nodes, as well as message redundancy.
            Relay::Auto => !self.external_addresses.is_empty(),
            Relay::Never => false,
            Relay::Always => true,
        }
    }

    pub fn features(&self) -> node::Features {
        node::Features::SEED
    }
}

/// Defaults as functions, for serde.
mod defaults {
    /// Default number of workers to spawn.
    #[inline]
    pub const fn workers() -> usize {
        8
    }

    /// Log level.
    #[inline]
    pub const fn log() -> log::Level {
        log::Level::Info
    }

    #[inline]
    pub const fn limit_connections_inbound() -> usize {
        128
    }

    #[inline]
    pub const fn limit_connections_outbound() -> usize {
        16
    }

    #[inline]
    pub const fn limit_routing_max_size() -> usize {
        1000
    }

    #[inline]
    pub const fn limit_routing_max_age() -> localtime::LocalDuration {
        localtime::LocalDuration::from_mins(7 * 24 * 60) // One week
    }

    #[inline]
    pub const fn limit_gossip_max_age() -> localtime::LocalDuration {
        localtime::LocalDuration::from_mins(2 * 7 * 24 * 60) // Two weeks
    }

    #[inline]
    pub const fn limit_fetch_concurrency() -> usize {
        1
    }

    #[inline]
    pub const fn limit_max_open_files() -> usize {
        4096
    }

    #[inline]
    pub const fn limit_rate_inbound() -> super::RateLimit {
        super::RateLimit {
            fill_rate: 5.0,
            capacity: 1024,
        }
    }

    #[inline]
    pub const fn limit_rate_outbound() -> super::RateLimit {
        super::RateLimit {
            fill_rate: 10.0,
            capacity: 2048,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    #[test]
    fn partial() {
        use super::Config;
        use serde_json::json;

        let config: Config = serde_json::from_value(json!({
            "alias": "example",
            "limits": {
                "connection": {
                    "inbound": 1337,
                },
            },
        }
        ))
        .unwrap();
        assert_eq!(config.limits.connection.inbound, 1337);
        assert_eq!(
            config.limits.connection.outbound,
            super::defaults::limit_connections_outbound()
        );

        let config: Config = serde_json::from_value(json!({
            "alias": "example",
            "limits": {
                "connection": {
                    "outbound": 1337,
                },
            },
        }
        ))
        .unwrap();
        assert_eq!(
            config.limits.connection.inbound,
            super::defaults::limit_connections_inbound()
        );
        assert_eq!(config.limits.connection.outbound, 1337);
    }
}
