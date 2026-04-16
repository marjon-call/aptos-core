// PoC Test Template for Aptos Node Security Audit
// =================================================
//
// This template tests the node itself — config loading, network setup,
// service initialization, and the wiring between subsystems.
//
// Usage:
//   cargo test -p aptos-node --lib -- poc::test_poc --nocapture
//
// Gotchas discovered during validation:
//   - default_validator_config() has NO keys/identity — use full_test_config()
//     for anything involving network setup or identity
//   - Network setup panics if mutual_authentication is disabled on validator network
//   - expose_system_information defaults to TRUE — potential info leakage
//   - Config merging via serde_yaml overlay preserves unset defaults
//   - full_test_config() generates real genesis, keys, waypoint — takes ~2-3s

#![allow(unused_imports, dead_code, unused_variables)]

use crate::{create_single_node_test_config, load_node_config, network};
use aptos_cached_packages;
use aptos_config::config::{NodeConfig, PersistableConfig, WaypointConfig};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::RwLock;
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, waypoint::Waypoint};
use rand::{rngs::StdRng, SeedableRng};
use std::{fs, sync::Arc};

// ============================================================================
// MOCK INFRASTRUCTURE
// ============================================================================

/// Mock database for testing network and config setup without real storage
struct MockDb;
impl DbReader for MockDb {}
impl DbWriter for MockDb {}

fn mock_db_rw() -> DbReaderWriter {
    DbReaderWriter::new(MockDb {})
}

// ============================================================================
// HELPER: Config creation
// ============================================================================

/// Create a default validator config with temp directory.
/// WARNING: Has NO keys/identity — cannot be used for network setup.
fn default_validator_config() -> (NodeConfig, TempPath) {
    let temp = TempPath::new();
    let mut config = NodeConfig::get_default_validator_config();
    config.set_data_dir(temp.path().to_path_buf());
    config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());
    (config, temp)
}

/// Create a full single-node test config (genesis, keys, identity, waypoint).
/// Use this for anything involving network setup or identity.
fn full_test_config() -> (NodeConfig, TempPath) {
    let temp = TempPath::new();
    let test_dir = temp.path().to_path_buf();
    fs::DirBuilder::new()
        .recursive(true)
        .create(&test_dir)
        .unwrap();
    let config = create_single_node_test_config(
        &None,
        &None,
        &test_dir,
        true,  // random_ports
        false, // lazy
        false, // performance
        aptos_cached_packages::head_release_bundle(),
        StdRng::from_entropy(),
    )
    .unwrap();
    (config, temp)
}

/// Create an event subscription service with mock DB
fn mock_event_service() -> EventSubscriptionService {
    EventSubscriptionService::new(Arc::new(RwLock::new(mock_db_rw())))
}

// ============================================================================
// HELPER: Network setup
// ============================================================================

/// Set up networks from a config — returns network handles.
/// Panics if config is invalid (e.g. mutual_authentication disabled on validator).
/// Requires full_test_config() — default_validator_config() will panic on missing keys.
fn setup_test_networks(
    node_config: &NodeConfig,
    event_service: &mut EventSubscriptionService,
) {
    let peers_and_metadata = network::create_peers_and_metadata(node_config);
    let _ = network::setup_networks_and_get_interfaces(
        node_config,
        ChainId::test(),
        peers_and_metadata,
        event_service,
    );
}

// ============================================================================
// HELPER: Config manipulation
// ============================================================================

/// Merge a YAML override into a default config
fn config_with_override(yaml_override: &str) -> NodeConfig {
    let override_value: serde_yaml::Value =
        serde_yaml::from_str(yaml_override).unwrap();
    aptos_config::config::merge_node_config(
        NodeConfig::get_default_validator_config(),
        override_value,
    )
    .unwrap()
}

// ============================================================================
// PoC TEST — Fill in your exploit logic below
// ============================================================================

#[test]
fn test_poc() {
    // --- Initialize test environment ---
    // let (config, _temp) = default_validator_config();  // lightweight, no keys
    // let (config, _temp) = full_test_config();          // full genesis + keys

    // --- Manipulate config ---
    // e.g. disable security features, inject bad values, test edge cases
    // let config = config_with_override(r#"
    //     api:
    //         address: 0.0.0.0:9999
    // "#);

    // --- Test network setup ---
    // let mut event_service = mock_event_service();
    // setup_test_networks(&config, &mut event_service);  // needs full_test_config

    // --- Verify behavior ---
    // e.g. assert panics on bad config, check defaults are secure,
    //      verify mutual_auth enforcement, test config sanitization
}
