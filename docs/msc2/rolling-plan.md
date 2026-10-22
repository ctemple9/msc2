# MSC 2 — Rolling Plan

**Current priority:** P16.123 addresses the Intel macOS ticket placement found in v0.1.35. The earlier v0.1.33, v0.1.34, and v0.1.35 tags remain fixed and none of those candidates was published. No phase gate or exact-release artifact acceptance is implied. Cameron closed and archived the Java-first and Bedrock saved-terrain work through P18.95 on October 21, 2026. His Bedtonic check confirmed the latest biome-colour fix works. The [archived step records](rolling-plan-archive.md#october-21-2026--java-and-bedrock-saved-terrain-map-work) preserve the implementation history; [pending acceptance](pending-acceptance.md) and the [Phase 18 parity worksheet](phase18-map-acceptance.md#p1889--java-and-bedrock-map-parity-worksheet) retain checks that have not been recorded as passed.

## v0.1.33 release candidate

**Owner request (2026-10-22):** Increment the next tag and start the release workflow so Cameron can test the macOS changes in a downloaded build. The previous published tag is `v0.1.32`; `v0.1.33` is the next patch version. This release includes the changes recorded below through P16.117.

**Release preparation:** Bump the agent, desktop, Tauri, bundle identity, and lockfile versions together because the release workflow compares them and requires the tag to match. Add candidate release notes and update the download links and signing description. The tag push will start the build-only workflow for macOS Intel and Apple Silicon, Windows, and Linux, then publish the prerelease only after all nine desktop and headless artifacts are assembled with checksums and signed update metadata. This is the first release using the newly configured Apple notarization path, so successful workflow publication and Cameron's clean-Mac first-open check are still separate outcomes.

**P16.118 — Prepare and launch v0.1.33 release**

- **Status:** Release source prepared; workflow completion and owner verification pending.
- **Files:** `crates/msc-agent/Cargo.toml`; `Cargo.lock`; `clients/desktop-web/package.json`; `clients/desktop-web/package-lock.json`; `clients/desktop-web/src-tauri/Cargo.toml`; `clients/desktop-web/src-tauri/Cargo.lock`; `clients/desktop-web/src-tauri/tauri.conf.json`; `clients/desktop-web/src/lib/bundle-identity.ts`; `clients/desktop-web/static/bundle-identity.json`; `docs/msc2/release/v0.1.33.md`; `README.md`; `docs/msc2/rolling-plan.md`.
- **What:** Align all release identities at 0.1.33, document the candidate, push its source and exact tag, and start the existing build-only release workflow without changing its checks or moving an old tag.
- **Verify:** Run `gh run list --workflow release.yml --limit 1 --repo ctemple9/msc2` to inspect the `v0.1.33` run. After it completes successfully, open the v0.1.33 release page and confirm all nine platform artifacts, `SHA256SUMS`, and both signed update metadata files are present. Cameron then installs the exact Intel macOS DMG on a clean Mac and checks Finder first-open behavior.
- **Batch:** P16.118 only.

**Release failure (2026-10-22):** The v0.1.33 workflow run failed both macOS jobs at Apple notarization; Windows and Linux builds succeeded, but the publish job was skipped. Apple's log named `msc`, `bedrock-map`, `vantage`, and the Intel-only `BedrockSidecar` inside `MSC 2.app/Contents/Resources/agent`. Each lacked a valid Developer ID signature, secure timestamp, and hardened runtime. Tauri copied these as resources and signed the outer app, which did not sign the embedded executables. The release was not published. The next attempt must use a new version and tag.

**P16.119 — Sign bundled macOS helpers before notarization**

- **Status:** Implementation prepared; owner verification and a new release candidate pending.
- **Files:** `.github/workflows/release.yml`; `tools/release/sign-macos-agent-resources.sh`; `README.md`; `docs/msc2/rolling-plan.md`.
- **What:** After staging, sign each macOS helper with the configured Developer ID identity, hardened runtime, and secure timestamp. Apply the existing virtualization entitlement explicitly to the Intel Bedrock sidecar. Fail before the Tauri build if any helper lacks a valid signature, timestamp, hardened runtime, or sidecar entitlement. Correct the README to point to the actual latest published release, v0.1.32. Preserve the existing build-only artifact set and release gates; do not move or rerun v0.1.33.
- **Verify:** On a macOS runner with the configured Developer ID identity, run `APPLE_SIGNING_IDENTITY='Developer ID Application: Cameron Temple (6898622Y3Y)' bash tools/release/sign-macos-agent-resources.sh macos-x86_64` after staging the matching release payload. Confirm all four helpers report `signed and verified`; on Apple Silicon use `macos-aarch64` and confirm the three supported helpers. The next new-tag workflow must then pass notarization and publish all nine artifacts; Cameron checks the exact downloaded DMG on a clean Mac.
- **Batch:** P16.119 only.

**P16.120 — Prepare and launch v0.1.34 release**

- **Status:** Release preparation in progress; workflow publication and owner verification pending.
- **Files:** `crates/msc-agent/Cargo.toml`; `Cargo.lock`; `clients/desktop-web/package.json`; `clients/desktop-web/package-lock.json`; `clients/desktop-web/src-tauri/Cargo.toml`; `clients/desktop-web/src-tauri/Cargo.lock`; `clients/desktop-web/src-tauri/tauri.conf.json`; `clients/desktop-web/src/lib/bundle-identity.ts`; `clients/desktop-web/static/bundle-identity.json`; `docs/msc2/release/v0.1.34.md`; `docs/msc2/rolling-plan.md`.
- **What:** Give the signing correction its own immutable release tag, align the agent and desktop versions at 0.1.34, document the candidate, and launch the unchanged build-only publishing workflow. Do not move or rerun v0.1.33.
- **Verify:** Run `gh run list --workflow release.yml --limit 1 --repo ctemple9/msc2` and inspect the v0.1.34 run. After publication, check the release page for nine platform artifacts, `SHA256SUMS`, and both signed update metadata files; then install the exact Intel DMG on a clean Mac and open it through Finder.
- **Batch:** P16.120 only.

**Release failure (2026-10-22):** The v0.1.34 workflow signed all bundled helpers on both macOS runners. Apple's notary service returned `Accepted` and Tauri stapled both apps. Tauri then built the DMGs and removed its temporary `bundle/macos/MSC 2.app` directories. The workflow's existing post-build verification searched those removed directories and failed with “Tauri app bundle is missing” on both macOS jobs. Windows and Linux succeeded, but publication was skipped. This is a verification-path error after successful notarization, not another signing failure. The finished DMG contains `MSC 2.app`; that mount-and-find path was checked locally using the prior Intel DMG.

**P16.121 — Verify the macOS app inside the finished DMG**

- **Status:** Implementation prepared; owner verification and a new release candidate pending.
- **Files:** `.github/workflows/release.yml`; `docs/msc2/rolling-plan.md`.
- **What:** Mount the finished macOS DMG read-only and run the existing signature, Gatekeeper, and stapled-ticket checks against its actual app. Detach the image after verification. Preserve the build-only release workflow and its complete artifact requirements.
- **Verify:** On macOS, mount a finished release DMG with `hdiutil attach -readonly -nobrowse`, confirm `MSC 2.app` is present at its top level, then run `codesign --verify --deep --strict`, `spctl --assess --type execute`, and `xcrun stapler validate` on that mounted app. The next new-tag workflow must pass both macOS jobs and publish all nine artifacts.
- **Batch:** P16.121 only.

**P16.122 — Prepare and launch v0.1.35 release**

- **Status:** Release preparation in progress; workflow publication and owner verification pending.
- **Files:** `crates/msc-agent/Cargo.toml`; `Cargo.lock`; `clients/desktop-web/package.json`; `clients/desktop-web/package-lock.json`; `clients/desktop-web/src-tauri/Cargo.toml`; `clients/desktop-web/src-tauri/Cargo.lock`; `clients/desktop-web/src-tauri/tauri.conf.json`; `clients/desktop-web/src/lib/bundle-identity.ts`; `clients/desktop-web/static/bundle-identity.json`; `docs/msc2/release/v0.1.35.md`; `docs/msc2/rolling-plan.md`.
- **What:** Align release identities at 0.1.35, document the macOS post-notarization correction, and launch a new immutable tag through the existing build-only workflow. Keep the failed v0.1.33 and v0.1.34 tags fixed.
- **Verify:** Inspect the v0.1.35 release workflow and, after publication, check the release page for all nine platform artifacts, `SHA256SUMS`, and both signed update metadata files. Cameron then installs the exact Intel macOS DMG on a clean Mac and opens it through Finder.
- **Batch:** P16.122 only.

**Release failure (2026-10-22):** In v0.1.35, the Apple Silicon, Windows, and Linux jobs succeeded. Apple accepted and Tauri stapled the Intel app; its finished DMG contained an app with a valid signature, and `spctl` reported `accepted` with source `Notarized Developer ID`. However, `xcrun stapler validate` on the Intel app inside the finished DMG said no ticket was stapled to that copy, so the Intel job failed and publication was skipped. Apple's distribution guidance recommends notarizing and stapling the outermost DMG. The workflow must check the final distribution image's ticket, not rely on the app's temporary pre-DMG ticket surviving packaging.

**P16.123 — Notarize and staple the distributed macOS DMG**

- **Status:** Implementation prepared; owner verification and a new release candidate pending.
- **Files:** `.github/workflows/release.yml`; `docs/msc2/rolling-plan.md`.
- **What:** After Tauri signs and packages the macOS app, submit the final signed DMG to Apple, require an Accepted result, and staple its ticket. Keep signature and Gatekeeper checks on the app inside the mounted image, but validate the stapled ticket on the distributable DMG. This applies to both macOS architectures and does not add a test or lint release gate.
- **Verify:** On the next tagged release, both macOS jobs must log Accepted for the final DMG, pass `xcrun stapler validate` on the DMG, and pass the mounted app's `codesign` and `spctl` checks. Publication must include nine artifacts, checksums, and signed update metadata; Cameron checks Finder first open on the downloaded Intel DMG.
- **Batch:** P16.123 only.

## macOS owner testing — 0.1.32

**Started:** October 21, 2026
**Artifact:** `/Users/camerontemple/Downloads/msc2-0.1.32-macos-x86_64.dmg`
**State:** In progress. Cameron is testing a clean install after uninstalling MSC 2 and clearing its remaining app data.

### Finding 1 — macOS blocks the downloaded app at first open

**What Cameron experienced:** After downloading and opening `MSC 2.app` from the 0.1.32 disk image, macOS showed a warning titled “MSC 2.app Not Opened.” It said Apple could not verify that the app is free of malware that may harm the Mac or compromise privacy. The dialog offered **Done** and **Move to Trash**, with no way to open the app from that dialog. Cameron expects a normal download-and-open flow: download MSC 2, open the app, and begin using it without having to work around a Gatekeeper block.

**Investigation:** The exact tested DMG reports `code object is not signed at all` under `codesign -dv`; `spctl` rejects it. The release workflow builds the Tauri app with `--no-sign` and writes an unsigned-beta notice stating there is no release code signing, notarization, or publisher identity. The published installation guide documents the same limitation. Apple’s [Developer ID distribution guidance](https://developer.apple.com/developer-id/) says Developer ID signing lets Gatekeeper verify software from an identified developer; Apple’s [notarization guide](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) describes the notarization ticket Gatekeeper can check when an app is first opened. This is therefore a release-signing and notarization gap, not a problem specific to Cameron’s Mac.

**User outcome required:** Sign distributed macOS desktop apps with the appropriate Developer ID identity, notarize the distributed app (and disk image where required by the packaging flow), staple the ticket where supported, and verify the exact downloaded artifact opens through the ordinary Finder double-click flow on a clean Mac without the malware-verification block.

**Implementation work (2026-10-21):** The macOS release legs in `.github/workflows/release.yml` now import the Developer ID certificate into a temporary keychain and require App Store Connect notarization credentials before building. Tauri builds the macOS DMGs with signing enabled; the workflow then checks the app signature with `codesign`, Gatekeeper assessment with `spctl`, and the app’s stapled notarization ticket with `xcrun stapler validate`. These checks are part of producing a trusted macOS release artifact. The existing platform artifact set, checksum generation, signed update manifest, and Windows/Linux unsigned-beta behavior remain in place. The per-platform macOS notice now says the desktop app is signed and notarized and distinguishes the separate, unsigned headless archive.

**Release setup (2026-10-21):** Cameron configured the required GitHub Actions secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `MACOS_KEYCHAIN_PASSWORD`, and `APPLE_API_KEY_P8_BASE64`) and repository variables (`APPLE_SIGNING_IDENTITY`, `APPLE_API_KEY`, and `APPLE_API_ISSUER`). The workflow fails before packaging if any are absent, instead of producing another untrusted app while appearing to succeed. The next macOS release still needs to complete and Cameron must verify the downloaded app on a clean Mac.

**Verify after publication:** Install the exact new x86_64 DMG on a clean Mac, drag MSC 2 to Applications, and open it with a normal Finder double-click. Gatekeeper should not show the malware-verification block. The release build also records machine checks for `codesign --verify --deep --strict`, `spctl --assess --type execute`, and `xcrun stapler validate` on the bundled app. Until that downloaded-artifact check passes, this finding remains open.

**P16.94 — Add Developer ID signing and notarization to macOS releases**

- **Status:** Implemented; Apple credentials configured, awaiting owner verification of the next release.
- **Files:** `.github/workflows/release.yml`; `docs/msc2/rolling-plan.md`.
- **What:** Require Apple signing and notarization credentials, build the macOS DMGs with Tauri signing enabled, and verify the app signature, Gatekeeper assessment, and stapled ticket. Preserve the nine release assets, checksum generation, signed update metadata, and unsigned Windows/Linux behavior.
- **Verify:** Install the next x86_64 DMG on a clean Mac, then run `codesign --verify --deep --strict "/Applications/MSC 2.app" && spctl --assess --type execute "/Applications/MSC 2.app" && xcrun stapler validate "/Applications/MSC 2.app"`; open the app with Finder and confirm Gatekeeper does not block it.
- **Batch:** P16.94 only.

### Finding 2 — The DMG install window needs clearer, cleaner guidance

**What Cameron experienced:** The mounted disk image opens as a large, mostly empty Finder window. It shows the Applications shortcut and the MSC 2 app near the upper left, but there is no arrow or other visual cue telling a new user to drag MSC 2 onto Applications. When Finder is set to show hidden files, a third item named `.VolumeIcon.icns` appears beside the app. Cameron wants the installer window to feel finished: smaller, arranged around the install action, and clearly showing the app being dragged to Applications.

**Investigation:** The Tauri bundle configuration had no `macOS.dmg` settings. Tauri’s [DMG configuration](https://v2.tauri.app/reference/config/#dmgconfig) supports a custom background image, window size, and positions for the app and Applications folder. Its normal Finder-based layout step is skipped in CI, however, unless `TAURI_BUNDLER_DMG_IGNORE_CI=true` is set; this release is built on GitHub Actions, which explains why relying on Tauri’s defaults does not produce the intended layout in the published DMG. The `.VolumeIcon.icns` name and appearance identify the extra item as the disk image’s custom volume icon resource. Finder normally hides dot-prefixed files, which is why it is absent in the first screenshot and visible in the second. Cameron decided to keep this resource, so it remains part of the volume presentation.

**User outcome required:** A compact Finder window should place the MSC 2 app and Applications shortcut with a clear arrow between them, making the expected drag-and-drop install obvious. Cameron decided to keep the custom mounted-volume icon even though `.VolumeIcon.icns` is visible when Finder is set to show hidden files; removing the icon is not part of this finding. Tauri’s documented layout controls have been confirmed; implementation can focus on the DMG window dimensions, item positions, and a background image that shows the drag direction.

**Implementation work (2026-10-21):** Added a 640 × 360 DMG layout in `clients/desktop-web/src-tauri/tauri.conf.json`, positioned MSC 2 and Applications on opposite sides, and added a restrained background instruction with a right-pointing arrow. The macOS release build now sets `TAURI_BUNDLER_DMG_IGNORE_CI=true` so Tauri applies its Finder-based background, window, and icon layout on the GitHub Actions runners. The custom volume icon remains untouched. Tauri documents the DMG layout fields in its [configuration reference](https://v2.tauri.app/reference/config/#dmgconfig) and [DMG guide](https://v2.tauri.app/distribute/dmg/); the CI override follows the Tauri bundler maintainers’ documented resolution in [issue #740](https://github.com/tauri-apps/tauri-action/issues/740).

**P16.95 — Polish the macOS DMG install window**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS release.
- **Files:** `clients/desktop-web/src-tauri/tauri.conf.json`; `clients/desktop-web/src-tauri/dmg-background.png`; `.github/workflows/release.yml`; `docs/msc2/rolling-plan.md`.
- **What:** Set a compact 640 × 360 Finder window, place MSC 2 and Applications around a clear drag instruction and arrow, and enable the Tauri DMG layout step on CI. Keep the custom volume icon.
- **Verify:** Open Downloads in Finder with `open "$HOME/Downloads"`, then open the next x86_64 DMG; confirm the window is compact, the app and Applications icons flank the arrow, and `.VolumeIcon.icns` remains hidden unless Finder is set to show hidden files.
- **Batch:** P16.95 only.

### Finding 3 — First-time setup leaves too much empty space and underspecifies the steps

**What Cameron experienced:** On the first page of First-time Setup, the “What You’ll Do” section puts three steps in a horizontal row: choosing a server platform, setting up the host, and reviewing access. The row ends well above the bottom of the sheet, leaving a large unused area. Cameron suggests stacking the three steps vertically and adding useful explanation so the page uses that area to help new users understand what they will choose, without merely stretching the layout or creating a different empty area.

**Investigation and design:** `SetupIntro.svelte` rendered this as a three-column ordered list. The later pages show what each step entails: selecting Java and/or Bedrock, choosing a servers folder and (for Java) selecting or installing a Java runtime, then reviewing optional playit.gg and Xbox Broadcast setup. The list can stay numbered while stacking its entries vertically and describing the choices shown on those pages:

1. **Choose what to host** — Select Java, Bedrock, or both. Bedrock is available when the selected host can run it.
2. **Set up your host** — Choose where server files live. For Java, select an installed runtime or install one; Bedrock uses MSC’s built-in runtime.
3. **Review ways for players to join** — playit.gg can let friends join without port forwarding, and Xbox Broadcast helps console players find the server. Both are optional and can be configured later.

Keep this as one simple numbered list with a clear title and readable secondary descriptions; do not wrap each row in another card. This fills the current blank area with information that prepares users for the next pages, while keeping optional network setup clearly optional. The screenshot establishes the layout problem; the descriptions follow the current setup-page behavior in `SetupIntro.svelte`.

**Implementation work (2026-10-21):** The three summaries now stack vertically, with more specific descriptions of the actual server type, host, and optional connection choices. The existing numbered-list structure remains, without new cards or decorative accents. Spacing and text size give the entries room to read while using the previously empty part of the fixed-height sheet. Compact and narrow layouts use the same vertical order, with tighter spacing on narrow windows.

**P16.96 — Explain the first-time setup steps vertically**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Replace the horizontal three-column setup preview with a single vertical numbered list and accurate descriptions of Java/Bedrock selection, server storage and runtime, and optional player access.
- **Verify:** On a clean install of the next macOS build, run `open -a "MSC 2"`; on the first First-time Setup page, confirm the three steps read vertically, explain the upcoming choices, and fit comfortably above the footer without clipping.
- **Batch:** P16.96 only.

### Finding 4 — Optional Playit setup needs a clearer label and account-capacity warning

**What Cameron experienced and wants:** On the playit.gg page in First-time Setup, the subtitle begins with “Optional,” but that word looks like ordinary subtitle text. Cameron wants “Optional” to stand out in orange on every optional page in this setup flow, including playit.gg and Xbox Broadcast, so users can recognize before proceeding that they may skip those services. He also reports a concrete failure from prior testing: when a Playit account already had one MSC agent, trying to create another MSC agent on that account did not work. This is why users need a warning before sign-in, rather than discovering the account limit through a failed setup. A user who already has a free account may have agents or tunnels using the account’s allowance and may need to make room. The page should explain that MSC reuses the Playit agent and tunnel for each server type across servers: Java servers share the Java public address, and Bedrock servers share the Bedrock public address. It should set that expectation before the user creates or reuses Playit resources.

**Investigation:** `SetupIntro.svelte` marks both optional pages with “Optional” in the subtitle but applies no separate emphasis. Its Playit copy currently says free accounts include one agent and up to three tunnels, but does not warn existing users that an account already containing an MSC agent may reject another MSC agent. Cameron’s prior hands-on result confirms that this is a real setup failure. Current Playit docs confirm that the free tier supports Minecraft Java and Bedrock and that Premium raises the port allowance from four to sixteen and provides more agents; they do not state the exact free-agent count or confirm the app’s “three tunnels” wording. See [Playit’s current pricing page](https://playit.gg/pricing) and [Premium feature details](https://playit.gg/support/playit-premium/). Preserve Cameron’s observed failure as the basis for the warning, and do not present the current exact quota text as independently verified until checked against Playit’s account UI or current support documentation.

The MSC setup code claims a Playit agent for a new account setup, or reuses the agent already saved for this host. It inventories and reuses named Java, Bedrock, and voice tunnels; the stable tunnel names are shared across servers rather than creating a separate tunnel for every server. That means servers of the same type use the same public tunnel address. MSC does not currently offer a choice of an arbitrary existing Playit agent or automatically delete Playit agents/tunnels to free account capacity. The warning must not imply that MSC will clean up the user’s account for them.

**User outcome required:** Emphasize the “Optional” label in orange on every optional First-time Setup page, consistently and without recoloring the entire subtitle. Before directing the user to sign in, explain that Playit limits the number of agents and ports/tunnels on an account, that existing resources may use that allowance, and that they may need to review or remove unused resources in their Playit account before MSC setup can finish. Explain that MSC reuses one Java tunnel and one Bedrock tunnel across servers of each type, so those servers share the corresponding public address rather than receiving a unique Playit address per server. Verify quota wording against Playit’s then-current authoritative information before shipping exact numbers.

**Voice direction and proposed copy:** Cameron wants this real first-person experience to appear in the app. It gives the guidance a human voice and shows that the person building MSC has encountered the same setup snag. Keep it direct and useful, without turning the page into marketing copy. Draft notice:

> I ran into this while testing MSC: I couldn’t add another MSC agent to a Playit account that already had one. If you already use Playit, check your account for room before continuing; you may need to remove agents or tunnels you no longer use. MSC reuses one Java tunnel and one Bedrock tunnel across your servers, so all your Java servers share one Playit address, and all your Bedrock servers share another.

Keep the first-person account tied to Cameron’s reported experience. Do not imply that MSC can inspect or clean up the user’s Playit account, and avoid fixed quota numbers until verified.

**Implementation work (2026-10-21):** Both optional setup pages now render “Optional” as a separate orange label before the page description. The Playit page removes the unverified fixed quota claim, adds Cameron’s first-person report about the failed second-agent setup, tells users to review their free-account capacity and remove only unused resources if needed, and explains that Java servers share one Playit address while Bedrock servers share another.

**P16.100 — Clarify optional Playit setup and account capacity**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Emphasize the optional label on both optional setup pages and replace fixed Playit quota text with the observed account-capacity warning and shared Java/Bedrock tunnel explanation.
- **Verify:** On a clean install of the next macOS build, open First-time Setup and confirm “Optional” is orange on the playit.gg and Xbox Broadcast pages; confirm the Playit page describes the prior second-agent failure, advises reviewing account capacity without fixed quota numbers, explains shared Java and Bedrock addresses, and still allows the page to be skipped.
- **Batch:** P16.100 only.

### Finding 5 — Xbox Broadcast does not explain where its JAR goes or what “shared” means

**What Cameron asked/experienced:** On the Xbox Broadcast page, the “Broadcast Helper” section says the tool is downloaded once and shared across all servers, and offers a **Download Now** button. It does not say where the file is stored, who publishes it, or whether “shared” means one copy is used by every server or each server gets its own JAR. Cameron wants to know what happens in a setup with three Java servers and one Bedrock server, all using Xbox Broadcast, and where the downloaded JAR comes from.

**Investigation:** MSC downloads the `MCXboxBroadcastStandalone.jar` asset from the latest release published by the [MCXboxBroadcast/Broadcaster project](https://github.com/MCXboxBroadcast/Broadcaster). MSC resolves the latest release through GitHub’s release API and verifies the asset using the upstream SHA-256 digest before caching it. The shared file lives below the agent data directory at `helpers/xbox-broadcast/<release tag>/MCXboxBroadcastStandalone.jar`. On macOS, the default agent data directory is `~/Library/Application Support/MSC2`; an installed agent can override this through `MSC2_DATA_DIR`, so the actual path may differ on a configured install.

The JAR is stored in the shared agent cache, not copied into each server directory. If a user skips **Download Now** and later enables Xbox Broadcast, MSC acquires the JAR as part of starting that server’s helper and puts it in the same shared cache; the server directory gets its `.msc2-broadcast` working/configuration folder, not another JAR. In Cameron’s example, all four server processes launch the cached JAR path. One implementation detail needs attention: the acquisition routine currently fetches the latest release asset on every acquisition, even when that version is already in the cache, then replaces the same cached path. So there is only one cached copy per release version, but the current code does not yet guarantee a one-time network download. **That means the setup download’s practical value today is only to confirm that the JAR can be fetched and verified early; it does not save the user a download or wait when they first start Xbox Broadcast, because startup fetches the asset again.** The page’s “downloaded once” sentence describes the shared storage model, but is too strong about actual download frequency and leaves the location unstated.

**Cameron’s corrected test sequence and filesystem check:** Cameron confirms he *did* select **Download Now** during first-time setup and saw the verified-download message, then created the Paper server with Xbox Broadcast enabled. The downloaded file is currently at `/Users/camerontemple/Library/Application Support/MSC 2/helpers/xbox-broadcast/159/MCXboxBroadcastStandalone.jar`. Paper has a per-server `.msc2-broadcast/config.yml`, but no copy of `MCXboxBroadcastStandalone.jar` in its server directory. When MSC starts the broadcast helper for Paper, it performs another acquisition and launches the JAR from the shared cache path; it does not move the setup download into Paper’s folder. The acquisition currently fetches the release asset again and replaces the file at that same versioned cache path.

**Cameron’s revised experience and desired behavior (2026-10-21):** Downloading Xbox Broadcast during First-time Setup gave Cameron a verified JAR, but creating his Paper server still downloaded the helper again. That makes the setup download feel disconnected from the server that later uses it. He wants one current Xbox Broadcast JAR managed by each MSC agent, whether the user downloads it during initial setup, while creating a server, or much later. Every server with Xbox Broadcast enabled should launch that same agent-owned JAR. Updating it from one server should update the agent’s JAR for every server. After a user signs in to Xbox Broadcast once on that agent, enabling it for another server should use the same Microsoft sign-in without asking for a new device code. A server created without Xbox Broadcast should have no Xbox Broadcast entry in its Components tab; enabling it later should make that entry appear. Cameron clarified that **the sign-in is shared**, not the entire server target configuration.

**Additional code investigation:** The current download and startup paths both use the agent’s versioned helper cache, but startup calls acquisition again; the acquisition code fetches the latest release and asset even when that version is cached. A server’s broadcaster launches from its own `.msc2-broadcast` working folder and writes a `config.yml` with that server’s target host, port, and display name. MSC already stores the alternate Microsoft account identity and password at agent scope, but the actual standalone helper runs in each server’s working folder; its sign-in state therefore does not automatically follow another server. MSC also has a per-server auth-token secret key. The Components tab currently renders an Xbox Broadcast row for eligible Java and Bedrock servers even when that server has not enabled it. The [upstream standalone instructions](https://github.com/MCXboxBroadcast/Broadcaster/blob/master/README.md) likewise place the target address in `config.yml` and launch the JAR from a working directory. Shared sign-in needs to account for the helper’s own cached authentication files; merely sharing the JAR or MSC’s displayed account name would not accomplish it.

**Updated user outcome required:**

1. The agent has one current, checksum-verified Xbox Broadcast JAR. All download entry points use that agent-owned installation. A valid installed JAR is reused on later starts without another asset download. An explicit update from any server replaces the agent’s current JAR safely, and every server then reports the same installed version. Temporary staging during a verified update is acceptable; failed updates must leave the working JAR usable. Account for running helper processes and Windows file locking before replacing a JAR they use.
2. Sign-in belongs to the agent. The first enabled server can complete the Microsoft device-code flow; subsequent enabled servers on that agent reuse the authenticated account. Keep each server’s target host, port, display name, enabled setting, and running broadcast process separate so changing one server does not redirect another. Determine how the upstream helper reads and refreshes its auth cache, then migrate existing per-server sign-ins without exposing tokens or silently forcing a new login. Verify shared sign-in when multiple enabled servers run at the same time before promising that case.
3. Show the Xbox Broadcast component only for a server that has Xbox Broadcast enabled, on eligible Java and Bedrock servers. The server creation and later server settings controls must still let someone enable it. Once enabled, the Components row reflects the one agent-owned JAR and offers an agent-wide download or update action; disabling the feature removes the row for that server.
4. Explain in setup that the JAR comes from the MCXboxBroadcast project, is stored with the agent, and is used by every enabled server. Explain that skipping the early download simply defers installation until the feature is enabled or started later. State clearly that Microsoft sign-in is shared on this agent while each server advertises its own destination.

**Implementation order:** First establish the agent-owned JAR and safe reuse/update path. Then move helper authentication to agent scope with migration from existing server folders while preserving per-server targets and processes. Finally, align setup, server creation, settings, and Components UI with the shared state. Cameron will verify both a clean setup and an existing signed-in installation, including a server enabled later and more than one running broadcaster. No release or test suite run is authorized by this design record.

**Shared-JAR implementation work (2026-10-21):** First-time Setup and the explicit download action still fetch and verify the current upstream JAR, but now record which release is current for the agent. Starting Xbox Broadcast reuses that verified cache entry without contacting GitHub again. Existing 0.1.32 caches have no selection record, so the agent discovers the most recent valid cached release rather than requiring a fresh download. The JAR status endpoint reports the selected, checksum-verified release. An explicit download updates that agent selection for all servers; it refuses the update while a broadcaster is starting or running, avoiding replacement of an in-use JAR, especially on Windows. A failed download leaves the previous selection in place. The agent still stores JARs in its shared helper cache rather than in server folders. Older cached release directories may remain on disk, but only one release is selected for starts. Agent-wide sign-in, existing-sign-in migration, and conditional Components visibility remain separate work under this finding.

The focused new test protects the concrete 0.1.32-to-next-build case Cameron encountered: after a setup download, starting while offline must reuse the verified file; the same must work for a legacy cache without a selection record, and damaged bytes must be rejected. It uses an in-memory filesystem and transport with no network or sleeps, and should run in milliseconds. It was added but not run, in keeping with the test policy.

**P16.102 — Reuse the agent's selected Xbox Broadcast JAR**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build. Finding 5 remains open for shared sign-in and Components behavior.
- **Files:** `crates/msc-infrastructure/src/xbox_broadcast.rs`; `crates/msc-application/src/xbox_broadcast.rs`; `crates/msc-application/tests/xbox_broadcast.rs`; `crates/msc-agent/src/routes/networking.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Reuse a verified setup download across server starts, recognize an existing valid cache, report one current JAR, and prevent an explicit update from replacing a JAR while any broadcaster uses it.
- **Verify:** Run `open -a "MSC 2"` on the next macOS build; download Xbox Broadcast in First-time Setup, create an enabled server, disconnect the network, and start its broadcaster. It should use the installed JAR without another download. Reconnect, stop all broadcasters, explicitly download from Components, and confirm the reported version is the same from each enabled server. An update attempted while a broadcaster is running should explain why it cannot proceed.
- **Batch:** P16.102 only.

**P16.101 — Define shared Xbox Broadcast behavior**

- **Status:** Design recorded; awaiting owner review before implementation.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Record the clarified agent-owned JAR and sign-in behavior, per-server targets and enablement, Components visibility, migration needs, and safe update requirements for Finding 5.
- **Verify:** Run `sed -n '99,145p' docs/msc2/rolling-plan.md` and review Finding 5 against the intended Xbox Broadcast experience.
- **Batch:** P16.101 only.

### Finding 6 — The first tour card uses an em dash Cameron considers AI-like

**What Cameron experienced:** On the initial **Welcome to MSC** tour card, the body reads: “Let’s get your first Minecraft server online. This tour takes about two minutes — we’ll do everything together.” Cameron called out the dash as AI slop and wants it removed from this first card. The punctuation makes a short, simple welcome line feel like generated marketing copy instead of MSC speaking plainly.

**Investigation and copy proposal:** The sentence is the `welcome` entry in `content/guides/onboarding.json`. The dash is an em dash, not a hyphen. Replace it with ordinary sentence punctuation and make the guidance concrete. Suggested copy: “Let’s get your first Minecraft server online. This tour takes about two minutes. I’ll guide you through each step.” This keeps the time estimate and friendly first-person voice while removing the stylized pause.

**User outcome required:** Remove the em dash from the body of the first tour card. Keep the welcome concise and natural; use a period between the time estimate and the guidance rather than another decorative dash.

**Implementation work (2026-10-21):** Replaced the em dash in the welcome card with a period and changed the second clause to a direct first-person promise: “I'll guide you through each step.” The tour duration remains unchanged.

**P16.103 — Rewrite the first tour card without an em dash**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `content/guides/onboarding.json`; `docs/msc2/rolling-plan.md`.
- **What:** Remove the em dash from the first Welcome to MSC tour card and make the guidance concise and personal.
- **Verify:** On the next macOS build, open the first tour card and confirm it reads, “Let's get your first Minecraft server online. This tour takes about two minutes. I'll guide you through each step.”
- **Batch:** P16.103 only.

### Finding 7 — Hidden-tour controls overlap MSC’s top-right controls

**What Cameron experienced:** After hiding the tour tip, the **Show tip** and **Skip tour** buttons appear in the upper-right corner of the app. In the screenshot they sit on top of MSC’s own header controls, so the controls visually collide and are harder to distinguish. Cameron wants this area cleaned up so the tour actions remain available without covering MSC buttons.

**Investigation:** `TourOverlay.svelte` renders the hidden-tour buttons in `.hidden-pills`, positioned `fixed` at `top: 16px; right: 16px`. The `TopBar.svelte` independently places MSC’s sidebar, console, help, preferences, and refresh buttons at the right edge of the same app window. The tour placement is therefore anchored to the viewport corner rather than to space reserved by the app shell, which causes the collision visible in Cameron’s screenshot.

**User outcome required:** Keep **Show tip** and **Skip tour** reachable while the tour card is hidden, but place them in a deliberate region that does not cover the native title bar or MSC’s top-right controls. Check the layout at typical and narrower window widths so the tour controls do not collide with either the header actions or page content.

**Implementation work (2026-10-21):** The hidden-tour actions now appear in the TopBar’s action layout beside MSC’s existing controls, so they take up real header space instead of floating over those controls. At widths of 600 pixels or less, the tour actions move to a separate row beneath the icon controls. The tour state publishes the two actions to the header only while its card is hidden and clears them when the tour advances or closes.

**P16.104 — Keep hidden-tour actions out of the header controls**

- **Status:** Implementation recorded; awaiting owner visual verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/help/onboarding.ts`; `clients/desktop-web/src/lib/help/TourOverlay.svelte`; `clients/desktop-web/src/lib/components/shell/TopBar.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Put **Show tip** and **Skip tour** in the TopBar’s normal layout, with a separate row at narrow widths, and clear those controls when the tour card returns or the tour closes.
- **Verify:** On the next macOS build, hide a tour card at the normal window width and at 600 pixels or less; confirm both actions remain reachable, sit within the header layout, and never cover MSC’s icon controls or page content. Use **Show tip** and **Skip tour** to confirm each action still works.
- **Batch:** P16.104 only.

### Finding 8 — Choosing Java can sit on “Detecting installed Java runtimes” for too long

**What Cameron experienced:** While creating a Java server, the **Choose Java for this server** dialog can remain on “Detecting installed Java runtimes…” for a noticeable time. Cameron described it as the app taking a long time to discover which Minecraft version he is using and wants the wait shortened so server setup does not stall on this step. The screenshot already shows that the selected Minecraft version requires Java 25, while the runtime list is still loading, so the user is left waiting after that key information is known.

**Investigation:** `AddServerWizard.svelte` first resolves the Minecraft release and required Java major, then waits for the `/v1/java-runtimes` response before it can show the runtime choices. That route scans the machine’s Java locations and probes every discovered Java executable by running `java -version`. Those probes ran one after another, and each has a timeout of up to ten seconds. This source tracing localizes the wait to Java runtime detection after the Minecraft version requirement is known. I did not measure timings on Cameron’s Mac, so I cannot say whether filesystem scanning also adds noticeable time or identify a specific slow Java executable from the screenshot.

**User outcome required:** Reduce the time the Java picker blocks on runtime detection while preserving the correct compatibility check for the selected Minecraft version. A focused investigation should measure the stages separately (version lookup, filesystem scan, Java probes); then avoid making the whole picker wait on slow probes, for example by reusing valid recent probe results or checking candidates with a bounded amount of parallel work. Keep slow or failed runtimes from holding up usable choices unnecessarily, and show an honest state if the results are still being completed.

**Implementation work (2026-10-21):** The `/v1/java-runtimes` route was checking discovered Java executables one at a time, and each `java -version` probe can wait up to ten seconds. The route now checks up to four runtimes concurrently and restores the original discovery order before responding. This keeps the same real version probes and compatibility results while preventing the delay from every installed runtime from accumulating in sequence. A single slow runtime can still take up to ten seconds; the next macOS build needs owner verification to confirm the picker feels faster on the test machine.

**P16.105 — Check installed Java runtimes concurrently**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `crates/msc-agent/src/routes/versions.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Probe up to four installed Java runtimes at once instead of waiting for each ten-second `java -version` timeout in sequence. Preserve each runtime's existing verification and the endpoint's stable response order.
- **Verify:** On the next macOS build, open **Add Server**, choose a Java server, and select Java for that server. Confirm the detected runtime list appears sooner when multiple Java runtimes are installed, and that the listed Java versions and compatibility choices remain correct.
- **Batch:** P16.105 only.

### Finding 9 — The Add Server success page is blank after creating a Paper server

**What Cameron experienced:** After creating his first Paper server, the Add Server wizard reaches its final **Confirm** step and shows a **Done** button, but the large page body is empty. Cameron expected the page to say that the server was created and explain what to do next. With only the progress row and **Done** visible, the result is unclear and the user has no visible confirmation or next-step guidance.

**Investigation:** The success state is present in `ConfirmStep.svelte`: when `createSucceeded` is true it should show “<server name> created,” followed by instructions based on the server’s world and software settings. `AddServerWizard.svelte` sets that state after its create operation succeeds, passes it to the confirmation component, and changes the footer action to **Done**. The screenshot shows the success-state footer but none of the expected body, so the visible behavior does not match the component’s intended success branch. The screenshot confirms the rendering defect; it does not establish why the success content failed to appear.

**User outcome required:** After successful creation, show a clear server-created confirmation and relevant next steps in the page body, with the server’s name visible. Investigate why that success content is absent in the tested build while the **Done** button is present, then verify the final page for Paper and other creation paths.

### Finding 10 — Playit setup claims an agent but stops before creating tunnels

**What Cameron experienced:** While connecting Playit for the first time, MSC said it could not start the Playit agent because the helper exited with code 1. Playit had already created an agent, but no tunnels appeared. The setup screen offered another sign-in attempt, which makes it look as though the account setup may have partly worked while leaving the server without a usable Java or Bedrock address.

**Investigation:** The installed MSC agent confirms that a Playit agent ID and key were saved, while the helper is stopped and both tunnel addresses are empty. In `PlayitAccountSetup::run_with_tunnels`, MSC first claims the cloud agent and saves its ID/key, then starts `playitd` and waits for that helper to confirm it connected to the same agent. MSC only lists or creates tunnels after that connection succeeds. So the observed “agent, but no tunnels” state follows the current order: the helper failed before tunnel provisioning began.

The installed macOS helper is the expected universal x86_64/arm64 binary, and its SHA-256 matches MSC’s pinned release digest. Running its help screen confirms it accepts both launch options MSC supplies (`--secret-path` and `--socket-path`), so the evidence does not point to a corrupt download or unsupported command-line arguments. I could not recover the helper’s output from this failed attempt. The setup error is built from the process exit status alone, even though the helper manager captures redacted output; the visible “code 1” does not identify whether Playit rejected the saved key, hit an account limit, or encountered another startup problem. The app’s current diagnostic endpoint also had no retained Playit lines for this attempt.

**Retest result:** Cameron deleted the failed agent from his Playit account and retried setup; it then worked. This confirms the flow can complete after removing that agent, but it does not establish why the first helper exited or whether deleting the agent itself fixed the underlying condition. The failure has not been reproduced, so treat the cause as unknown rather than a confirmed code defect.

**Follow-up if it recurs:** Preserve the user-facing error’s useful, redacted helper output so the actual startup reason can be identified. Do not report tunnel provisioning as the cause until the helper connection succeeds. No code change is currently justified by this one non-reproducible failure.

### Finding 11 — Put “Change Version” in the server card’s action menu

**What Cameron experienced and wants:** On the Components page, the Paper server card has a standalone **Change Version** button at its far right. Cameron wants the whole server card to use the same interaction as the installed plugin rows: clicking the card selects it, shows the blue perimeter highlight, and opens an anchored action menu. That menu should contain **Change Version**, so the action appears in the context of the selected server card instead of sitting by itself beside the card.

**Investigation and design direction:** In `ComponentsSection.svelte`, the Java server card currently renders a passive row plus a separate **Change Version** button; the Bedrock runtime card uses the same standalone-button arrangement. Installed add-on rows already implement the requested pattern: clicking the row records which item is selected, applies the blue outline, and opens the shared anchored `Menu`. The version picker itself is already a separate `VersionPickerSheet`, so the menu item can open the existing picker without changing version-selection behavior. Preserve the current rule that version changes are unavailable while the server is running. The server card can follow the add-on row’s selected outline and menu placement, with keyboard activation opening the menu at a sensible position relative to the card.

**User outcome required:** Make the Java server card select and highlight on activation, then show an anchored action menu containing **Change Version**. Keep the existing version picker and running-server restriction. Apply the same interaction to the Bedrock runtime card for consistency, unless a platform-specific reason makes that behavior inappropriate.

**Implementation work (2026-10-21):** The Java, modded-loader, and Bedrock server cards now open the shared anchored action menu when activated. The selected card gets the same restrained blue perimeter used by the add-on rows, and **Change Version** opens the existing picker. The action remains disabled while the server is running. Keyboard activation places the menu below the focused card; mouse activation anchors it at the click.

**P16.106 — Move server version changes into the card menu**

- **Status:** Implementation recorded; awaiting owner visual and keyboard verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Make Java/loader and Bedrock server cards selectable, show the blue perimeter while their action menu is open, and move **Change Version** into that menu. Reuse the current picker and keep version changes unavailable while the server runs.
- **Verify:** On the next macOS build, open **Components** for a Java server and for a Bedrock server. Click each server card and confirm it highlights and opens a menu with **Change Version**; choose it and confirm the existing picker opens. Repeat by focusing the card and pressing Enter or Space. Start a server and confirm **Change Version** is disabled in the menu.
- **Batch:** P16.106 only.

### Finding 12 — Show “chunks” beside Simulation Distance on Java and Bedrock

**What Cameron experienced and wants:** In the Paper Settings screen, **View Distance** shows its unit, “chunks,” while **Simulation Distance** is only a number. The missing unit leaves users to infer what the value measures and makes two related distance settings look inconsistent. Cameron wants the “chunks” unit shown for Simulation Distance anywhere MSC exposes that setting, across all server types.

**Investigation:** The Java Settings response exposes `simulation-distance`; the Bedrock Settings response exposes the equivalent `tick-distance`, both labeled **Simulation Distance**. Both currently omit their unit in the setting-field DTO, while View Distance includes `chunks`. The archived P14.97 step removed the suffix from both Java and Bedrock fields, so this new owner direction reverses that earlier presentation choice for both server types. Preserve the existing numeric limits and values; this is a unit-label change.

**User outcome required:** Show **chunks** beside Simulation Distance for Java (`simulation-distance`) and Bedrock (`tick-distance`), matching the existing View Distance unit treatment. Apply it consistently wherever these server settings are rendered.

**Implementation work (2026-10-21):** Added the existing `chunks` unit to the Java `simulation-distance` and Bedrock `tick-distance` field descriptions. Their numeric values, bounds, and setting behavior are unchanged. The Settings screen already displays a field’s unit when supplied, so no separate UI change was needed.

**P16.107 — Show chunks for Simulation Distance**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `crates/msc-agent/src/routes/settings.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Add the `chunks` unit to Java Simulation Distance and Bedrock Simulation Distance while preserving the numeric settings and limits.
- **Verify:** On the next macOS build, open Settings for Java and Bedrock servers and confirm both Simulation Distance fields show **chunks**, matching View Distance.
- **Batch:** P16.107 only.

### Finding 13 — Add thumbnail and map actions to the Overview’s Active World card

**What Cameron wants:** On Overview, clicking the Active World thumbnail should select the thumbnail area with the same blue perimeter used for selected cards elsewhere in MSC, then open a small action menu with **Change Thumbnail** and **View Map**. Choosing **Change Thumbnail** should take the user to the Worlds tab and open the same image-selection flow as **Set Thumbnail** on the world card. Choosing **View Map** should take the user to the Worlds tab and start the existing map-view flow for that world. This gives users a direct route from the Overview card while keeping the actual world actions in the Worlds area.

**Investigation:** `ActiveWorldCard.svelte` currently renders the thumbnail as a non-interactive `div`; the card has only **Switch** and **Backup** actions. `WorldSlotCard.svelte` already provides the thumbnail flow: **Set Thumbnail** opens the native image picker for PNG, JPG/JPEG, GIF, or WebP, then uses the existing staged upload and per-slot thumbnail endpoint. `WorldsSection.svelte` owns the world selection state, action menus, and map viewer. Its map viewer is opened for the active slot, and the existing **View Map** button is available when that slot is selected. The Overview and Worlds sections are separate route sections, so the action needs to pass the active slot and requested action across navigation, then have Worlds select that slot before starting the corresponding existing flow. For changing the thumbnail, reuse the existing picker/upload sequence rather than creating a second upload implementation; for viewing the map, reuse `WorldMapViewer` rather than opening a separate map instance.

**User outcome required:** Make the Active World thumbnail a clear, keyboard-accessible action target. Activating it should show the blue selected perimeter and an anchored menu with **Change Thumbnail** and **View Map**. Choosing either action should navigate to Worlds and target the active world. **Change Thumbnail** should open the same native file picker and upload flow as that world card’s **Set Thumbnail** action. **View Map** should select the active slot and open its existing map viewer. Preserve the existing map behavior and thumbnail upload safeguards.

**Implementation work (2026-10-21):** Made the Overview thumbnail a keyboard-accessible button with the blue selection perimeter while its action menu is open. The menu offers **Change Thumbnail** and **View Map**. A small navigation request carries the host, server, and active world across the lazy-loaded tab switch; Worlds selects that same active slot, then starts the existing native thumbnail picker/upload path or opens the existing map viewer. The thumbnail flow still uses the per-world staged upload and cleanup safeguards.

**P16.108 — Add Active World thumbnail actions**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`; `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`; `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`; `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`; `clients/desktop-web/src/lib/sections/worlds/world-navigation.ts`; `docs/msc2/rolling-plan.md`.
- **What:** Add an accessible action menu to the Overview Active World thumbnail and route its thumbnail and map actions to the matching active world in Worlds, reusing the existing workflows.
- **Verify:** On the next macOS build, open Overview for Java and Bedrock servers. Activate the Active World thumbnail by mouse and keyboard; confirm its blue perimeter and the **Change Thumbnail** / **View Map** menu. Choose **Change Thumbnail** and confirm Worlds selects that world and opens the existing image picker; cancel the picker. Return to Overview, choose **View Map**, and confirm Worlds opens that world’s existing map viewer.
- **Batch:** P16.108 only.

### Finding 14 — Select the active world automatically when opening Worlds

**What Cameron experienced and wants:** When he opens the **Worlds** tab, the active world card should already be selected. That way the related **Datapacks** section and **Backups** panel are immediately showing the active world’s information, without requiring an extra click on the card first. This should happen for every server type, including Java and Bedrock.

**Investigation:** `WorldsSection.svelte` initializes `selectedSlotId` as undefined. Loading the world list fills `worlds.activeSlotId` but does not copy that value into the selection state. The selected slot drives the per-world details below the cards: the datapack or Bedrock pack section and the selected world’s backups. A fresh visit can therefore show the active badge while those sections remain empty until the user clicks a card. Refreshes and server changes also need to distinguish a new server’s active slot from a user’s deliberate selection of another existing slot. Recommendation: select the active slot when data first loads or the active world changes; preserve a deliberate selection through ordinary refreshes while that slot still exists. Clicking an already-selected card should leave it selected so the details do not silently disappear.

**User outcome required:** On entering Worlds, select the active slot by default so its card has the selected outline and its backups and datapacks/world packs appear immediately. Apply this consistently to Java and Bedrock servers. On a server change or active-world change, select that server’s current active slot; if the selected slot disappears during refresh, fall back to the active slot. Preserve a user’s deliberate selection of another existing slot through routine refreshes. If a server has no active slot, keep the initial no-selection empty state. Clicking a selected card should keep it selected.

**Implementation work (2026-10-21):** World data now initializes the selected slot from `activeSlotId` when it loads for a host/server, and selection is reset while switching scope so another server’s selection cannot linger. Refreshes select the active slot when the active world changes or the current selection disappears, while preserving an existing deliberate selection otherwise. Clicking a selected card no longer clears it. These rules apply to Java and Bedrock and keep each slot’s backups and datapacks/world packs in view.

**P16.109 — Select the active world in Worlds**

- **Status:** Implementation recorded; awaiting owner verification on the next macOS build.
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Select the active world on load and scope changes, follow active-world changes or removal of the selected slot, preserve another deliberate selection during ordinary refreshes, and keep a selected card selected when clicked again.
- **Verify:** On the next macOS build, open Worlds for Java and Bedrock servers. Confirm the active card is selected and its Datapacks/World Packs and Backups are shown. Switch servers and confirm the newly selected server’s active world is selected. Select another slot, wait through a normal world refresh, and confirm that slot remains selected; then activate a different slot and confirm the new active world is selected. Click the selected card again and confirm its details stay visible. If available, open a server with no active world and confirm it begins with no selection.
- **Batch:** P16.109 only.

### Finding 15 — A new Bedrock world map has large holes despite a complete cache count

**What Cameron experienced:** While playing in a newly created Bedrock world, Cameron opened its 3D map and saw separated columns of trees and terrain with large empty gaps and sheer vertical walls between them. Some gaps are close to his live player marker even though he is standing on normal, rendered ground in Minecraft. The view did not visibly fill in as he waited. Clicking **Refresh terrain** changed the map, but it still looked patchy. **Pre-render map** reported “12 / 12 saved tiles cached” and “All saved terrain tiles are cached,” which made the missing ground especially confusing. On a newly created Paper server, joining the world produced a much more complete nearby map without this obvious pattern. Cameron expects Bedrock’s saved terrain around a player to appear with comparable coverage and responsiveness, or for MSC to explain accurately why it cannot yet show that terrain.

**Investigation (2026-10-21):** The Bedrock map reads a consistent *saved snapshot*, not the live terrain that the game client is drawing. The agent takes a Bedrock save hold/query/copy/resume snapshot when preparing the map, converts the saved LevelDB chunks into private Anvil region files, and keeps using that prepared generation until **Refresh terrain** captures another snapshot. The viewer polls for changes to the prepared map, but that polling does not capture a newer Bedrock save by itself. The pre-render count comes from the prepared renderer manifest's tile progress: it says all tiles *listed in that saved-terrain manifest* are cached, not that all terrain visible in Minecraft is present or that every saved chunk was converted. The first screenshot showed 12/12 tiles before the later, wider view.

The current private converted cache was inspected read-only. Its five Overworld region files contain 365 nonempty chunk positions in a sparse pattern, with missing positions among populated ones near Cameron's reported player location (approximately X -217, Z 120). Thus the conspicuous gaps are already present in the converted terrain input; waiting for those same tiles to draw will not fill them. The agent log shows initial conversion of four saved regions and later refreshes that rebuilt changed regions, so refresh is seeing changing saved input. This establishes that the count is misleading for coverage and that no automatic snapshot update is occurring. It does **not** yet establish whether the missing chunks are absent from the captured Bedrock LevelDB data or whether the conversion/indexing path omits saved chunks. The Paper comparison is Cameron's observed difference, not proof that Java captures live terrain automatically.

**User outcome required:** Compare chunk positions in a fresh, consistent Bedrock snapshot with the converted region files at the same location, particularly beside the player. If chunks are in the snapshot but not the converted output, repair conversion or indexing. If they are not in the snapshot, investigate why the Bedrock save boundary yields sparse terrain while the game shows ground, and improve snapshot freshness or the map's explanation accordingly. Make the map visibly distinguish “all known tiles cached” from “nearby saved terrain complete”; do not claim the latter from the pre-render count. Verify that newly explored Bedrock ground appears within a reasonable, stated interval or after an explicit refresh, and compare that behavior with Java on the same build. Preserve the source world and inspect snapshots read-only during diagnosis.

**P16.97 — Record the Bedrock map coverage finding**

- **Status:** Investigation recorded; cause and fix remain open.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Describe the owner-visible gaps, the limited meaning of the 12/12 cache count, the sparse converted output observed locally, and the remaining source-versus-conversion question.
- **Verify:** In the current Bedrock world, compare the in-game ground near the player with the 3D map before and after **Refresh terrain**; the gap remains open until the map reliably covers saved nearby ground and its cache status accurately describes that coverage.
- **Batch:** P16.97 only.

### Finding 16 — Java default game mode changes do not switch players who are already online

**What Cameron experienced:** On his Paper server, Cameron changed the game mode from Survival to Creative using MSC's sidebar and the active world's settings. The control showed the new choice, but his player did not switch modes while playing. He later stopped the server, started it again, and rejoined; only then did he see the new mode. He is not sure whether the restart was necessary, but the change did not feel immediate. Bedrock behaves the way he expects: changing the active world's game mode also switches everyone currently online, after the Bedrock-specific achievements warning when applicable. Cameron wants Java servers to switch all online players at once too, without an achievements warning. He also wants to understand what **Enforce Gamemode** does and whether it is needed to make that live change work on Paper.

**Investigation (2026-10-21):** The sidebar saves `gameplay.default-game-mode` to the active world's profile; the World Settings flow uses the same profile path. On a running Java server, the agent calls `apply_active_java_world_gameplay()`. That path sends `defaultgamemode <mode>` but does **not** send `gamemode <mode> @a`. Bedrock's corresponding path deliberately sends both commands; P12.215 added the second command to switch players already online. [Minecraft's Java 26.3 notes](https://feedback.minecraft.net/hc/en-us/articles/48913133328013-Minecraft-Java-Edition-26-3) distinguish the world default set by `defaultgamemode` from a player's current mode set by `gamemode`. Therefore Java supports an immediate change for online players; MSC simply has not wired it to this world-wide control. The Paper server inspected during this finding has `gamemode=creative` and `force-gamemode=true` saved in `server.properties`, consistent with the result seen after rejoining, but this does not prove the restart itself was required to change the world default.

**Enforce Gamemode explained:** [Paper's server.properties reference](https://docs.papermc.io/paper/reference/server-properties/) says `force-gamemode` makes players join in the default mode and resets their previous mode when they reconnect. It is a server-wide join policy, separate from immediately changing the mode of players who are online. MSC currently treats toggling it as restart-required. It should remain a separate choice: turning it on may be useful when a server owner wants returning players' personal modes replaced at every join, but it should not be a prerequisite for the requested live switch.

**User outcome required:** When the active Java world's default game mode changes from either the sidebar or World Settings while a Java server is running, save the world default and issue the appropriate `gamemode <mode> @a` command so all currently connected players switch immediately, matching the intended Bedrock experience. Keep **Enforce Gamemode** separate as the policy for future joins/rejoins; changing it need not itself switch current players. Report success only after the live command is accepted, and show a clear pending or failed state if it cannot be sent. Do not show Bedrock's achievement-loss notice for Java. Check this on Paper and other supported Java flavors, with multiple players online and with Enforce Gamemode both on and off; ensure the selected world default survives restart and applies as intended to later joiners.

**P16.98 — Record the Java live game mode finding**

- **Status:** Investigation recorded; live-player fix remains open.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Record the observed Java delay, the difference between world default and online player mode, and Enforce Gamemode's separate join-time purpose.
- **Verify:** On a running Paper server with two players online, change the active world's game mode in MSC; both players should switch immediately without restarting or reconnecting, and the world default should persist for future joins. Repeat with Enforce Gamemode on and off.
- **Batch:** P16.98 only.

**Implementation work (2026-10-22):** When the active Java world's game mode is changed while the server is running, MSC now sends `defaultgamemode` to retain the world default and then `gamemode <mode> @a` to switch players already online. The second command is sent only for a deliberate game mode change, not when gameplay settings are restored at startup, so starting a server does not override returning players' modes when Enforce Gamemode is off. The profile response reports the live change only if both commands are accepted by the server process; otherwise it reports that the saved choice could not be fully applied live, with the command error. Enforce Gamemode remains a separate join policy.

**P16.110 — Apply Java game mode changes to online players**

- **Status:** Implemented; awaiting owner verification on a running Java server.
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`; `crates/msc-agent/src/routes/worlds.rs`; `docs/msc2/rolling-plan.md`.
- **What:** Send the live all-player Java game mode command after updating the world default, only when the active world's mode is explicitly changed.
- **Verify:** On a running Paper server with two players online, change the active world's game mode in MSC; both players should switch immediately without restarting or reconnecting. Repeat with Enforce Gamemode on and off, then restart and check that the world default persists. The same flow should work on another supported Java flavor.
- **Batch:** P16.110 only.

### Finding 17 — Messages beneath the console cannot be dismissed

**What Cameron experienced:** After an attempted console action failed, MSC showed red text beneath the console input saying “server is not running.” That message stayed visible while Cameron continued using the app. In the screenshot, the Paper server now appears to be running and the console has startup output, yet the old red message is still pinned under the input. It looks like a current server problem even though it describes an earlier attempt. Cameron wants every transient message shown in that under-console position to disappear when he clicks it, so he can clear outdated notices without sending another command or closing the console.

**Investigation:** `ConsoleDock.svelte` stores command and relative-time failures in `sendError` and renders that string as a red paragraph below the input. It clears `sendError` only when another command or relative-time action begins. The message has no click or keyboard dismissal handler, and **Clear console** clears log lines but does not clear this separate error. The dock stays mounted while the selected host changes, so a prior error can remain visible after the server state or selected host changes. The screenshot is consistent with this persistence; it does not prove why the original command was rejected.

**User outcome required:** Let the user dismiss any transient message rendered beneath the docked console by activating the message itself with a mouse or keyboard. Use a semantic, accessible control styled to retain the current low-profile text treatment; clearing it should not remove actual console history. Ensure stale command errors do not carry into a different host's console or continue to imply a current failure after a successful later action. Check command failures and relative-time failures, including the case where the server state changes after the error appears.

**Implementation work (2026-10-22):** The under-console error is now a text-styled button inside a status region. Clicking it, or focusing it and pressing Enter or Space, clears the notice without clearing console lines. Changing hosts clears the notice and resets the dock's pending send generation. Command and relative-time requests now ignore late results from an older action or host, so an old failure cannot reappear after a later success. A new successful action also clears the previous notice.

**P16.111 — Dismiss transient console notices**

- **Status:** Implemented; awaiting owner verification in the next desktop build.
- **Files:** `clients/desktop-web/src/lib/components/ApplicationShell.svelte`; `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Make the error beneath the console input accessible and dismissible, and prevent stale async errors from carrying across actions or hosts.
- **Verify:** In the next desktop build, cause a command or relative-time action to fail, then click its red notice to dismiss it; repeat with keyboard focus and Enter or Space. Confirm console history remains. Cause another failure, then complete a later action or switch hosts, and confirm the old notice does not return.
- **Batch:** P16.111 only.

### Finding 18 — Java Creative console warning is misleading and blocks the command

**What Cameron experienced:** After entering a Java `/gamemode` command for Creative on his Paper server, the dock showed a long red message beneath the input: “Java Creative mode changes the world's advancement and command semantics. Java does not have Bedrock's permanent Xbox-achievement consequence.” Cameron questions whether this is true and wants to click the message to dismiss it. Presented as an error with no visible Continue or Confirm action, it does not tell him whether his command ran, what changed for his player, or what he should do next. The same message position is affected by Finding 17's persistent, non-dismissible notice behavior.

**Investigation:** The text comes from `JavaCreative` in `world_safety.rs`. The command route requires an acknowledgement token before dispatching Creative-changing Java commands and otherwise returns a `confirmation_required` response. `ConsoleDock.svelte` catches that response as a generic error and prints its message as `sendError`; it has no confirmation flow to resubmit the command with the token. For a two-token `gamemode creative` command, the safety classifier chooses the *world-scoped* `JavaCreative` warning; an explicitly targeted `gamemode creative <player>` gets a different player-scoped confirmation but is still unconfirmable in this dock. Thus the warning can stop the command from being sent at all.

The warning also overstates the consequence. [Minecraft's Java 26.3 notes](https://feedback.minecraft.net/hc/en-us/articles/48913133328013-Minecraft-Java-Edition-26-3) distinguish `gamemode` for a player's current mode from `defaultgamemode` for the world default. A Java player switching to Creative does not itself change the world's command permissions or permanently disable Java advancements. Particular gameplay actions may behave differently in Creative, but the current “world's advancement and command semantics” statement is not a sound description of this command. The comparison with Bedrock's permanent Xbox-achievement consequence is directionally correct, but irrelevant as a red failure message after an ordinary Java player-mode command.

**User outcome required:** Correct or remove the broad Java Creative warning so the copy accurately describes the specific action and does not imply a permanent advancement or command-permission change. Decide whether a Java player-mode command merits confirmation at all; if a confirmation remains, the dock must present an actionable confirmation and send the command after acknowledgement, rather than strand it as red error text. Distinguish player-targeted `gamemode` from world-default `defaultgamemode`, including commands entered without an explicit target from the server console. Give clear success or failure feedback. Any resulting under-console message must be click- and keyboard-dismissible as in Finding 17. Keep Bedrock's separate achievement warning where its permanent world consequence actually applies.

**P16.99 — Record console message and Creative warning findings**

- **Status:** Investigation recorded; dismissal behavior remains open.
- **Files:** `docs/msc2/rolling-plan.md`.
- **What:** Record the persistent under-console error, Cameron's click-to-dismiss expectation, and the misleading Java Creative confirmation that the dock cannot complete.
- **Verify:** Trigger a command error in the docked console, click the message beneath the input, and confirm it disappears without clearing console log lines. Repeat with keyboard activation, a relative-time error, and a server or host state change. On Java, check that `gamemode creative <player>` either runs directly or offers a working, accurate confirmation rather than only red text.
- **Batch:** P16.99 only.

**Implementation work (2026-10-22):** Java `gamemode creative <player>` and `gamemode creative @a` are now treated as player-mode commands and sent without an achievement or world-change warning. A Java `gamemode creative` command entered in the server console without a player is rejected with a specific explanation and an example; `defaultgamemode creative` remains the world-default command. The Java world-default confirmation now says only that Creative becomes the default and does not permanently disable Java advancements. The dock displays an actionable confirmation for commands that still require one, then sends the command with the agent's acknowledgement token and the originally selected server ID so a later server switch cannot redirect it. It reports that a command was *sent* rather than claiming Minecraft executed it, and shows failures separately; both messages can be dismissed with a click or keyboard activation. Bedrock's achievement confirmation remains in place. One focused assertion was added to the existing safety-classifier test because the prior regression trapped player-mode commands behind a world confirmation; it has no timing or environment dependency and should take under a millisecond. Cameron has not authorized running it.

**P16.112 — Correct Java Creative console commands and confirmation**

- **Status:** Implemented; awaiting owner verification on a running Java server.
- **Files:** `crates/msc-application/src/world_safety.rs`; `crates/msc-agent/src/routes/commands.rs`; `clients/desktop-web/src/lib/components/ApplicationShell.svelte`; `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Let targeted Java player-mode commands run directly, reject a missing player clearly, correct the world-default warning, and make required dock confirmations actionable with dismissible send feedback.
- **Verify:** On a running Paper server, enter `gamemode creative <online-player>` in the dock and confirm the player switches without a warning. Enter `gamemode creative` and confirm MSC explains that a player is required. Enter `defaultgamemode creative`, use **Send command** in the confirmation, and confirm the command is sent; cancel a second attempt and confirm it is not sent. Click or keyboard-dismiss the resulting notice. Repeat the Creative command on a Bedrock server and confirm its achievement warning still appears.
- **Batch:** P16.112 only.

### Finding 19 — Xbox Broadcast actions should match the Components menu pattern

**What Cameron experienced and wants:** On a server’s Components tab, MCXboxBroadcast has an always-visible Download button while plugins and the server JAR open an action menu when their row is clicked. The Java row also has a toggle beside the broadcast name. Cameron wants to click the broadcast row, see the same blue selection outline as other component rows, and use its menu to download the JAR or enable or disable Xbox Broadcast. The action should read **Disable** when that server has broadcast enabled and **Enable** when it is disabled. Cameron withdrew a later suggestion to replace the row’s Installed status with Enabled or Disabled or to hide the row when the JAR is absent; retain its existing status display and visibility.

**Investigation:** The old Java toggle called `/v1/broadcast/autostart`, which controls automatic start at the agent level. It did not change whether the selected server uses Xbox Broadcast. The server setting is `xboxBroadcastEnabled` and is changed through `/v1/servers/xbox-broadcast`; the separate agent-wide automatic-start setting remains in Broadcast settings. The JAR download endpoint already stores the helper in the shared agent cache, so moving its action into a menu does not change download or launch behavior.

**Implementation work (2026-10-22):** Both Java and Bedrock broadcast rows now open a Components-style action menu and receive the blue perimeter while selected. The menu contains a per-server Enable or Disable action and the existing shared-JAR download action. The Java row keeps Installed or Missing, and the Bedrock row keeps its Running or Stopped indicator. The obsolete agent-wide toggle and its extra status request were removed from this tab.

### Finding 20 — Handbook topic list stops halfway down the page

**What Cameron experienced and wants:** In the Server Handbook, the topic list on the left stops around the middle of the available page even while the article on the right continues below it. Cameron likes being able to scroll the topics and wants the list to reach down through the usable Handbook area. The scroll indicator should stay hidden.

**Investigation:** `HandbookBrowser.svelte` capped the topic list at `26rem`, regardless of window height or the article’s space. The desktop shell already hides scrollbars globally, so no scrollbar styling is needed. The Handbook can fill the available content panel and let its topic list and article scroll within that panel; the narrow single-column layout still needs its compact topic list and ordinary page scrolling.

**Implementation work (2026-10-22):** The Handbook panel now fills the available tab height. The fixed desktop list cap is gone; its list grows to the bottom of the panel and remains scrollable. The article scrolls alongside it, while the narrow layout keeps its short list and page flow. No scroll indicator was added.

**P16.113 — Align broadcast component actions and extend Handbook navigation**

- **Status:** Implemented; awaiting owner verification in the next desktop build.
- **Files:** `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`; `clients/desktop-web/src/lib/sections/handbook/HandbookBrowser.svelte`; `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Move Xbox Broadcast download and per-server enable/disable into the selected-row menu while preserving the row’s status text, and make the Handbook topic list use the full available panel height without showing a scrollbar.
- **Verify:** In the next desktop build, select a Java server and a Bedrock server in turn. Open Components, click MCXboxBroadcast, and confirm the blue outline and Download plus Enable or Disable menu actions; confirm the row’s Installed/Missing or Running/Stopped status remains. Toggle broadcast, reopen the menu, and confirm the opposite action appears. Open Handbook, scroll the left topics to the last entry, and confirm the list reaches the panel bottom without a visible scrollbar; confirm the article still scrolls and a narrow window remains usable.
- **Batch:** P16.113 only.

### Finding 21 — The Handbook needs a useful starting point and app-specific first-server guides

**What Cameron experienced and wants:** After installing MSC 2 and completing First-time Setup, a user can open the Handbook and face a long list of topics with no reminder that it is a reference they can return to later. Cameron wants a short MSC-themed message at the top explaining that they do not have to read everything now. He also wants the three Getting Started guides saved at the top of the contents by default, with the option to remove those favorites or save other topics without moving any topic out of its usual category. The existing first-server pages describe broad Minecraft server setup, but they do not walk someone through MSC 2's actual screens after a skipped or forgotten tour. A reader should be able to follow the Java, modded Java, or Bedrock creation path, open related Handbook topics in place, and use a Back button beside the page title to return to the guide.

**Investigation:** The Handbook catalog and article prose come from the Rust agent. `HandbookBrowser.svelte` rendered that catalog as a grouped list and displayed Markdown through a deliberately limited HTML renderer. The three Getting Started files in `content/help/handbook/` used generic steps. The current Add Server flow exposes Configure, Network, World, optional Add-ons, and Confirm; the Java path offers a runtime chooser, while Bedrock has no Java runtime or Add-ons step. Local browser storage can hold a user's favorites without changing the shared agent catalog. Handbook-only `help:` links can be recognized and routed through the existing topic selection function, leaving normal web links under the renderer's existing URL safety rules.

**Implementation work (2026-10-22):** Added the quiet opening message and a Favorites section above the original topic groups. The three first-server guides are favorites on first use; saving and removing topics updates local favorites while their original entries stay in place. Rewrote the standard Java, modded Java, and Bedrock guides around the MSC 2 creator, optional services, first start, and later management. Related-topic links now open inside the Handbook without reloading the desktop app. A history-based Back button sits to the right of each article title, and opening another article returns its reader to the top.

### Finding 22 — The Handbook contents moved above the article

**What Cameron experienced and wants:** While reviewing the Handbook in his macOS app, Cameron found that the table of contents had become a full-width block above the article. Its search field and topics occupied the top of the Handbook, and the article began beneath them. This differs from the intended view, where the scrollable contents stay in a left column beside the article. He wants that side-by-side layout restored while keeping the taller contents list from Finding 20 and without adding a visible scroll indicator.

**Investigation:** The Handbook's narrow-screen rule changed its two-column layout into a single column at a 720 CSS-pixel viewport width. The app's available content area can be narrower than the full macOS window because the main sidebar also occupies width. That responsive rule is consistent with the screenshot's stacked search and article. The in-progress banner and favorites need one explicit columns container so both stay aligned beneath the banner. The contents and article also need to be allowed to shrink within that container instead of forcing a wrap.

**Implementation work (2026-10-22):** The Handbook now uses an explicit full-width side-by-side container with a stable left contents column and a flexible article column. The single-column fallback starts only at a genuinely narrow 560 CSS-pixel viewport. Both columns retain independent scrolling at desktop widths, and the existing hidden-scrollbar behavior is unchanged. Cameron's installed build has not yet been updated with this correction.

**P16.114 — Make the Handbook a usable in-app reference**

- **Status:** Implemented; awaiting owner verification in the next desktop build.
- **Files:** `clients/desktop-web/src/lib/help/markdown.ts`; `clients/desktop-web/src/lib/sections/handbook/HandbookBrowser.svelte`; `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`; `content/help/handbook/first-server.md`; `content/help/handbook/first-modded-server.md`; `content/help/handbook/first-bedrock-server.md`; `docs/msc2/rolling-plan.md`.
- **What:** Add the opening message, persistent favorites, in-app related-topic links and Back navigation; rewrite the three first-server guides around MSC 2; restore the contents column beside the article at Cameron's desktop window size.
- **Verify:** In the next desktop build, open Handbook at Cameron's usual window size and confirm the contents list is on the left of the article, scrolls to its end, and shows no scroll indicator. Confirm the opening message and three default favorites; save and remove a topic and reopen Handbook to check persistence while the topic remains in its original category. Open each first-server guide, follow a related-topic link, then use Back to return without an app reload. Narrow the window enough to check the compact layout remains usable.
- **Batch:** P16.114 only.

### Finding 23 — Favorites controls crowd every Handbook title

**What Cameron experienced and wants:** With Favorites added to the Handbook, every entry in the left table of contents has a visible Save or Remove button. The same action repeats beside each title, taking space from topic names and making the contents list feel busy. Cameron wants the list to show just the titles. On a Mac, right-clicking or two-finger clicking a title should open the relevant Save or Remove action for that topic.

**Investigation:** `HandbookBrowser.svelte` rendered a second button in each favorite and category row. Both buttons called the same existing favorites toggle, which already saves the choice locally; changing how the action is opened does not require a new storage path. MSC 2 already has a compact, viewport-clamped `Menu` component for row actions. The browser's `contextmenu` event covers a mouse right-click and a trackpad secondary click, and the title button can also expose the action to keyboard users with Shift+F10 or the Menu key.

**Implementation work (2026-10-22):** Removed the repeated visible Save and Remove controls. A secondary click on any title opens a single action menu, labeled **Save to Favorites** or **Remove from Favorites** according to its current state. The same menu opens from the keyboard, with focus moved to its action. Normal clicking still opens the article, and topics stay in their original category after being saved.

**P16.115 — Put Handbook favorite actions in a title context menu**

- **Status:** Implemented; awaiting owner verification in the next desktop build.
- **Files:** `clients/desktop-web/src/lib/sections/handbook/HandbookBrowser.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Remove repeated Save and Remove buttons and use the existing action menu on a Handbook title's secondary click or keyboard context-menu shortcut.
- **Verify:** In the next desktop build, open Handbook and confirm titles have no adjacent Save or Remove buttons. Right-click or two-finger click an unsaved topic, choose **Save to Favorites**, and confirm it appears at the top while remaining in its category. Secondary-click it again, choose **Remove from Favorites**, and confirm it remains in the category. Repeat the menu with Shift+F10 while a title has keyboard focus.
- **Batch:** P16.115 only.

### Finding 24 — Router Guide entries repeat their names

**What Cameron experienced and wants:** In Router Guide's picker, each router row shows a main name and a second line with the brand or provider. For example, the list shows **ASUS Router** with **ASUS Router** again beneath it, and **Linksys / Velop** repeats the same words on both lines. Even when the wording differs slightly, such as **AT&T Gateway** followed by **AT&T**, the category and main name already give the reader enough context. Cameron wants each router listed once so the inventory is easier to scan.

**Investigation:** `RouterGuidePanel.svelte` renders `guide.displayName` on the first line, then chooses `providerDisplayName`, `deviceDisplayName`, or category for a second line. Some catalog records put the same product name in both fields; others put a provider name already present in the main label. The router guides are already grouped by device category, and search still uses the catalog data, so the secondary display line is unnecessary in the picker. The generic **I don't know my router** card has a distinct explanatory line and is not affected.

**Implementation work (2026-10-22):** Removed the secondary line from the shared router-row renderer, which covers browse entries and search results. Each row now shows its guide name once with its navigation chevron. The underlying catalog, search behavior, and guide content are unchanged.

**P16.116 — Show each router guide name once**

- **Status:** Implemented; awaiting owner verification in the next desktop build.
- **Files:** `clients/desktop-web/src/lib/sections/handbook/RouterGuidePanel.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Remove the redundant provider or device subtitle from Router Guide picker rows while keeping the distinct generic-guide explanation.
- **Verify:** In the next desktop build, open Router Guide and confirm entries such as ASUS Router and Linksys / Velop show one name each. Search for a router and confirm result rows also show one name and still open the correct guide.
- **Batch:** P16.116 only.

### Finding 25 — Port forwarding advice does not lead to the Router Guide

**What Cameron experienced and wants:** While reading **Port Forwarding & DuckDNS** in the Handbook, Cameron sees an explanation of ports and why router settings matter, but there is no direct way from that page to the guided router instructions. He wants a short statement near the top telling readers that help configuring their router is available, with a link they can click to open MSC 2's Router Guide. The reader should stay in the app instead of having to find the separate Guide switcher themselves.

**Investigation:** The article text lives in `content/help/handbook/port-forwarding-duckdns.md` and is served by the agent. The desktop Handbook renders that text with a limited Markdown renderer; it already recognizes in-app links to other Handbook topics, but not the Router Guide view. `HelpSection.svelte` already switches between Handbook and Router Guide without a reload. A dedicated internal link can reuse that view switch while keeping the article's guidance in the agent-owned content file.

**Implementation work (2026-10-22):** Added a brief question and **Open the Router Guide** link at the start of the port forwarding article. The Markdown renderer recognizes this exact internal destination, and the Handbook passes its click to the existing Router Guide view. The link opens the router picker in the same desktop screen; ordinary web links and other Handbook topic links keep their existing behavior.

**P16.117 — Link port forwarding advice to Router Guide**

- **Status:** Implemented; awaiting owner verification in the next desktop and agent build.
- **Files:** `content/help/handbook/port-forwarding-duckdns.md`; `clients/desktop-web/src/lib/help/markdown.ts`; `clients/desktop-web/src/lib/sections/handbook/HandbookBrowser.svelte`; `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`; `docs/msc2/rolling-plan.md`.
- **What:** Put a Router Guide link near the top of the Port Forwarding & DuckDNS article and route it to MSC 2's existing Router Guide picker without reloading the app.
- **Verify:** In a build with the updated desktop and agent content, open Handbook → Port Forwarding & DuckDNS. Click **Open the Router Guide** near the top and confirm Router Guide opens in the same window at its router picker. Switch back to Handbook and confirm the article is still available.
- **Batch:** P16.117 only.
