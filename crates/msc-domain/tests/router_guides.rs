use msc_domain::router_guides::{
    embedded_catalog, embedded_symptoms, embedded_troubleshooting_topics,
};

#[test]
fn router_guide_catalog_is_complete_and_has_renderable_steps() {
    let guides = embedded_catalog().expect("embedded router catalog is valid JSON");
    assert_eq!(guides.len(), 14);
    assert!(guides.iter().all(|guide| {
        !guide.id.is_empty()
            && !guide.family.is_empty()
            && !guide.display_name.is_empty()
            && !guide.search_keywords.is_empty()
            && !guide.steps.is_empty()
            && guide
                .steps
                .iter()
                .all(|step| !step.title.is_empty() && !step.body.is_empty())
            && guide.review.last_reviewed.is_some()
    }));
    assert!(guides.iter().any(|guide| guide.id == "generic-router"));
    assert_eq!(
        guides.iter().map(|guide| guide.steps.len()).sum::<usize>(),
        70
    );
    assert_eq!(
        guides.iter().map(|guide| guide.notes.len()).sum::<usize>(),
        28
    );
    for guide in &guides {
        guide
            .to_matcher_guide()
            .expect("every full guide converts into matcher input");
        guide
            .to_composer_guide()
            .expect("every full guide converts into composer input");
    }
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
    assert!(
        topics
            .iter()
            .all(|topic| !topic.suggested_next_actions.is_empty())
    );
    let symptoms = embedded_symptoms().expect("embedded symptom catalog is valid JSON");
    assert_eq!(symptoms.len(), 18);
    assert!(
        symptoms
            .iter()
            .all(|symptom| !symptom.title.is_empty() && !symptom.description.is_empty())
    );
}
