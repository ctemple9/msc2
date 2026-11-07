//! Versioned text fallback. Illustrated launcher-specific guides are supplied
//! in P24.20; these files make that limitation explicit without remote assets.
use crate::content_publications::Publication;
pub const VERSION: &str = "text-v4-pack-exports";
pub struct Asset {
    pub path: String,
    pub bytes: Vec<u8>,
}
pub fn assets(publication: &Publication) -> Vec<Asset> {
    let setup = format!(
        "Minecraft: {}\nLoader: {}\nLoader version: {}\n",
        publication
            .minecraft_version
            .as_deref()
            .unwrap_or("UNKNOWN — resolve before playing"),
        publication.loader,
        publication
            .loader_version
            .as_deref()
            .unwrap_or("not recorded")
    );
    let player_setup = crate::content_environment::for_publication(publication);
    let player_header = format!(
        "Minecraft: {}\nPlayer loader: {}\nPlayer loader version: {}\n",
        publication
            .minecraft_version
            .as_deref()
            .unwrap_or("UNKNOWN — resolve before playing"),
        player_setup
            .as_ref()
            .map_or("UNKNOWN — review player setup", |setup| setup
                .loader
                .as_str()),
        player_setup
            .as_ref()
            .and_then(|setup| setup.loader_version.as_deref())
            .unwrap_or("not required or not recorded")
    );
    let mut player = format!(
        "PLAYER CONTENT\n\n{player_header}\nCreate a separate instance in your compatible launcher with the exact Minecraft and loader versions above. Open that instance's Minecraft folder. Copy the exported mods, config, scripts, kubejs, defaultconfigs, resourcepacks and shaderpacks folders into their matching locations, preserving subfolders.\n\nUse a clean instance for updates so old mod versions do not remain alongside new ones. Back up personal settings before replacing configuration. Compare the manifest with the files you copied before joining the server.\n\nOwner-selected optional content is included when eligible. Leave optional shader/resource packs inactive until you choose to enable them in Minecraft. A launcher may not automatically disable optional mod JARs; check the manifest and choose which optional mods to use.\n\n"
    );
    player.push_str("Datapacks under datapacks/<world-slot-id>/ belong to the source world. They are included for completeness; joining this server does not require installing them in your launcher. To use them in a separate local world, copy the contents of the chosen slot group into that world's datapacks folder. Do not copy the slot ID as an extra subfolder. This bundle contains no world terrain or player saves.\n\n");
    for file in publication.player_files.values() {
        player.push_str(&format!(
            "{}{}{}\n",
            file.path,
            if file.optional {
                " [optional]"
            } else {
                " [required]"
            },
            if file.enabled {
                ""
            } else {
                " [inactive selection]"
            }
        ));
    }
    let mut server = format!(
        "SERVER CONTENT\n\n{setup}\nStop the target server and back up its current content before applying this bundle. Match the Minecraft and loader versions above. Copy only the provided content folders into the matching server folders. Remove obsolete mod versions deliberately; do not combine old and new JARs blindly.\n\nDatapacks are grouped under datapacks/<world-slot-id>/. For each group, choose the intended world's datapacks folder and copy the contents of that slot group there, preserving each datapack's internal folders. The slot ID identifies the source world; it is not an extra folder to install inside the target world. This export does not copy worlds, terrain, players, or world activation settings. Review/enable datapacks on the target server as appropriate.\n\n"
    );
    for file in publication.server_files.values() {
        server.push_str(&format!("{}\n", file.path));
    }
    for issue in &publication.issues {
        let line = format!(
            "MISSING/UNRESOLVED: {} — {}\n",
            issue.path.as_deref().unwrap_or("publication setup"),
            issue.reason
        );
        if matches!(issue.audience.as_str(), "players" | "all") {
            player.push_str(&line);
        }
        if matches!(issue.audience.as_str(), "server" | "all") {
            server.push_str(&line);
        }
    }
    let mut result = vec![
        Asset {path:"PLAYER.txt".into(),bytes:player.into_bytes()},
        Asset {path:"SERVER.txt".into(),bytes:server.into_bytes()},
        Asset {path:"GUIDE-STATUS.txt".into(),bytes:format!("Guide version {VERSION}. Offline launcher-specific text and HTML drafts are included. Owner screenshots and exact installed launcher-build verification are still pending; this version is not an illustrated guide. No remote assets are required to read these instructions. Read the publication manifest for exact setup and missing content.\n").into_bytes()},
    ];
    let guides = [
        (
            "minecraft-launcher",
            "Minecraft Launcher",
            include_str!("../../../docs/msc2/addons/launcher-guides/minecraft-launcher.txt"),
        ),
        (
            "prism",
            "Prism Launcher",
            include_str!("../../../docs/msc2/addons/launcher-guides/prism.txt"),
        ),
        (
            "modrinth-app",
            "Modrinth App",
            include_str!("../../../docs/msc2/addons/launcher-guides/modrinth-app.txt"),
        ),
        (
            "curseforge-app",
            "CurseForge App",
            include_str!("../../../docs/msc2/addons/launcher-guides/curseforge-app.txt"),
        ),
        (
            "sklauncher",
            "SKlauncher",
            include_str!("../../../docs/msc2/addons/launcher-guides/sklauncher.txt"),
        ),
    ];
    let mut index = format!(
        "<p>Guide version {VERSION}. Text drafts; owner screenshots and exact launcher-build verification are pending.</p><p>Read the <a href=\"../README.html\">publication instructions</a> first for exact setup, content, optional files and missing items.</p><ul>"
    );
    for (name, title, text) in guides {
        result.push(Asset {
            path: format!("{name}.txt"),
            bytes: text.as_bytes().to_vec(),
        });
        result.push(Asset {
            path: format!("{name}.html"),
            bytes: html(
                title,
                &format!(
                    "<p><a href=\"INDEX.html\">All launchers</a></p><pre>{}</pre>",
                    escape(text)
                ),
            )
            .into_bytes(),
        });
        index.push_str(&format!(
            "<li><a href=\"{name}.html\">{title}</a> (<a href=\"{name}.txt\">text</a>)</li>"
        ));
    }
    index.push_str("</ul><p>The ordinary MSC content ZIP is a folder-copy bundle. Only a separately included .mrpack is offered for compatible pack import. Installation can require network downloads even though these instructions are readable offline.</p>");
    result.push(Asset {
        path: "INDEX.html".into(),
        bytes: html("Launcher instructions", &index).into_bytes(),
    });
    result
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn html(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'; img-src 'self' data:; base-uri 'none'; form-action 'none'\"><title>{}</title><style>body{{font:16px/1.6 system-ui,sans-serif;max-width:54rem;margin:2rem auto;padding:0 1.25rem;color:#222;background:#fff}}h1{{font-size:1.6rem}}pre{{font:inherit;white-space:pre-wrap;overflow-wrap:anywhere}}a{{color:#164c91}}</style></head><body><h1>{}</h1>{body}</body></html>",
        escape(title),
        escape(title)
    )
}
