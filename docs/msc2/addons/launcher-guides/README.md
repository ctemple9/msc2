# Offline launcher guide preparation

P24.20 is **prepared, incomplete**. Owner screenshots and exact installed launcher
build verification are absent. These text drafts can ship as a clearly labelled
development fallback; they do not satisfy the illustrated-guide gate. Exported
files contain no remote assets or invitation links. Sources below are developer
research, not a network dependency of the exported guide.

Research checked 2026-11-06:

| Launcher | Evidence and current limit |
|---|---|
| Minecraft Launcher | [Official installation instructions](https://help.minecraft.net/hc/en-us/articles/23431114561037-Troubleshooting-Launching-Minecraft-from-the-Minecraft-Launcher) and [folder control](https://help.minecraft.net/hc/en-us/articles/4409159214605-Minecraft-Java-Edition-Installation-Issues-FAQ). Manual content route; no direct pack import claimed. Owner must verify exact build, loader installer and separate directory controls. |
| Prism | [Official import documentation](https://prismlauncher.org/wiki/help-pages/Zip-import/) and [instance creation](https://prismlauncher.org/wiki/getting-started/create-instance/). `.mrpack` import supported; generic content ZIP is not an instance archive. Exact build/screenshots pending. |
| Modrinth App | [Official app announcement](https://modrinth.com/news/article/modrinth-app-beta/) documents local `.mrpack` import; [current changelog](https://modrinth.com/news/changelog?filter=app) continues to describe that path. Exact current control labels/build need owner screenshots; no guessed button labels are frozen as verified. |
| CurseForge App | [Official profile import rules](https://support.curseforge.com/support/solutions/articles/9000198501-exporting-modpacks) require its own ZIP structure. Manual profile-copy route for MSC ZIP; direct `.mrpack` import not claimed. Exact build/custom-profile controls pending. |
| SKlauncher | [Stable 3.2 Fabric setup](https://docs.skmedix.pl/modding/mod-loaders/fabric), [4.0 beta scope](https://docs.skmedix.pl/4.0/). Keep versions distinct; no assumed 3.2 `.mrpack` support. Owner must verify chosen build and other loader selectors. |

For each launcher supply original screenshots of: version/build, creation/import
control, exact Minecraft/loader selection, isolated game-directory/open-folder
control, and resulting content/optional-pack selection. Hide account identifiers
and private paths before providing images. Record image filename, visible action,
launcher build, OS, alt text and owner approval. Never fabricate screenshots or
use third-party documentation screenshots as Cameron's supplied assets.

Integrate approved screenshots as bundled byte assets with local-only HTML image
references. Replace the missing-image status, verify text/HTML and image references
offline, then increment the guide version before making new publications. Existing
publications keep their frozen guide bytes/version. A compatible launcher may need
internet to install Minecraft/loaders or resolve `.mrpack` downloads; offline guide
readability is separate from a promise of offline game installation.
