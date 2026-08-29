---
id: handbook.server-files
kind: handbook
title: Server Files Browser
category: server-management
subtitle: "Browse, preview, and edit your server's files without leaving the app."
analogy: "Your server directory is like a filing cabinet. MSC's Files tab is a window into that cabinet — you can browse every drawer, read any document, and carefully edit the ones you need to change."
relatedIds: [handbook.worlds-backups, health.directory]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: serverFilesContent}
---

The **Files tab** gives you a full view of everything on disk inside your server's directory. For Java servers this includes your Paper JAR, plugins folder, world data, and all config files. For Bedrock servers it shows the server directory shared with the VM — BDS binary, worlds folder, config files, and more.

You can navigate into any subfolder and get back with the breadcrumb trail at the top.

### In This App

- Click any folder to navigate into it — breadcrumbs at the top let you jump back
- Text files (.yml, .json, .properties, .log, .txt, .sh, .cfg, .conf) can be previewed in-app with a click
- The preview sheet has an "Edit File" button — tap it, confirm the warning, and the file becomes editable
- Changes are written directly to disk when you click Save — there is no undo inside MSC
- Non-text files (JARs, ZIPs, images) open via "Reveal in Finder" instead
- "Show in Finder" in the breadcrumb bar opens the current folder in Finder

### Callout: warning

Editing server files directly can break your server if you make a mistake. Always stop the server before editing critical config files like server.properties or paper.yml. When in doubt, make a backup first.

### Callout: tip

The most commonly edited files are server.properties (core settings), ops.json (operator list), whitelist.json (allowlist), and plugin config YAMLs inside the plugins/ folder.

### Advanced Details

For Bedrock servers, all files shown are on your Mac's filesystem (your server's host directory). The VM accesses this directory via a direct file share — no container layer. Edits you make here while the server is stopped take effect the next time the server starts.

For Java (Paper) servers, files are read and written directly — no container layer involved. The server process and MSC both access the same directory.

Neither server type locks individual files while running (except active world region files). Editing a config while the server is live won't cause a crash, but the change won't take effect until the server is reloaded or restarted.
