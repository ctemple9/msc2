//! Versioned text fallback. Illustrated launcher-specific guides are supplied
//! in P24.20; these files make that limitation explicit without remote assets.
use crate::content_publications::Publication;
pub const VERSION: &str = "text-v2";
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
    vec![Asset {path:"PLAYER.txt".into(),bytes:player.into_bytes()},Asset {path:"SERVER.txt".into(),bytes:server.into_bytes()},Asset {path:"GUIDE-STATUS.txt".into(),bytes:b"Text fallback guide version text-v2. Launcher-specific illustrated instructions and owner-provided screenshots are not included in this development guide version. Read the publication manifest for missing content and exact setup. No network assets are required to read these files.\n".to_vec()}]
}
