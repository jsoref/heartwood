use std::collections::HashMap;
use std::sync::LazyLock;

use radicle::node::config::ConnectAddress;
use radicle::node::Address;
use radicle::profile::Config;

static NODES_RENAMED: LazyLock<HashMap<Address, Address>> = LazyLock::new(|| {
    HashMap::from([
        (
            "seed.radicle.garden:8776".parse().unwrap(),
            "iris.radicle.xyz:8776".parse().unwrap(),
        ),
        (
            "ash.radicle.garden:8776".parse().unwrap(),
            "rosa.radicle.xyz:8776".parse().unwrap(),
        ),
    ])
});

fn nodes_renamed_for_option(
    option: &'static str,
    iter: impl IntoIterator<Item = ConnectAddress>,
) -> Vec<String> {
    let mut warnings: Vec<String> = vec![];

    for (i, value) in iter.into_iter().enumerate() {
        let old: Address = value.into();
        if let Some(new) = NODES_RENAMED.get(&old) {
            warnings.push(format!(
                "Value of configuration option `{option}` at index {i} mentions node with address '{}', which has been renamed to '{}'. Please update your configuration.",
                old, new
            ));
        }
    }

    warnings
}

pub(crate) fn nodes_renamed(config: &Config) -> Vec<String> {
    let mut warnings = nodes_renamed_for_option("node.connect", config.node.connect.clone());
    warnings.extend(nodes_renamed_for_option(
        "preferred_seeds",
        config.preferred_seeds.clone(),
    ));
    warnings
}
