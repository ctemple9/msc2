//! Port of `RouterPortForwardTroubleshootingEngine.swift` (550 lines) — "a
//! rule-based troubleshooting engine for router and port-forwarding
//! failures. Accepts user-reported symptoms and returns prioritised causes
//! and recommended actions" (the file's own doc comment).
//!
//! **Excluded per the port plan's deletion test:** `RouterPortForwardSymptom`
//! (the `title`/`description` UI text attached to each `SymptomId`) and the
//! `supportedSymptoms`/`symptom(id:)` static-lookup surface. Nothing here
//! computes over that text — it exists purely so a client can render a
//! symptom picklist, which is client-owned static data, not executable
//! behavior. Only `SymptomId` itself (the thing rules actually match
//! against) is ported.
//!
//! **`makeRules(repository:)`'s `repository` parameter is dropped.** Reading
//! the real 9-rule table, nothing in it reads `repository` — it exists only
//! so `evaluate` can resolve a `topicID` to its full topic content, and
//! `evaluate` already takes `topics` directly here. `make_rules` needs no
//! catalog/topics input at all.
//!
//! **The `analyze(symptomIDs: Set<SymptomID>, ...)` overload is dropped.**
//! It's a one-line wrapper — `analyze(symptoms: Array(symptomIDs), ...)` —
//! and a Swift `Set`'s iteration order is already unspecified, so the one
//! Rust entry point (`analyze`, taking an ordered slice) is equivalent
//! modulo an ordering Swift itself never guaranteed.
//!
//! **Two branches are structurally unreachable against MSC 1's real rule
//! table and topic catalog, both confirmed empirically (not by inspection
//! alone) via the same Swift-harness methodology P1.10–P1.12 used:**
//!
//! - `evaluate`'s `excludedSymptoms` short-circuit: every one of the 9 real
//!   rules declares `excludedSymptoms: []`. Exercised via a synthetic 10th
//!   rule built directly in `router_troubleshooting.rs`'s test file (not
//!   literal MSC 1 content), the same pattern P1.12 used for its synthetic
//!   guide.
//! - `evaluate`'s "rule's topic isn't in the repository" `nil` return: the
//!   real 9-rule table and 9-topic catalog are exactly 1:1, so every real
//!   rule's `topicID` always resolves. Exercised via a second, deliberately
//!   incomplete topics list built in the test file, the same pattern P1.11
//!   used for its no-mesh catalog.

use crate::router::composer::{TroubleshootingTopic, TroubleshootingTopicId};
use crate::router::fallback_tree::{self, FallbackResolution, FallbackState};
use crate::router::matcher::Guide;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymptomId {
    CannotConnectExternally,
    LocalNetworkWorksButInternetFails,
    RouterRulePointsToOldIp,
    MacIpAddressChanged,
    SelectedWrongTargetDevice,
    ForwardedOnProviderButOwnRouterExists,
    ForwardedOnOwnRouterButProviderGatewayExists,
    TwoRoutersPresent,
    WanIpDiffersFromPublicIp,
    ApartmentDormManagedNetwork,
    NoRouterAdminAccess,
    JavaWorksBedrockFails,
    BedrockWorksJavaFails,
    FirewallPromptSeen,
    SecurityToolMayBeBlocking,
    ChangesSavedButStillFails,
    RouterAskedToReboot,
    UsingMeshBridgeOrApMode,
}

