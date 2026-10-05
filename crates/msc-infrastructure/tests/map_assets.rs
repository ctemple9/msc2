//! Essential: wrong namespace/empty classification can make a repair report falsely succeed;
//! cancelled publication must never replace a usable generation. Controlled data, no clocks/network.
use msc_domain::map_assets::{Area, Binding, Classification as C, Report, ResourceManifest};
use msc_infrastructure::map_assets::{
    hash,
    inventory::{Inventory, Resource},
    resolver::Resolver,
    store::Store,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

fn resource(value: serde_json::Value) -> Resource {
    Resource {
        sha256: hash(value.to_string().as_bytes()),
        json: Some(value),
        sources: vec!["fixture-source".into()],
        ..Default::default()
    }
}
#[test]
fn namespaced_dependencies_and_empty_models_do_not_hide_capture_requirements() {
    let mut inventory = Inventory::default();
    inventory.resources.insert(
        "assets/alpha/blockstates/panel.json".into(),
        resource(json!({"variants":{"":{"model":"alpha:block/panel"}}})),
    );
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({"parent":"beta:block/base","textures":{"surface":"alpha:block/panel"}})),
    );
    inventory.resources.insert("assets/beta/models/block/base.json".into(),resource(json!({"elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{"north":{"texture":"#surface"}}}]})));
    inventory.resources.insert(
        "assets/alpha/textures/block/panel.png".into(),
        Resource::default(),
    );
    // Same local name in beta has different content; it must never replace alpha's texture.
    inventory.resources.insert(
        "assets/beta/textures/block/panel.png".into(),
        Resource {
            invalid: true,
            ..Default::default()
        },
    );
    let classify = |inventory: &Inventory, entity: bool| {
        Resolver { inventory }
            .inspect("alpha:panel", &BTreeMap::new(), entity)
            .into_iter()
            .map(|f| f.classification)
            .collect::<Vec<_>>()
    };
    assert!(classify(&inventory, false).contains(&C::ModelResolved));
    assert!(!classify(&inventory, false).contains(&C::MissingTexture));
    assert!(classify(&inventory, true).contains(&C::MissingContext));
    inventory
        .resources
        .get_mut("assets/alpha/textures/block/panel.png")
        .unwrap()
        .conflict = true;
    assert!(classify(&inventory, false).contains(&C::SelectionUnknown));
    assert!(!classify(&inventory, false).contains(&C::ModelResolved));
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({})),
    );
    assert!(!classify(&inventory, false).contains(&C::IntentionalEmpty));
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({"elements":[]})),
    );
    assert!(classify(&inventory, false).contains(&C::IntentionalEmpty));
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({"loader":"fixture:custom","elements":[]})),
    );
    assert!(classify(&inventory, false).contains(&C::UnsupportedLoader));
    assert!(!classify(&inventory, false).contains(&C::IntentionalEmpty));
}
#[test]
fn cancelled_candidate_retains_current_previous_and_reader_data() {
    let root =
        std::env::temp_dir().join(format!("msc-map-resource-store-{}", uuid::Uuid::new_v4()));
    let store = Store::open(root.clone()).unwrap();
    let binding = Binding {
        agent_host_id: "host".into(),
        server_id: "server".into(),
        slot_id: "slot".into(),
        world_incarnation: "incarnation".into(),
        revision: "revision".into(),
    };
    let manifest = ResourceManifest {
        schema_version: 1,
        generation_id: hash(b"resources"),
        minecraft_version: Some("1.21.1".into()),
        loader: "neoforge".into(),
        loader_version: Some("21.1.251".into()),
        resolver_version: "fixture".into(),
        renderer_version: "fixture".into(),
        capture_formats: vec![],
        sources: vec![],
        selected_pack_order: None,
        selection_known: false,
        selection_revision: "selection".into(),
        config_fingerprint: "config".into(),
        manifest_receipts: BTreeMap::new(),
        archive_entries: 0,
        decompressed_bytes: 0,
        resource_documents: 0,
        resource_conflicts: 0,
    };
    let mut report = Report {
        schema_version: 1,
        binding: binding.clone(),
        snapshot_id: hash(b"snapshot"),
        snapshot_minecraft_version: Some("1.21.1".into()),
        resource_generation_id: manifest.generation_id.clone(),
        geometry_generation_id: None,
        dimension: "minecraft:overworld".into(),
        area: Area {
            min: [0, 0, 0],
            max: [0, 0, 0],
        },
        operation_id: "first".into(),
        outcome: "checked".into(),
        visual_acceptance: "pending".into(),
        scope: "fixture".into(),
        inspected_blocks: 1,
        inspected_chunks: 1,
        distinct_states: 1,
        visible_faces: None,
        counts: BTreeMap::new(),
        diagnostics: vec![],
        omitted_issues: 0,
        omitted_samples: 0,
        sources: vec![],
    };
    let candidate = store.begin().unwrap();
    store
        .publish(&candidate, &manifest, &report, &|| false)
        .unwrap();
    drop(candidate);
    let first = store.pointer(&binding).unwrap().unwrap();
    let reader = store.lease(&first.current).unwrap();
    report.operation_id = "second".into();
    let candidate = store.begin().unwrap();
    store
        .publish(&candidate, &manifest, &report, &|| false)
        .unwrap();
    drop(candidate);
    let second = store.pointer(&binding).unwrap().unwrap();
    assert_eq!(second.previous, Some(first.current.clone()));
    report.operation_id = "cancelled".into();
    let candidate = store.begin().unwrap();
    assert!(
        store
            .publish(&candidate, &manifest, &report, &|| true)
            .is_err()
    );
    drop(candidate);
    let retained = store.pointer(&binding).unwrap().unwrap();
    assert_eq!(retained.current, second.current);
    assert_eq!(retained.previous, second.previous);
    assert_eq!(reader.report.unwrap().operation_id, "first");
    assert!(store.lease(&first.current).is_ok());
    let record = root
        .join("generations")
        .join(format!("{}.json", retained.current));
    let mut damaged: serde_json::Value =
        serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    damaged["report"]["operationId"] = json!("tampered");
    fs::write(record, serde_json::to_vec(&damaged).unwrap()).unwrap();
    assert!(store.lease(&retained.current).is_err());
    assert!(store.lease(&first.current).is_ok());
    drop(store);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn negative_coordinates_and_missing_saved_chunks_are_not_air_successes() {
    use fastnbt::Value;
    use msc_infrastructure::map_assets::saved_terrain::WorldSource;
    use std::collections::HashMap;
    let root = std::env::temp_dir().join(format!("msc-map-saved-chunks-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("region")).unwrap();
    fs::write(root.join("level.dat"), b"controlled-snapshot-receipt").unwrap();
    let palette = Value::Compound(HashMap::from([(
        "Name".into(),
        Value::String("fixture:block".into()),
    )]));
    let section = Value::Compound(HashMap::from([
        ("Y".into(), Value::Byte(-4)),
        (
            "block_states".into(),
            Value::Compound(HashMap::from([(
                "palette".into(),
                Value::List(vec![palette]),
            )])),
        ),
    ]));
    let chunk = Value::Compound(HashMap::from([
        ("xPos".into(), Value::Int(-1)),
        ("zPos".into(), Value::Int(-1)),
        ("Status".into(), Value::String("minecraft:full".into())),
        ("sections".into(), Value::List(vec![section])),
    ]));
    let nbt = fastnbt::to_bytes(&chunk).unwrap();
    let mut region = vec![0u8; 12288];
    let index = 1023 * 4;
    region[index + 2] = 2;
    region[index + 3] = 1;
    region[8192..8196].copy_from_slice(&((nbt.len() + 1) as u32).to_be_bytes());
    region[8196] = 3;
    region[8197..8197 + nbt.len()].copy_from_slice(&nbt);
    fs::write(root.join("region/r.-1.-1.mca"), &region).unwrap();
    let area = Area {
        min: [-1, -64, -1],
        max: [0, -64, 0],
    };
    let source = WorldSource::Directory(root.clone());
    let terrain = source
        .inspect("minecraft:overworld", area, &|| false)
        .unwrap();
    assert_eq!(terrain.chunks, 1);
    assert_eq!(terrain.missing.len(), 3);
    assert_eq!(terrain.blocks.len(), 1);
    assert_eq!(terrain.blocks[0].position, [-1, -64, -1]);
    assert_eq!(terrain.blocks[0].id, "fixture:block");
    // Truncating a saved record is an error, never a substituted air chunk.
    fs::write(root.join("region/r.-1.-1.mca"), &region[..8198]).unwrap();
    assert!(
        source
            .inspect("minecraft:overworld", area, &|| false)
            .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

/// Essential: wrong-client substitution or retry loss breaks a repair and can
/// repaint terrain with another release. Fixed bytes/fake transport, <2 s locally.
#[test]
fn exact_download_refuses_mismatch_and_reuses_verified_bytes_without_network() {
    use msc_infrastructure::addon_provider::{AddonTransport, RawResponse, TransportError};
    use msc_infrastructure::map_assets::acquire::{RequiredSource, acquire};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Provider {
        calls: AtomicUsize,
        bytes: Vec<u8>,
    }
    impl AddonTransport for Provider {
        fn get(
            &self,
            _: &str,
            _: &str,
            _: &[(&str, &str)],
            _: u64,
        ) -> Result<RawResponse, TransportError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(RawResponse {
                status: 200,
                body: self.bytes.clone(),
            })
        }
        fn post_json(
            &self,
            _: &str,
            _: &str,
            _: &serde_json::Value,
            _: &[(&str, &str)],
            _: u64,
        ) -> Result<RawResponse, TransportError> {
            panic!("unexpected identity guess")
        }
    }
    let root = std::env::temp_dir().join(format!("msc-map-acquire-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(root.join("server/mods")).unwrap();
    let source = RequiredSource {
        identity: "exact-release-file".into(),
        path: "mods/client.jar".into(),
        provider: "fixture".into(),
        project_id: Some("project".into()),
        release_id: Some("release".into()),
        file_id: Some("file".into()),
        hashes: BTreeMap::from([("sha256".into(), hash(b"exact"))]),
        bytes: 5,
        urls: vec!["https://cdn.modrinth.com/exact.jar".into()],
        reason: None,
    };
    let provider = Provider {
        calls: AtomicUsize::new(0),
        bytes: b"wrong".to_vec(),
    };
    let issue = acquire(
        &source,
        &root.join("server"),
        &root.join("downloads"),
        &provider,
        false,
        &|| false,
    )
    .unwrap_err();
    assert_eq!(issue.code, "published_checksum_mismatch");
    assert_eq!(issue.source.release_id.as_deref(), Some("release"));
    let provider = Provider {
        calls: AtomicUsize::new(0),
        bytes: b"exact".to_vec(),
    };
    let acquired = acquire(
        &source,
        &root.join("server"),
        &root.join("downloads"),
        &provider,
        false,
        &|| false,
    )
    .unwrap();
    assert_eq!(fs::read(&acquired).unwrap(), b"exact");
    assert!(!root.join("server/mods/client.jar").exists());
    assert_eq!(
        acquire(
            &source,
            &root.join("server"),
            &root.join("downloads"),
            &provider,
            true,
            &|| false
        )
        .unwrap(),
        acquired
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
    let mut blocked = source.clone();
    blocked.identity = "blocked-file".into();
    blocked.path = "mods/blocked.jar".into();
    blocked.urls.clear();
    assert_eq!(
        acquire(
            &blocked,
            &root.join("server"),
            &root.join("downloads"),
            &provider,
            false,
            &|| false
        )
        .unwrap_err()
        .code,
        "manual_download_required"
    );
    assert!(acquired.exists());
    fs::remove_dir_all(root).unwrap();
}

// Essential: store pointer tests cannot catch late renderer jobs replacing the
// visible scene. Explicit completions cover this without helpers, clocks or I/O.
#[test]
fn renderer_coordinator_retains_scene_on_failure_cancel_and_stale_completion() {
    use msc_infrastructure::map_assets::adoption::Coordinator;
    use std::sync::Arc;
    let mut coordinator = Coordinator::<String>::default();
    let first = coordinator
        .begin("host-a/slot/dimension", "r1", "first", false)
        .unwrap();
    let usable = Arc::new("usable".to_string());
    assert!(coordinator.finish(&first, Some(usable.clone()), "ready"));
    assert!(
        coordinator
            .begin("host-a/slot/dimension", "r1", "duplicate", false)
            .is_none()
    );
    let old = coordinator
        .begin("host-a/slot/dimension", "r2", "old", false)
        .unwrap();
    let next = coordinator
        .begin("host-a/slot/dimension", "r3", "new", false)
        .unwrap();
    assert!(old.cancelled());
    assert!(!coordinator.finish(&old, Some(Arc::new("stale".into())), "ready"));
    assert!(coordinator.finish(&next, None, "failed"));
    assert!(Arc::ptr_eq(
        coordinator.entries["host-a/slot/dimension"]
            .current
            .as_ref()
            .unwrap(),
        &usable
    ));
    let cancelled = coordinator
        .begin("host-a/slot/dimension", "r4", "cancel", false)
        .unwrap();
    coordinator.cancel("host-a/slot/dimension");
    assert!(!coordinator.finish(&cancelled, Some(Arc::new("cancelled".into())), "ready"));
    assert!(coordinator.abort(&cancelled, "cancelled"));
    let host_b = coordinator
        .begin("host-b/slot/dimension", "r1", "other-host", false)
        .unwrap();
    assert!(coordinator.finish(&host_b, Some(Arc::new("other".into())), "ready"));
    assert!(Arc::ptr_eq(
        coordinator.entries["host-a/slot/dimension"]
            .current
            .as_ref()
            .unwrap(),
        &usable
    ));
}

// Essential: namespace flattening and invented opacity can hide real terrain.
// The existing inspection regression does not exercise the production adapter
// or prove that changing a parent texture invalidates dependent palettes.
#[test]
fn private_adapter_keeps_namespaces_non_occlusion_and_parent_texture_dependencies() {
    use msc_infrastructure::map_assets::adapter::{Adapter, block_id, document, reference};
    assert_ne!(
        reference("alpha:block/panel").unwrap(),
        reference("beta:block/panel").unwrap()
    );
    assert_eq!(reference("minecraft:block/stone").unwrap(), "block/stone");
    assert!(block_id("alpha:panel").unwrap().ends_with("_glass"));
    let rewritten=document("assets/alpha/models/block/panel.json",json!({"parent":"beta:block/base","textures":{"surface":"alpha:block/panel"},"elements":[{"faces":{"north":{"texture":"#surface"}}}],"payload":{"id":"alpha:original"}})).unwrap();
    assert_eq!(rewritten["parent"], reference("beta:block/base").unwrap());
    assert_eq!(
        rewritten["textures"]["surface"],
        reference("alpha:block/panel").unwrap()
    );
    assert_eq!(
        rewritten["elements"][0]["faces"]["north"]["texture"],
        "#surface"
    );
    assert_eq!(rewritten["payload"]["id"], "alpha:original");
    let mut inventory = Inventory::default();
    inventory.resources.insert(
        "assets/alpha/blockstates/panel.json".into(),
        resource(json!({"variants":{"facing=north":{"model":"alpha:block/panel"}}})),
    );
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({"parent":"beta:block/base"})),
    );
    inventory.resources.insert("assets/beta/models/block/base.json".into(),resource(json!({"textures":{"surface":"beta:block/base"},"elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{"north":{"texture":"#surface"}}}]})));
    inventory.resources.insert(
        "assets/beta/textures/block/base.png".into(),
        Resource {
            sha256: hash(b"first-texture"),
            ..Default::default()
        },
    );
    let state = BTreeMap::from([("facing".into(), "north".into())]);
    let first = Adapter {
        inventory: &inventory,
    }
    .palette("alpha:panel", &state, false)
    .unwrap();
    assert!(first.model_resolved);
    let contextual = Adapter {
        inventory: &inventory,
    }
    .palette("alpha:panel", &state, true)
    .unwrap();
    assert!(!contextual.model_resolved);
    assert!(contextual.renderer_id.contains("/fallback/"));
    inventory.resources.insert(
        "assets/unrelated/models/block/other.json".into(),
        resource(json!({"elements":[]})),
    );
    assert_eq!(
        first.fingerprint,
        Adapter {
            inventory: &inventory
        }
        .palette("alpha:panel", &state, false)
        .unwrap()
        .fingerprint
    );
    inventory
        .resources
        .get_mut("assets/beta/textures/block/base.png")
        .unwrap()
        .sha256 = hash(b"changed-texture");
    assert_ne!(
        first.fingerprint,
        Adapter {
            inventory: &inventory
        }
        .palette("alpha:panel", &state, false)
        .unwrap()
        .fingerprint
    );
    let missing = Adapter {
        inventory: &inventory,
    }
    .palette("missing:block", &BTreeMap::new(), false)
    .unwrap();
    assert!(!missing.model_resolved);
    assert!(missing.renderer_id.ends_with("_glass"));
    assert!(missing.renderer_id.contains("/fallback/"));
    inventory.resources.insert(
        "assets/alpha/models/block/panel.json".into(),
        resource(json!({"elements":[]})),
    );
    let empty = Adapter {
        inventory: &inventory,
    }
    .palette("alpha:panel", &state, false)
    .unwrap();
    assert!(!empty.model_resolved);
    assert!(empty.classifications.contains(&C::IntentionalEmpty));
    assert!(!empty.renderer_id.contains("/fallback/"));
}
