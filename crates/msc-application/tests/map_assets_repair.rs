//! Essential success-boundary regression: acquiring bytes alone must not claim repair.
//! Synthetic reports avoid renderer/network/timing assumptions; expected runtime < 2 seconds.
use msc_application::map_assets::classify_repair;
use msc_domain::map_assets::{Area, Binding, Classification as C, Report};
use std::collections::BTreeMap;

fn report(counts: &[(C, u64)]) -> Report {
    Report {
        schema_version: 1,
        binding: Binding {
            agent_host_id: "host".into(),
            server_id: "server".into(),
            slot_id: "slot".into(),
            world_incarnation: "world".into(),
            revision: "revision".into(),
        },
        snapshot_id: "saved-chunks".into(),
        snapshot_minecraft_version: Some("1.21.1".into()),
        resource_generation_id: "resources".into(),
        geometry_generation_id: Some("validated-geometry".into()),
        dimension: "example:dimension".into(),
        area: Area {
            min: [0, 0, 0],
            max: [15, 15, 15],
        },
        operation_id: "operation".into(),
        outcome: "ready".into(),
        repair: None,
        visual_acceptance: "pending".into(),
        scope: "controlled saved area".into(),
        inspected_blocks: 4096,
        inspected_chunks: 1,
        distinct_states: 1,
        visible_faces: None,
        counts: counts.iter().copied().collect(),
        diagnostics: vec![],
        omitted_issues: 0,
        omitted_samples: 0,
        sources: vec![],
    }
}

#[test]
fn repair_requires_original_failures_resolved_and_artifacts_adoptable_in_same_saved_area() {
    let before = report(&[(C::MissingTexture, 8)]);
    let mut unchanged = before.clone();
    classify_repair(Some(&before), &mut unchanged, true);
    assert_eq!(unchanged.outcome, "needs_input");
    let mut unsupported = report(&[(C::UnsupportedLoader, 8)]);
    classify_repair(Some(&before), &mut unsupported, true);
    assert_eq!(unsupported.outcome, "unsupported");
    for (validated, geometry, same_snapshot, same_area) in [
        (false, true, true, true),
        (true, false, true, true),
        (true, true, false, true),
        (true, true, true, false),
    ] {
        let mut after = report(&[(C::ModelResolved, 8)]);
        if !geometry {
            after.geometry_generation_id = None;
        }
        if !same_snapshot {
            after.snapshot_id = "different-chunks".into();
        }
        if !same_area {
            after.area.min[0] = 1;
        }
        classify_repair(Some(&before), &mut after, validated);
        assert_ne!(after.outcome, "repaired");
        assert_eq!(after.visual_acceptance, "pending");
    }
    let mut wrong_host = report(&[(C::ModelResolved, 8)]);
    wrong_host.binding.agent_host_id = "other-host".into();
    classify_repair(Some(&before), &mut wrong_host, true);
    assert_ne!(wrong_host.outcome, "repaired");
    assert!(!wrong_host.repair.unwrap().same_saved_area);
    let mut partial = report(&[(C::MissingTexture, 2), (C::ModelResolved, 6)]);
    classify_repair(Some(&before), &mut partial, true);
    assert_eq!(partial.outcome, "partially_repaired");
    let mut after = report(&[(C::ModelResolved, 8)]);
    after.resource_generation_id = "replacement".into();
    classify_repair(Some(&before), &mut after, true);
    assert_eq!(after.outcome, "repaired");
    let evidence = after.repair.unwrap();
    assert!(evidence.same_saved_area);
    assert_eq!(
        evidence.before_counts,
        BTreeMap::from([(C::MissingTexture, 8)])
    );
    assert_eq!(evidence.target_generation_id, "replacement");
}