impl SymptomId {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::CannotConnectExternally => "cannot_connect_externally",
            Self::LocalNetworkWorksButInternetFails => "local_network_works_but_internet_fails",
            Self::RouterRulePointsToOldIp => "router_rule_points_to_old_ip",
            Self::MacIpAddressChanged => "mac_ip_address_changed",
            Self::SelectedWrongTargetDevice => "selected_wrong_target_device",
            Self::ForwardedOnProviderButOwnRouterExists => {
                "forwarded_on_provider_but_own_router_exists"
            }
            Self::ForwardedOnOwnRouterButProviderGatewayExists => {
                "forwarded_on_own_router_but_provider_gateway_exists"
            }
            Self::TwoRoutersPresent => "two_routers_present",
            Self::WanIpDiffersFromPublicIp => "wan_ip_differs_from_public_ip",
            Self::ApartmentDormManagedNetwork => "apartment_dorm_managed_network",
            Self::NoRouterAdminAccess => "no_router_admin_access",
            Self::JavaWorksBedrockFails => "java_works_bedrock_fails",
            Self::BedrockWorksJavaFails => "bedrock_works_java_fails",
            Self::FirewallPromptSeen => "firewall_prompt_seen",
            Self::SecurityToolMayBeBlocking => "security_tool_may_be_blocking",
            Self::ChangesSavedButStillFails => "changes_saved_but_still_fails",
            Self::RouterAskedToReboot => "router_asked_to_reboot",
            Self::UsingMeshBridgeOrApMode => "using_mesh_bridge_or_ap_mode",
        }
    }

    pub fn from_raw_value(raw: &str) -> Option<Self> {
        Some(match raw {
            "cannot_connect_externally" => Self::CannotConnectExternally,
            "local_network_works_but_internet_fails" => Self::LocalNetworkWorksButInternetFails,
            "router_rule_points_to_old_ip" => Self::RouterRulePointsToOldIp,
            "mac_ip_address_changed" => Self::MacIpAddressChanged,
            "selected_wrong_target_device" => Self::SelectedWrongTargetDevice,
            "forwarded_on_provider_but_own_router_exists" => {
                Self::ForwardedOnProviderButOwnRouterExists
            }
            "forwarded_on_own_router_but_provider_gateway_exists" => {
                Self::ForwardedOnOwnRouterButProviderGatewayExists
            }
            "two_routers_present" => Self::TwoRoutersPresent,
            "wan_ip_differs_from_public_ip" => Self::WanIpDiffersFromPublicIp,
            "apartment_dorm_managed_network" => Self::ApartmentDormManagedNetwork,
            "no_router_admin_access" => Self::NoRouterAdminAccess,
            "java_works_bedrock_fails" => Self::JavaWorksBedrockFails,
            "bedrock_works_java_fails" => Self::BedrockWorksJavaFails,
            "firewall_prompt_seen" => Self::FirewallPromptSeen,
            "security_tool_may_be_blocking" => Self::SecurityToolMayBeBlocking,
            "changes_saved_but_still_fails" => Self::ChangesSavedButStillFails,
            "router_asked_to_reboot" => Self::RouterAskedToReboot,
            "using_mesh_bridge_or_ap_mode" => Self::UsingMeshBridgeOrApMode,
            _ => return None,
        })
    }
}

/// Swift declares no explicit `rawValue`, so the implicit raw value is the
/// (already-lowercase, single-word) case name itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CauseConfidence {
    Strong,
    Possible,
}

impl CauseConfidence {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::Strong => "strong",
            Self::Possible => "possible",
        }
    }
}

/// Also an implicit raw value — see [`CauseConfidence::raw_value`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cause<'a> {
    pub id: TroubleshootingTopicId,
    pub confidence: CauseConfidence,
    pub score: i32,
    pub matched_symptoms: Vec<SymptomId>,
    pub topic: &'a TroubleshootingTopic,
}

