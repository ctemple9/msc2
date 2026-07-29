# MSC 2 Exact Audit Diff

**Inputs:** `msc2-codex-file-inventory.csv` and `msc2-claude-file-inventory.csv`  
**Files compared:** 246  
**Date:** July 29, 2026

## Result

After normalizing bucket names:

- **218 files agree**
- **28 files disagree**
- **Exact bucket agreement: 88.6%**

The independent audits therefore agree at file level on nearly nine out of ten production files.

Claude’s reconciled inventory already incorporates the major corrections from the written reconciliation. The remaining 28 differences are narrower classification questions rather than architectural disagreements.

---

## Confusion matrix

Rows are Codex classifications; columns are Claude’s reconciled classifications.

| Codex \ Claude | UI | Mixed | I/O | Pure | API | Platform | Legacy |
|---|---:|---:|---:|---:|---:|---:|---:|
| UI | 92 | 8 | 0 | 2 | 0 | 0 | 0 |
| Mixed | 0 | 50 | 1 | 1 | 7 | 0 | 0 |
| I/O | 0 | 1 | 28 | 6 | 0 | 0 | 0 |
| Pure | 0 | 1 | 0 | 28 | 0 | 0 | 0 |
| API | 0 | 0 | 0 | 0 | 8 | 0 | 0 |
| Platform | 1 | 0 | 0 | 0 | 0 | 11 | 0 |
| Legacy | 0 | 0 | 0 | 0 | 0 | 0 | 1 |

The largest systematic difference is classification granularity:

- Claude’s reconciliation promotes seven API-wiring files from Mixed to API.
- Claude promotes eight behavior-bearing views from UI to Mixed.
- Codex distinguishes filesystem operations from pure parsing more strictly.

---

## Adjudicated disagreements

| File | Codex | Claude | Final | Reason |
|---|---|---|---|---|
| `DetailsComponentsTabView.swift` | UI | Mixed | **Mixed** | Contains version comparison, source detection, and direct component-folder behavior beneath the view |
| `ContentView.swift` | UI | Mixed | **UI** | Color conversion, WebView auth, crash-report presentation, and window state are client/platform behavior rather than agent logic |
| `ServerEditorView.swift` | UI | Mixed | **Mixed** | Coordinates settings validation, world operations, backup state, and destructive server actions |
| `MSCSettingsView.swift` | UI | Mixed | **Mixed** | Contains pairing-host safety, token lifecycle, reset, recovery, and storage calculations that need agent/API replacements |
| `RouterPortForwardGuideReader.swift` | UI | Mixed | **UI** | Token-to-row parsing is guide presentation; executable runtime resolution already lives in a separate file |
| `ResourcePackManager.swift` | I/O | Mixed | **I/O** | Domain records and filesystem behavior should be separated internally, but the whole file is portable resource-pack orchestration with no UI state |
| `AppViewModel+APIWiringAddons.swift` | Mixed | API | **API** | Glue should be deleted after route mappings and edge behavior become contract tests |
| `AppViewModel+APIWiringServerMgmt.swift` | Mixed | API | **API** | Same API-glue disposition |
| `AppViewModel+APIWiringSettings.swift` | Mixed | API | **API** | Same API-glue disposition |
| `AppViewModel+APIWiringContent.swift` | Mixed | API | **API** | Same API-glue disposition |
| `ServerEditorJarsTab.swift` | UI | Mixed | **Mixed** | Performs installation detection and JAR/file inspection that must move behind the agent |
| `PlayerNBTReader.swift` | I/O | Pure | **I/O** | `readAll` loads a path from disk before decompression and parsing; split reader from pure NBT parser |
| `AppViewModelModels.swift` | Pure | Mixed | **Pure** | Despite its filename, it does not depend on `AppViewModel`; it contains value types, property parsing/merge policy, and client-facing models |
| `CurseForgeManualDownloadSheet.swift` | UI | Mixed | **Mixed** | Watches a folder, detects downloads, and moves files; the workflow needs an explicit desktop/agent replacement |
| `BedrockPropertiesManager.swift` | I/O | Pure | **I/O** | Directly reads and atomically writes properties, allowlist, and permissions files |
| `RouterPortForwardGuideRuntimeResolver.swift` | Mixed | I/O | **Mixed** | Portable resolver logic shares a file with `AppViewModel` runtime-context gathering |
| `PlayerProfileCardView.swift` | UI | Mixed | **UI** | Avatar fetching, image cropping, and appearance presentation belong in the client; identity resolution should be supplied separately by the agent |
| `AppViewModel+APIWiringBackupsHealth.swift` | Mixed | API | **API** | API glue; capture mappings, then delete |
| `AppViewModel+APIWiring.swift` | Mixed | API | **API** | API glue and provider assembly |
| `AppViewModel+APIWiringWorlds.swift` | Mixed | API | **API** | API glue; capture mappings, then delete |
| `RouterPortForwardGuideCatalogLoader.swift` | I/O | Pure | **I/O** | Reads bundled JSON/Data and performs fallback loading; validation can be split as pure logic |
| `ServerLifecycleManager.swift` | Mixed | Pure | **Mixed** | Mutable lifecycle state and timers make this an application service/state machine, not a pure function |
| `JavaServerBackend.swift` | I/O | Pure | **I/O** | Owns and delegates to `ServerProcessManager`, resolves files, starts the process, sends commands, and terminates it |
| `QuickStartWindowController.swift` | Platform | UI | **UI** | AppKit is used only to present a window; rebuild as Tauri window behavior |
| `ServerPropertiesManager.swift` | I/O | Pure | **I/O** | Reads and writes `server.properties` on disk |
| `EULAManager.swift` | I/O | Pure | **I/O** | Reads and writes `eula.txt` on disk |
| `OverviewHealthHelpers.swift` | UI | Pure | **UI** | Pure calculation exists, but it is a view extension duplicating version logic that should consolidate into the component-version domain rather than port as a file |
| `NotesAutoSaveProxy.swift` | UI | Pure | **UI** | NSObject trampoline for debounced `TextEditor` saving; no engine behavior to translate |

