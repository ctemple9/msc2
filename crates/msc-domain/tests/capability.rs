//! Hand-written coverage of `capability.rs` against its written
//! specification, `docs/msc2/api-contract/capability-model.md` (P2.6) and
//! D-019's nine-category permission vocabulary (P2.1).
//!
//! Greenfield MSC 2 construction, not a port (per P2.10's own framing in
//! `rolling-plan.md`) — MSC 1 has no equivalent route, so there is no
//! fixture to extract; this suite checks the type against the contract
//! document directly.
//!
//! Test functions are prefixed `capability_` so the plan's Verify command
//! (a plain nextest substring filter, which matches on test name, not
//! file/binary name) selects all of them.

use msc_domain::capability::{
    BedrockBackend, BedrockSupport, CapabilitySet, HelperPresence, HostOs, PermissionCategory,
    ServerTypeSupport,
};
use std::collections::BTreeSet;

#[test]
fn capability_host_os_raw_value_round_trips() {
    for os in HostOs::ALL {
        assert_eq!(HostOs::from_raw_value(os.raw_value()), Some(os));
    }
    assert_eq!(HostOs::from_raw_value("bsd"), None);
}

/// D-019's revised decision names exactly nine categories, in this order:
/// `serverControl`, `players`, `settings`, `addons`, `worlds`,
/// `broadcast`, `networking`, `fleet`, `admin`.
#[test]
fn capability_permission_category_matches_d019_vocabulary() {
    let raw_values: Vec<&str> = PermissionCategory::ALL
        .iter()
        .map(|c| c.raw_value())
        .collect();
    assert_eq!(
        raw_values,
        vec![
            "serverControl",
            "players",
            "settings",
            "addons",
            "worlds",
            "broadcast",
            "networking",
            "fleet",
            "admin",
        ]
    );
}

#[test]
fn capability_permission_category_raw_value_round_trips() {
    for category in PermissionCategory::ALL {
        assert_eq!(
            PermissionCategory::from_raw_value(category.raw_value()),
            Some(category)
        );
    }
    assert_eq!(PermissionCategory::from_raw_value("mods"), None);
}

#[test]
fn capability_bedrock_backend_raw_value_round_trips() {
    for backend in [BedrockBackend::Native, BedrockBackend::VzSidecar] {
        assert_eq!(
            BedrockBackend::from_raw_value(backend.raw_value()),
            Some(backend)
        );
    }
    assert_eq!(BedrockBackend::from_raw_value("qemu"), None);
}

/// `permissions` is a set of granted categories, not an ordered list —
/// duplicate inserts collapse and membership is what callers care about,
/// not insertion order.
#[test]
fn capability_permissions_are_a_set_not_a_list() {
    let mut permissions = BTreeSet::new();
    permissions.insert(PermissionCategory::ServerControl);
    permissions.insert(PermissionCategory::Players);
    permissions.insert(PermissionCategory::ServerControl);
    assert_eq!(permissions.len(), 2);
    assert!(permissions.contains(&PermissionCategory::ServerControl));
    assert!(permissions.contains(&PermissionCategory::Players));
    assert!(!permissions.contains(&PermissionCategory::Admin));
}

/// Constructs capability-model.md §3's own worked example end to end,
/// checking the type can actually represent the confirmed contract's
/// shape rather than just its individual pieces.
#[test]
fn capability_set_represents_the_contract_example() {
    let set = CapabilitySet {
        agent_version: "2.0.0-dev".to_string(),
        api_major: 1,
        api_minor: 0,
        host_os: HostOs::Macos,
        permissions: BTreeSet::from([
            PermissionCategory::ServerControl,
            PermissionCategory::Players,
            PermissionCategory::Settings,
        ]),
        server_types: ServerTypeSupport {
            vanilla: true,
            paper: true,
            fabric: true,
            forge: true,
            neoforge: true,
            bedrock: BedrockSupport {
                supported: false,
                backend: None,
            },
        },
        helpers: HelperPresence {
            playit: false,
            duckdns: false,
            geyser: false,
        },
    };

    assert_eq!(set.agent_version, "2.0.0-dev");
    assert_eq!((set.api_major, set.api_minor), (1, 0));
    assert_eq!(set.host_os, HostOs::Macos);
    assert_eq!(set.permissions.len(), 3);
    assert!(set.server_types.vanilla);
    assert!(!set.server_types.bedrock.supported);
    assert_eq!(set.server_types.bedrock.backend, None);
    assert!(!set.helpers.playit);
}

/// An unsupported Bedrock host still round-trips through equality with a
/// `None` backend — the DTO's own §3 pairing (`supported: false, backend:
/// null`) rather than a state where `supported` and `backend` disagree.
#[test]
fn capability_bedrock_support_equality_respects_backend() {
    let unsupported = BedrockSupport {
        supported: false,
        backend: None,
    };
    let native = BedrockSupport {
        supported: true,
        backend: Some(BedrockBackend::Native),
    };
    assert_ne!(unsupported, native);
    assert_eq!(
        unsupported,
        BedrockSupport {
            supported: false,
            backend: None,
        }
    );
}