impl Cause<'_> {
    pub fn severity(&self) -> Severity {
        match self.score {
            9.. => Severity::High,
            5..=8 => Severity::Medium,
            _ => Severity::Low,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Requirement {
    pub symptom: SymptomId,
    pub weight: i32,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub topic_id: TroubleshootingTopicId,
    pub title: &'static str,
    pub all_of: Vec<Requirement>,
    pub any_of: Vec<Requirement>,
    pub excluded_symptoms: HashSet<SymptomId>,
    pub explanation: &'static str,
    pub next_actions: Vec<&'static str>,
    pub escalation_bullets: Vec<&'static str>,
}

#[derive(Debug, Clone)]
pub struct Report<'a> {
    pub symptoms: Vec<SymptomId>,
    pub likely_causes: Vec<Cause<'a>>,
    pub recommended_actions: Vec<String>,
    pub escalation_bullets: Vec<String>,
    pub fallback_resolution: Option<FallbackResolution<'a>>,
    pub summary: String,
}

pub fn make_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "local-ip-changed",
            topic_id: TroubleshootingTopicId::LocalIpChanged,
            title: "Local IP changed or rule points to old IP",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::RouterRulePointsToOldIp,
                    weight: 4,
                },
                Requirement {
                    symptom: SymptomId::MacIpAddressChanged,
                    weight: 4,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "A stale local IP is one of the most common reasons a previously working port forward stops working.",
            next_actions: vec![
                "Re-check the Mac's current local IP in Minecraft Server Controller.",
                "Update the router rule so it points to the current host Mac address.",
                "Reserve the Mac's DHCP address later so it stops changing.",
            ],
            escalation_bullets: vec![
                "If the router UI lists multiple copies of the same device, delete stale target entries before re-testing.",
            ],
        },
        Rule {
            id: "wrong-device",
            topic_id: TroubleshootingTopicId::WrongDevice,
            title: "Wrong target device selected",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::SelectedWrongTargetDevice,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::BedrockWorksJavaFails,
                    weight: 2,
                },
                Requirement {
                    symptom: SymptomId::JavaWorksBedrockFails,
                    weight: 2,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "A correct port number still fails if the rule targets the wrong machine on the LAN.",
            next_actions: vec![
                "Confirm the selected target device is the Mac running Minecraft Server Controller.",
                "Match the target device entry to the local IP shown in the app.",
                "Remove duplicate rules that still point to older devices.",
            ],
            escalation_bullets: vec![],
        },
        Rule {
            id: "wrong-router",
            topic_id: TroubleshootingTopicId::WrongRouter,
            title: "Wrong router configured",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::ForwardedOnProviderButOwnRouterExists,
                    weight: 4,
                },
                Requirement {
                    symptom: SymptomId::ForwardedOnOwnRouterButProviderGatewayExists,
                    weight: 4,
                },
                Requirement {
                    symptom: SymptomId::UsingMeshBridgeOrApMode,
                    weight: 3,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "The provider name is not always the device that actually holds the routing table.",
            next_actions: vec![
                "Identify which device is assigning LAN IP addresses and acting as the main router.",
                "If your mesh or own router is in bridge / AP mode, make the rule on the upstream router instead.",
                "If your ISP device is bridged, configure forwarding only on your own router.",
            ],
            escalation_bullets: vec![
                "When in doubt, compare the LAN IP ranges and DHCP clients shown by each device.",
            ],
        },
        Rule {
            id: "double-nat",
            topic_id: TroubleshootingTopicId::DoubleNat,
            title: "Double NAT or multiple routers",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::TwoRoutersPresent,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::ForwardedOnProviderButOwnRouterExists,
                    weight: 3,
                },
                Requirement {
                    symptom: SymptomId::ForwardedOnOwnRouterButProviderGatewayExists,
                    weight: 3,
                },
                Requirement {
                    symptom: SymptomId::UsingMeshBridgeOrApMode,
                    weight: 2,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "Two routing layers often mean forwarding was only done on one layer, so the connection still cannot reach the host Mac.",
            next_actions: vec![
                "Check whether both the ISP device and another router or mesh system are routing.",
                "Put one device into bridge / passthrough mode when possible.",
                "Otherwise forward on the upstream router as well.",
            ],
            escalation_bullets: vec![
                "If the WAN IP of your own router is private, there is definitely another upstream routing layer.",
            ],
        },
        Rule {
            id: "cgnat",
            topic_id: TroubleshootingTopicId::Cgnat,
            title: "Carrier-grade NAT or blocked public IPv4",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::WanIpDiffersFromPublicIp,
                    weight: 6,
                },
                Requirement {
                    symptom: SymptomId::ApartmentDormManagedNetwork,
                    weight: 3,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "If your router never receives a real public IPv4 address, standard inbound forwarding may never work.",
            next_actions: vec![
                "Compare the WAN IP on the router to the public IP seen on the internet.",
                "Ask the ISP whether they offer a real public IP or bridgeable modem mode.",
                "Use an alternative remote-access method if your network environment blocks direct forwarding.",
            ],
            escalation_bullets: vec![],
        },
        Rule {
            id: "firewall-blocked",
            topic_id: TroubleshootingTopicId::FirewallBlocked,
            title: "Host firewall or security tool blocking traffic",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::FirewallPromptSeen,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::SecurityToolMayBeBlocking,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::LocalNetworkWorksButInternetFails,
                    weight: 2,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "A correct router rule still fails if the host Mac or another security layer rejects the traffic locally.",
            next_actions: vec![
                "Verify the server is running and reachable on your LAN first.",
                "Review macOS firewall permissions and any third-party security tools.",
                "Re-test externally after confirming the host is accepting local traffic.",
            ],
            escalation_bullets: vec![],
        },
        Rule {
            id: "wrong-protocol",
            topic_id: TroubleshootingTopicId::WrongProtocol,
            title: "Wrong protocol or wrong service rule",
            all_of: vec![Requirement {
                symptom: SymptomId::CannotConnectExternally,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::JavaWorksBedrockFails,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::BedrockWorksJavaFails,
                    weight: 5,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "Java and Bedrock often need different protocol choices and sometimes separate rules.",
            next_actions: vec![
                "Use TCP for Java unless your specific setup says otherwise.",
                "Use UDP for Bedrock unless your specific setup says otherwise.",
                "If both Java and Bedrock need to work, verify that both rules exist and point to the same host Mac.",
            ],
            escalation_bullets: vec![],
        },
        Rule {
            id: "router-reboot",
            topic_id: TroubleshootingTopicId::RouterRebootRequired,
            title: "Router has not applied the rule yet",
            all_of: vec![Requirement {
                symptom: SymptomId::ChangesSavedButStillFails,
                weight: 3,
            }],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::RouterAskedToReboot,
                    weight: 5,
                },
                Requirement {
                    symptom: SymptomId::CannotConnectExternally,
                    weight: 1,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "Some firmware does not activate the new forwarding rule immediately after you edit it.",
            next_actions: vec![
                "Press Save or Apply in the forwarding page.",
                "If the router requested a reboot, complete it and wait for the network to come back.",
                "Run the external test again after the router is fully online.",
            ],
            escalation_bullets: vec![],
        },
        Rule {
            id: "no-admin-access",
            topic_id: TroubleshootingTopicId::NoAdminAccess,
            title: "Admin access or network policy blocks forwarding",
            all_of: vec![],
            any_of: vec![
                Requirement {
                    symptom: SymptomId::NoRouterAdminAccess,
                    weight: 6,
                },
                Requirement {
                    symptom: SymptomId::ApartmentDormManagedNetwork,
                    weight: 5,
                },
            ],
            excluded_symptoms: HashSet::new(),
            explanation: "Some networks simply do not permit user-managed forwarding.",
            next_actions: vec![
                "Check whether you have the router admin credentials.",
                "Ask the ISP, property manager, or network owner whether inbound forwarding is allowed.",
                "Use another remote-access path when the network policy blocks direct inbound rules.",
            ],
            escalation_bullets: vec![
                "This is especially common on apartment, dorm, campus, and shared-building networks.",
            ],
        },
    ]
}

