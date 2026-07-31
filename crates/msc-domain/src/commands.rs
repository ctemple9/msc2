//! The Minecraft command catalog and its autocomplete suggestion engine.
//!
//! Ported from `MinecraftCommandRegistry.swift` (542 lines, 42 command
//! definitions). No MSC 1 test file exists for this type —
//! `fixtures/command-catalog/` characterizes it fresh by reading the
//! source's closed, deterministic logic directly, per `rolling-plan.md`
//! P1.9's own instructions. `description`, `category` (and its `icon`/
//! `color`), and each argument slot's `label` are client-rendering (port
//! plan §1's deletion test) and are not ported — none of them affect
//! `commands_for` or `suggestions`, the two functions this step
//! characterizes.

use crate::identity::ServerType;

/// What kind of value an argument slot expects, reduced to only what the
/// suggestion engine branches on. Swift's `CommandArgSlot` also carries
/// `.coordinates`/`.integer`/`.freeText` as distinct cases, but
/// `suggestions` treats all three identically (its `default: return []`
/// arm) — collapsed here into `Other` rather than ported as separate
/// cases with no distinct behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgSlotKind {
    PlayerName,
    Keyword(&'static [&'static str]),
    Other,
}

pub struct CommandDef {
    pub name: &'static str,
    pub arg_slots: &'static [ArgSlotKind],
    pub supports_java: bool,
    pub supports_bedrock: bool,
}

/// A currently connected player. MSC 1's `OnlinePlayer` also carries an
/// optional `xuid` (Bedrock's player identifier) used only for `id` —
/// client-list identity, not suggestion-engine logic — so it's not ported.
pub struct OnlinePlayer {
    pub name: String,
}