### Adjudication result

- Claude classification adopted for **13** files.
- Codex classification retained for **15** files.

This is not a quality score. Most changes reflect the reconciled taxonomy becoming more precise after both audits.

---

## Canonical bucket decisions

### API wiring

All seven `AppViewModel+APIWiring*.swift` files belong to the API-contract migration set:

- Capture route/provider mappings
- Capture DTO transformation and defaults
- Add contract tests
- Delete the glue once Rust application services sit directly behind the routes

### UI deletion gate

A SwiftUI file is not automatically Mixed because it contains filesystem or network calls. The deciding question is whether the behavior belongs to the server agent or to the replacement client.

Examples:

- Avatar image cropping remains client behavior.
- Finder/file-picker presentation remains client behavior.
- Server installation detection belongs to the agent.
- Pairing security and token lifecycle belong to the agent/API.
- Console log parsing belongs to the agent/domain.

Static scanning should flag candidates, but deletion requires a short symbol-level disposition record.

### Pure versus I/O

A type is not pure merely because it avoids `AppViewModel`.

Files that read paths, write configuration, own processes, load bundles, or manage timers should be represented as application/I/O services with pure parsing extracted underneath.

This distinction matters because fixture strategy differs:

- Pure functions use input/output fixtures.
- I/O workflows require temporary directories, fake providers, process doubles, interruption cases, and rollback assertions.

---

## Final conclusion

The exact diff strengthens the reconciliation:

> There is no remaining disagreement about the MSC 2 architecture or whether Rust is justified.

The remaining differences were about where to draw test and ownership boundaries inside otherwise agreed migration work.

The canonical migration rules are:

1. API behavior becomes a formal contract.
2. AppViewModel services are translated only after implicit dependencies become explicit.
3. UI files receive symbol-level disposition before deletion.
4. Pure parsers and policies move against language-neutral fixtures.
5. Filesystem, process, timer, and provider workflows receive integration and rollback tests.
6. The legacy Docker Bedrock backend is not ported.
7. The macOS VZ backend remains a Swift sidecar initially.