fn normalized(symptoms: &[SymptomId]) -> Vec<SymptomId> {
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for &symptom in symptoms {
        if seen.insert(symptom) {
            ordered.push(symptom);
        }
    }
    ordered
}

fn deduplicated_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.clone()) {
            result.push(trimmed);
        }
    }
    result
}

fn evaluate<'a>(
    rule: &Rule,
    symptoms: &[SymptomId],
    topics: &'a [TroubleshootingTopic],
) -> Option<Cause<'a>> {
    let symptom_set: HashSet<SymptomId> = symptoms.iter().copied().collect();

    if rule
        .excluded_symptoms
        .intersection(&symptom_set)
        .next()
        .is_some()
    {
        return None;
    }

    if !rule.all_of.iter().all(|r| symptom_set.contains(&r.symptom)) {
        return None;
    }

    let any_matches: Vec<&Requirement> = rule
        .any_of
        .iter()
        .filter(|r| symptom_set.contains(&r.symptom))
        .collect();
    if !rule.any_of.is_empty() && any_matches.is_empty() {
        return None;
    }

    let topic = topics.iter().find(|t| t.id == rule.topic_id)?;

    let required_score: i32 = rule.all_of.iter().map(|r| r.weight).sum();
    let optional_score: i32 = any_matches.iter().map(|r| r.weight).sum();
    let score = required_score + optional_score;

    let matched_symptoms: Vec<SymptomId> = rule
        .all_of
        .iter()
        .map(|r| r.symptom)
        .chain(any_matches.iter().map(|r| r.symptom))
        .collect();

    let confidence =
        if rule.any_of.is_empty() || any_matches.len() >= std::cmp::max(1, rule.any_of.len() / 2) {
            CauseConfidence::Strong
        } else {
            CauseConfidence::Possible
        };

    Some(Cause {
        id: rule.topic_id,
        confidence,
        score,
        matched_symptoms,
        topic,
    })
}