// Declaration order from `MinecraftCommandRegistry.swift`'s `all` array —
// `commands_for`'s filter and `suggestions`'s command-name-prefix match
// both preserve this order, so it must match source exactly.
pub static ALL: &[CommandDef] = &[
    CommandDef {
        name: "tp",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "teleport",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "give",
        arg_slots: &[
            ArgSlotKind::PlayerName,
            ArgSlotKind::Other,
            ArgSlotKind::Other,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "kick",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "ban",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "ban-ip",
        arg_slots: &[ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "pardon",
        arg_slots: &[ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "op",
        arg_slots: &[ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "deop",
        arg_slots: &[ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "msg",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "tell",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "kill",
        arg_slots: &[ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "gamemode",
        arg_slots: &[
            ArgSlotKind::Keyword(&["survival", "creative", "adventure", "spectator"]),
            ArgSlotKind::PlayerName,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "effect",
        arg_slots: &[
            ArgSlotKind::PlayerName,
            ArgSlotKind::Other,
            ArgSlotKind::Other,
            ArgSlotKind::Other,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "xp",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::PlayerName],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "experience",
        arg_slots: &[
            ArgSlotKind::Keyword(&["add", "set", "query"]),
            ArgSlotKind::PlayerName,
            ArgSlotKind::Other,
            ArgSlotKind::Keyword(&["points", "levels"]),
        ],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "clear",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "time",
        arg_slots: &[
            ArgSlotKind::Keyword(&["set", "add", "query"]),
            ArgSlotKind::Other,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "weather",
        arg_slots: &[ArgSlotKind::Keyword(&["clear", "rain", "thunder"])],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "difficulty",
        arg_slots: &[ArgSlotKind::Keyword(&[
            "peaceful", "easy", "normal", "hard",
        ])],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "gamerule",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "setworldspawn",
        arg_slots: &[ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "spawnpoint",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "list",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "seed",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "say",
        arg_slots: &[ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "title",
        arg_slots: &[
            ArgSlotKind::PlayerName,
            ArgSlotKind::Keyword(&["title", "subtitle", "actionbar", "clear", "reset"]),
            ArgSlotKind::Other,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "save-all",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "save-off",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "save-on",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "reload",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "stop",
        arg_slots: &[],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "whitelist",
        arg_slots: &[
            ArgSlotKind::Keyword(&["on", "off", "add", "remove", "list", "reload"]),
            ArgSlotKind::PlayerName,
        ],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "allowlist",
        arg_slots: &[
            ArgSlotKind::Keyword(&["on", "off", "add", "remove", "list", "reload"]),
            ArgSlotKind::PlayerName,
        ],
        supports_java: false,
        supports_bedrock: true,
    },
    CommandDef {
        name: "banlist",
        arg_slots: &[ArgSlotKind::Keyword(&["players", "ips"])],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "enchant",
        arg_slots: &[
            ArgSlotKind::PlayerName,
            ArgSlotKind::Other,
            ArgSlotKind::Other,
        ],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "attribute",
        arg_slots: &[ArgSlotKind::PlayerName, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: false,
    },
    CommandDef {
        name: "setblock",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "fill",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "clone",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "summon",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: true,
    },
    CommandDef {
        name: "particle",
        arg_slots: &[ArgSlotKind::Other, ArgSlotKind::Other],
        supports_java: true,
        supports_bedrock: false,
    },
];

/// The catalog filtered to commands available for `server_type`, in
/// catalog declaration order.
pub fn commands_for(server_type: ServerType) -> Vec<&'static CommandDef> {
    ALL.iter()
        .filter(|c| match server_type {
            ServerType::Java => c.supports_java,
            ServerType::Bedrock => c.supports_bedrock,
        })
        .collect()
}

/// Up to 6 suggested completions for the current command-line `input`.
/// Typing a command-name prefix suggests matching command names; typing
/// inside an argument position suggests online players (`playerName`
/// slots) or fixed keywords (`keyword` slots) — coordinates/integer/
/// free-text slots never suggest anything.
///
/// Ported verbatim, off-by-one and all: `input.split(' ')` (Rust's
/// default, matching Swift's `omittingEmptySubsequences: false`) leaves a
/// trailing empty token after a trailing space, which shifts
/// `slot_index` one past where a reader would expect right after typing
/// "`cmd `" — see `fixtures/command-catalog/suggestions-space-after-*`
/// for the three ways that plays out (out of range on a 1-slot command,
/// landing on the *next* slot on a 2-slot command, out of range past a
/// 2-slot command). Confirmed against a standalone copy of the literal
/// Swift source run through `swift`, not hand-derived, because this
/// off-by-one is easy to get wrong by inspection alone.
pub fn suggestions(
    input: &str,
    server_type: ServerType,
    online_players: &[OnlinePlayer],
) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let tokens: Vec<&str> = input.split(' ').collect();
    let ends_with_space = input.ends_with(' ');
    let available = commands_for(server_type);

    if tokens.len() == 1 && !ends_with_space {
        let raw = tokens[0];
        let prefix = raw.strip_prefix('/').unwrap_or(raw);
        if prefix.is_empty() {
            return Vec::new();
        }
        let prefix_lower = prefix.to_lowercase();
        return available
            .iter()
            .filter(|c| c.name.starts_with(prefix_lower.as_str()))
            .take(6)
            .map(|c| format!("/{}", c.name))
            .collect();
    }

    let raw_command = tokens[0];
    let command_name = raw_command.strip_prefix('/').unwrap_or(raw_command);
    let Some(def) = available.iter().find(|c| c.name == command_name) else {
        return Vec::new();
    };
    if def.arg_slots.is_empty() {
        return Vec::new();
    }

    let filled_tokens = &tokens[1..];
    let slot_index = if ends_with_space {
        filled_tokens.len()
    } else {
        filled_tokens.len().saturating_sub(1)
    };
    if slot_index >= def.arg_slots.len() {
        return Vec::new();
    }

    let slot = def.arg_slots[slot_index];
    let partial_arg = if ends_with_space {
        ""
    } else {
        filled_tokens.last().copied().unwrap_or("")
    };
    let partial = partial_arg.to_lowercase();

    let base_tokens: &[&str] = if ends_with_space {
        &tokens
    } else {
        &tokens[..tokens.len() - 1]
    };
    let base = base_tokens.join(" ");

    match slot {
        ArgSlotKind::PlayerName => online_players
            .iter()
            .filter(|p| partial.is_empty() || p.name.to_lowercase().starts_with(partial.as_str()))
            .take(6)
            .map(|p| format!("{base} {}", p.name))
            .collect(),
        ArgSlotKind::Keyword(options) => options
            .iter()
            .filter(|o| partial.is_empty() || o.to_lowercase().starts_with(partial.as_str()))
            .take(6)
            .map(|o| format!("{base} {o}"))
            .collect(),
        ArgSlotKind::Other => Vec::new(),
    }
}
