use std::collections::HashMap;
use std::sync::LazyLock;

use radicle::node::config::ConnectAddress;
use radicle::node::{Address, HostName};
use radicle::profile::Config;

const IRIS: &str = "iris.radicle.network";
const ROSA: &str = "rosa.radicle.network";

static NODES_RENAMED: LazyLock<HashMap<HostName, HostName>> = LazyLock::new(|| {
    HashMap::from([
        (
            HostName::Dns("seed.radicle.garden".to_string()),
            HostName::Dns(IRIS.to_string()),
        ),
        (
            HostName::Dns("iris.radicle.xyz".to_string()),
            HostName::Dns(IRIS.to_string()),
        ),
        (
            HostName::Dns("ash.radicle.garden".to_string()),
            HostName::Dns(ROSA.to_string()),
        ),
        (
            HostName::Dns("rosa.radicle.xyz".to_string()),
            HostName::Dns(ROSA.to_string()),
        ),
    ])
});

fn nodes_renamed_for_option(
    option: &'static str,
    iter: impl IntoIterator<Item = ConnectAddress>,
) -> Vec<String> {
    iter.into_iter().enumerate().fold(Vec::new(), |mut warnings, (i, value)| {
        let old: Address = value.into();
        let old = old.host();
        if let Some(new) = NODES_RENAMED.get(old) {
            warnings.push(format!(
                "Value of configuration option `{option}` at index {i} mentions node with hostname '{old}', which has been renamed to '{new}'. Please edit your configuration file to use the new address."
            ));
        }
        warnings
    })
}

fn nodes_renamed(config: &Config) -> Vec<String> {
    let mut warnings = nodes_renamed_for_option("node.connect", config.node.connect.clone());
    warnings.extend(nodes_renamed_for_option(
        "preferredSeeds",
        config.preferred_seeds.clone(),
    ));

    warnings
}

fn implicit_seeding_policy_allow_scope(config: &Config) -> Vec<String> {
    use radicle::node::config::DefaultSeedingPolicy;
    use radicle::node::policy::Scope::*;

    let DefaultSeedingPolicy::Allow { scope } = config.node.seeding_policy else {
        return vec![];
    };

    if !scope.is_implicit() {
        return vec![];
    }

    vec![format!(
        "Configuration option 'node.seedingPolicy.scope' is not set, and thus takes the value '{}' by default. The default value will change to '{}' in a future release. Please edit your configuration file, and set it to one of ['{}', '{}'] explicitly.",
        scope.into_inner(),
        Followed,
        All,
        Followed,
    )]
}

pub(crate) fn config_warnings(config: &Config) -> Vec<String> {
    let mut warnings = nodes_renamed(config);
    warnings.extend(implicit_seeding_policy_allow_scope(config));

    warnings
}

/// Prints a deprecation warning to standard error.
pub(crate) fn deprecated(old: impl std::fmt::Display, new: impl std::fmt::Display) {
    eprintln!(
        "{} {} The command/option `{old}` is deprecated and will be removed. Please use `{new}` instead.",
        radicle_term::PREFIX_WARNING,
        radicle_term::Paint::yellow("Deprecated:").bold(),
    );
}

/// Prints an obsoletion warning to standard error.
pub(crate) fn obsolete(command: impl std::fmt::Display) {
    eprintln!(
        "{} {} The command `{command}` is obsolete and will be removed. Please stop using it.",
        radicle_term::PREFIX_WARNING,
        radicle_term::Paint::yellow("Obsolete:").bold(),
    );
}