fn make_summary(
    likely_causes: &[Cause],
    fallback_resolution: Option<&FallbackResolution>,
) -> String {
    if let Some(first) = likely_causes.first() {
        if likely_causes.len() == 1 {
            return format!("Most likely cause: {}.", first.topic.title);
        }
        let remaining: Vec<&str> = likely_causes[1..]
            .iter()
            .map(|c| c.topic.title.as_str())
            .collect();
        return format!(
            "Most likely cause: {}. Also check: {}.",
            first.topic.title,
            remaining.join(", ")
        );
    }

    if let Some(resolution) = fallback_resolution
        && matches!(
            resolution.kind,
            fallback_tree::ResolutionKind::UnknownRouterHelp
                | fallback_tree::ResolutionKind::NeedsMoreInfo
        )
    {
        return "No strong failure cause was detected yet. Identify the correct router path first, then re-run troubleshooting with more specific symptoms.".to_string();
    }

    "No strong failure cause was detected yet. Use the generic troubleshooting path and verify the router, target device, IP, port, and protocol.".to_string()
}

pub fn analyze<'a>(
    symptoms: &[SymptomId],
    rules: &[Rule],
    topics: &'a [TroubleshootingTopic],
    catalog: &'a [Guide],
    fallback_state: Option<&FallbackState>,
    detected_gateway_ip_address: Option<&str>,
) -> Report<'a> {
    let normalized_symptoms = normalized(symptoms);
    let fallback_resolution = fallback_state
        .map(|state| fallback_tree::resolve(state, catalog, detected_gateway_ip_address));

    let mut likely_causes: Vec<Cause> = rules
        .iter()
        .filter_map(|rule| evaluate(rule, &normalized_symptoms, topics))
        .collect();
    likely_causes.sort_by(|lhs, rhs| {
        if lhs.score != rhs.score {
            return rhs.score.cmp(&lhs.score);
        }
        if lhs.confidence != rhs.confidence {
            return if lhs.confidence == CauseConfidence::Strong {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        lhs.topic.title.cmp(&rhs.topic.title)
    });

    let recommended_actions = deduplicated_strings(
        likely_causes
            .iter()
            .flat_map(|c| c.topic.suggested_next_actions.iter().cloned())
            .chain(likely_causes.iter().flat_map(|cause| {
                rules
                    .iter()
                    .find(|r| r.topic_id == cause.id)
                    .map(|r| {
                        r.next_actions
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            }))
            .collect(),
    );

    let mut escalation_bullets = deduplicated_strings(
        likely_causes
            .iter()
            .flat_map(|cause| {
                rules
                    .iter()
                    .find(|r| r.topic_id == cause.id)
                    .map(|r| {
                        r.escalation_bullets
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect(),
    );

    if let Some(resolution) = &fallback_resolution {
        escalation_bullets = deduplicated_strings(
            escalation_bullets
                .into_iter()
                .chain(resolution.explanation_bullets.iter().cloned())
                .collect(),
        );
    }

    let summary = make_summary(&likely_causes, fallback_resolution.as_ref());

    Report {
        symptoms: normalized_symptoms,
        likely_causes,
        recommended_actions,
        escalation_bullets,
        fallback_resolution,
        summary,
    }
}
