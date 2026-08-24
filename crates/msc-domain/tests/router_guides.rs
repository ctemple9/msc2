use msc_domain::router_guides::{embedded_catalog, embedded_troubleshooting_topics};

#[test]
fn router_guide_catalog_is_complete_and_has_renderable_steps() {
    let guides = embedded_catalog().expect("embedded router catalog is valid JSON");
    assert_eq!(guides.len(), 14);
    assert!(guides.iter().all(|guide| !guide.id.is_empty()
        && !guide.family.is_empty()
        && !guide.display_name.is_empty()
        && !guide.steps.is_empty()));
    assert!(guides.iter().any(|guide| guide.id == "generic-router"));
}

#[test]
fn troubleshooting_catalog_matches_the_ported_rule_vocabulary() {
    let topics =
        embedded_troubleshooting_topics().expect("embedded troubleshooting catalog is valid JSON");
    assert_eq!(topics.len(), 9);
    for expected in ["local_ip_changed", "double_nat", "cgnat", "no_admin_access"] {
        assert!(
            topics.iter().any(|topic| topic.id == expected),
            "missing {expected}"
        );
    }
}
