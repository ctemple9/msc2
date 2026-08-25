<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../components/ActionButton.svelte';
  import type { Capabilities } from '../navigation/types';
  import { getPlatform, openExternal, type PlatformKind } from '../platform';
  import type { ScreenApi } from '../sections/shared/types';
  import {
    ACCENT_PRESETS,
    applyAccent,
    saveAccent,
    storedAccent,
    type AccentChoice,
  } from '../styles/accent';

  export let compact = false;
  export let headingId = 'first-launch-title';
  export let api: ScreenApi | undefined = undefined;
  export let onComplete: () => void = () => {};

  const javaFamilies = [
    ['Paper', 'Plugin-based. Fast, stable, largest ecosystem.', '▣', '#67e8f9'],
    ['Purpur', 'Paper + extra gameplay tweaks. All Paper plugins work.', '✣', '#c084fc'],
    ['Vanilla', 'No plugins. Fully authentic Mojang experience.', '◆', '#9ca3af'],
    ['Fabric', 'Lightweight mods. Fast updates, great for optimization.', '✂', '#60a5fa'],
    ['Forge', 'Classic modding platform. Widest mod selection.', '⚒', '#fb923c'],
    ['NeoForge', 'Forge’s modern successor. More active development.', '⚒', '#2dd4bf'],
  ] as const;

  let selected = 'green';
  let customColor = '#22c85a';
  let setupPage = 0;
  let wantsJava = true;
  let wantsBedrock = false;
  let capabilities: Capabilities | null = null;
  let serversRoot = '';
  let javaPath = 'java';
  let rootStatus: 'checking' | 'ready' | 'unavailable' | 'unknown' = 'checking';
  let javaStatus: 'checking' | 'found' | 'not-found' | 'unavailable' | 'unknown' = 'checking';
  let javaRuntimes: Array<{ executablePath: string; majorVersion?: number }> = [];
  let rootPickerBusy = false;
  let javaPickerBusy = false;
  let xboxStatus: 'checking' | 'installed' | 'not-installed' | 'downloading' | 'unavailable' =
    'checking';
  let xboxFilename = '';
  let tailscaleStatus: 'unknown' | 'installed' | 'not-installed' | 'unavailable' = 'unknown';
  let tailscaleChecking = false;
  let platformKind: PlatformKind | null = null;
  let rootMessage = '';
  let javaMessage = '';
  let xboxMessage = '';
  let tailscaleMessage = '';

  $: bedrockAdvertisement = capabilities?.serverTypes?.bedrock;
  $: bedrockReady =
    bedrockAdvertisement?.runtime?.state === 'available' ||
    bedrockAdvertisement?.runtime?.state === 'provisioning_required' ||
    (bedrockAdvertisement?.runtime === undefined && bedrockAdvertisement?.supported === true);
  $: bedrockProvisioning = bedrockAdvertisement?.runtime?.state === 'provisioning_required';
  $: bedrockReason =
    bedrockAdvertisement?.runtime?.message ??
    (capabilities === null
      ? 'The selected agent has not reported Bedrock readiness yet.'
      : 'Bedrock is not available on this host.');
  $: optionalPage = setupPage === 3 || setupPage === 4 || setupPage === 5;

  onMount(() => {
    selected = storedAccent();
    if (selected.startsWith('#')) customColor = selected;
    applyAccent(selected);
    void getPlatform().then((platform) => (platformKind = platform.kind));
    if (typeof localStorage !== 'undefined') {
      try {
        const stored = JSON.parse(localStorage.getItem('msc.server-types') ?? '{}') as {
          java?: boolean;
          bedrock?: boolean;
        };
        if (typeof stored.java === 'boolean') wantsJava = stored.java;
        if (typeof stored.bedrock === 'boolean') wantsBedrock = stored.bedrock;
      } catch {
        // A damaged preference must never prevent setup from opening.
      }
    }
    void probeHost();
  });

  async function probeHost(): Promise<void> {
    if (!api) {
      serversRoot = '~/MinecraftServers';
      rootStatus = 'ready';
      javaStatus = 'found';
      return;
    }
    await Promise.all([probeCapabilities(), probeServersRoot(), probeJava(), probeXbox()]);
    updateTailscaleFromCapabilities();
  }

  async function probeCapabilities(): Promise<void> {
    try {
      capabilities = await api!.get<Capabilities>('/v1/capabilities');
    } catch {
      capabilities = null;
    }
  }

  async function probeServersRoot(): Promise<void> {
    try {
      const result = await api!.get<{ path: string }>('/v1/config/servers-root');
      serversRoot = result.path;
      rootStatus = result.path.trim() ? 'ready' : 'unavailable';
      rootMessage = result.path.trim() ? '' : 'The agent did not provide a servers folder.';
    } catch {
      rootStatus = 'unavailable';
      rootMessage = 'This agent cannot report its servers folder yet.';
    }
  }

  async function probeJava(pathToSave?: string): Promise<void> {
    javaStatus = 'checking';
    javaMessage = '';
    try {
      if (pathToSave !== undefined) {
        await api!.post('/v1/config/java-runtime', {
          executablePath: pathToSave === 'java' ? '' : pathToSave,
        });
      }
      const [config, detected] = await Promise.all([
        api!.get<{ executablePath?: string }>('/v1/config/java-runtime'),
        api!.get<{
          runtimes: Array<{ executablePath: string; majorVersion?: number }>;
        }>('/v1/java-runtimes'),
      ]);
      javaRuntimes = detected.runtimes ?? [];
      const configured = config.executablePath?.trim();
      if (configured) javaPath = configured;
      const usable =
        javaRuntimes.find(
          (runtime) =>
            (runtime.majorVersion ?? 0) >= 21 &&
            (!configured || runtime.executablePath === configured),
        ) ?? javaRuntimes.find((runtime) => (runtime.majorVersion ?? 0) >= 21);
      if (usable) {
        javaStatus = 'found';
        if (!configured) javaPath = usable.executablePath;
      } else {
        javaStatus = 'not-found';
        javaMessage = 'No Java 21 or later runtime was found on this host.';
      }
    } catch {
      javaStatus = 'unavailable';
      javaMessage = 'The selected agent could not verify Java.';
    }
  }

  async function probeXbox(): Promise<void> {
    try {
      const result = await api!.get<{
        installed: boolean;
        downloading?: boolean;
        filename?: string;
      }>('/v1/broadcast/jar-status');
      xboxStatus = result.downloading
        ? 'downloading'
        : result.installed
          ? 'installed'
          : 'not-installed';
      xboxFilename = result.filename ?? '';
    } catch {
      xboxStatus = 'unavailable';
    }
  }

  function updateTailscaleFromCapabilities(): void {
    const state = capabilities?.helpers?.tailscale;
    tailscaleStatus = state === true ? 'installed' : state === false ? 'not-installed' : 'unknown';
  }

  function openExternalLink(event: MouseEvent, url: string): void {
    if (platformKind !== 'tauri') return;
    event.preventDefault();
    void openExternal(url);
  }

  function chooseAccent(choice: AccentChoice): void {
    selected = choice.id;
    saveAccent(choice);
  }

  function chooseCustom(): void {
    selected = customColor;
    saveAccent(customColor);
  }

  function toggleServerType(type: 'java' | 'bedrock'): void {
    if (type === 'java') wantsJava = !wantsJava;
    else if (bedrockReady) wantsBedrock = !wantsBedrock;
  }

  async function chooseRoot(): Promise<void> {
    rootPickerBusy = true;
    rootMessage = '';
    try {
      const chosen = await (await getPlatform()).pickFolder('Choose your Minecraft servers folder');
      if (chosen) {
        serversRoot = chosen;
        rootStatus = 'unknown';
        rootMessage = 'Click Next to ask the agent to validate this folder.';
      }
    } catch {
      rootStatus = 'unavailable';
      rootMessage = 'The folder picker could not open. Enter an absolute path manually.';
    } finally {
      rootPickerBusy = false;
    }
  }

  async function browseJava(): Promise<void> {
    javaPickerBusy = true;
    javaMessage = '';
    try {
      const chosen = await (await getPlatform()).pickFilePath({ label: 'Choose Java executable' });
      if (chosen) {
        javaPath = chosen;
        await probeJava(chosen);
      }
    } catch {
      javaStatus = 'unavailable';
      javaMessage = 'The Java picker could not open. Enter the executable path manually.';
    } finally {
      javaPickerBusy = false;
    }
  }

  async function validateAndSaveHost(): Promise<void> {
    if (!api) {
      setupPage = 3;
      return;
    }
    rootStatus = 'checking';
    rootMessage = '';
    try {
      const result = await api.post<{ path: string }>('/v1/config/servers-root', {
        path: serversRoot,
      });
      serversRoot = result.path;
      rootStatus = 'ready';
      if (wantsJava && javaStatus !== 'found') await probeJava(javaPath);
      if (!wantsJava || javaStatus === 'found') setupPage = 3;
    } catch {
      rootStatus = 'unavailable';
      rootMessage = 'The agent could not save this folder. Use an absolute path it can access.';
    }
  }

  async function downloadXbox(): Promise<void> {
    if (!api || xboxStatus === 'installed') return;
    xboxStatus = 'downloading';
    xboxMessage = '';
    try {
      const result = await api.post<{ filename?: string }>('/v1/broadcast/download-jar');
      xboxFilename = result.filename ?? '';
      await probeXbox();
      xboxMessage =
        xboxStatus === 'installed'
          ? xboxFilename
            ? `Verified downloaded: ${xboxFilename}`
            : 'Verified downloaded and present in the agent helper cache.'
          : 'The download completed, but the agent could not verify the helper file.';
    } catch {
      xboxStatus = 'unavailable';
      xboxMessage = 'The helper can be downloaded after the agent is ready for broadcast access.';
    }
  }

  async function checkTailscale(): Promise<void> {
    if (!api || tailscaleChecking) return;
    tailscaleChecking = true;
    tailscaleStatus = 'unknown';
    tailscaleMessage = '';
    try {
      capabilities = await api.get<Capabilities>('/v1/capabilities');
      updateTailscaleFromCapabilities();
      if (tailscaleStatus === 'unknown') {
        tailscaleStatus = 'unavailable';
        tailscaleMessage = 'This agent does not advertise a Tailscale installation check.';
      }
    } catch {
      tailscaleStatus = 'unavailable';
      tailscaleMessage = 'The selected agent could not check Tailscale.';
    } finally {
      tailscaleChecking = false;
    }
  }

  function nextSetupPage(): void {
    if (setupPage === 0) {
      setupPage = 1;
      return;
    }
    if (setupPage === 1) {
      if (!wantsJava && !wantsBedrock) return;
      setupPage = 2;
      return;
    }
    if (setupPage === 2) {
      void validateAndSaveHost();
      return;
    }
    if (setupPage < 6) {
      setupPage += 1;
      return;
    }
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(
        'msc.server-types',
        JSON.stringify({ java: wantsJava, bedrock: wantsBedrock }),
      );
    }
    onComplete();
  }

  function skipOptional(): void {
    if (setupPage < 6) setupPage += 1;
  }
</script>

<div class="setup-intro" class:compact>
  {#if !compact}
    <header class="setup-heading">
      <div class="setup-track" aria-label={`Setup step ${setupPage + 1} of 7`}>
        {#each Array(7) as _, index}
          {#if index > 0}<span class:complete={index <= setupPage} class="track-line"></span>{/if}
          <span
            class:active={index === setupPage}
            class:complete={index < setupPage}
            class="track-dot"
          ></span>
        {/each}
      </div>
      <div class="setup-heading-body">
        <div class="setup-heading-icon" aria-hidden="true">
          {setupPage === 0
            ? '▤'
            : setupPage === 1
              ? '▣'
              : setupPage === 2
                ? '▰'
                : setupPage === 3
                  ? '⌁'
                  : setupPage === 4
                    ? '⌘'
                    : setupPage === 5
                      ? '◎'
                      : '✓'}
        </div>
        <div>
          <p class="eyebrow">First-time Setup</p>
          <h2 id={headingId}>
            {setupPage === 0
              ? 'First-time Setup'
              : setupPage === 1
                ? 'Server Type'
                : setupPage === 2
                  ? 'Server Setup'
                  : setupPage === 3
                    ? 'playit.gg'
                    : setupPage === 4
                      ? 'Xbox Broadcast'
                      : setupPage === 5
                        ? 'Tailscale'
                        : 'You’re All Set'}
          </h2>
          <p class="setup-subtitle">
            {setupPage === 0
              ? 'Let’s get Minecraft Server Controller configured.'
              : setupPage === 1
                ? 'Choose which platform you’ll host servers on.'
                : setupPage === 2
                  ? 'Where your servers live and how to run them.'
                  : setupPage === 3
                    ? 'Optional · Let friends join without port forwarding.'
                    : setupPage === 4
                      ? 'Optional · Console players see your server in Friends.'
                      : setupPage === 5
                        ? 'Optional · Remote access from any network.'
                        : 'Create your first server to get started.'}
          </p>
        </div>
      </div>
    </header>
  {:else}
    <h3 id={headingId}>{setupPage === 6 ? 'You’re All Set' : 'First-time Setup'}</h3>
    <p class="setup-subtitle">Continue through the setup steps to configure this host.</p>
  {/if}

  {#key setupPage}
    <div class="setup-page">
      {#if setupPage === 0}
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon blue" aria-hidden="true">▤</span>
            <div>
              <h3>What is Minecraft Server Controller?</h3>
              <p>MSC helps you run and manage Minecraft servers on your computer.</p>
            </div>
          </div>
          <ul>
            <li>
              <span class="feature-icon green" aria-hidden="true">▶</span>Start and stop Java and
              Bedrock servers with one click
            </li>
            <li>
              <span class="feature-icon blue" aria-hidden="true">●●●</span>Invite friends via
              tunnels, port forwarding, or Tailscale
            </li>
            <li>
              <span class="feature-icon purple" aria-hidden="true">◆</span>Install plugins, mods,
              and resource packs from Modrinth
            </li>
            <li>
              <span class="feature-icon orange" aria-hidden="true">▰</span>Schedule backups and
              manage multiple worlds
            </li>
          </ul>
        </section>
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon green" aria-hidden="true">✿</span>
            <div>
              <h3>Pick an Accent Color</h3>
              <p>Tints the app shell and overlays. Change it anytime in Preferences.</p>
            </div>
          </div>
          <div class="accent-choices" aria-label="Accent colors">
            {#each ACCENT_PRESETS as choice (choice.id)}
              <button
                class="accent-choice"
                class:selected={selected === choice.id}
                type="button"
                aria-label={choice.label}
                aria-pressed={selected === choice.id}
                style={`--choice-color: ${choice.color}`}
                onclick={() => chooseAccent(choice)}
                >{#if selected === choice.id}<span aria-hidden="true">✓</span>{/if}</button
              >
            {/each}
            <label class="custom-accent" title="Pick a custom accent color"
              ><input
                aria-label="Custom accent color"
                type="color"
                bind:value={customColor}
                oninput={chooseCustom}
              /><span aria-hidden="true">+</span></label
            >
          </div>
        </section>
      {:else if setupPage === 1}
        <section class="server-type-page">
          <div class="type-grid">
            <button
              class="type-card java"
              class:on={wantsJava}
              type="button"
              aria-pressed={wantsJava}
              onclick={() => toggleServerType('java')}
              ><span class="type-icon">☕</span><span
                ><strong>Java Servers</strong><small>Plugins, mods &amp; crossplay</small></span
              ><span class="type-check">{wantsJava ? '✓' : '○'}</span></button
            >
            <button
              class="type-card bedrock"
              class:on={wantsBedrock}
              class:disabled={!bedrockReady}
              type="button"
              aria-pressed={wantsBedrock}
              disabled={!bedrockReady}
              onclick={() => toggleServerType('bedrock')}
              ><span class="type-icon">◆</span><span
                ><strong>Bedrock Servers</strong><small
                  >{bedrockReady
                    ? bedrockProvisioning
                      ? 'Built in · prepared on first use'
                      : 'Mobile, console &amp; Windows'
                    : 'Unavailable on this host'}</small
                ></span
              ><span class="type-check">{wantsBedrock ? '✓' : '○'}</span></button
            >
          </div>
          {#if !bedrockReady}<p class="selection-warning">{bedrockReason}</p>{/if}
          {#if !wantsJava && !wantsBedrock}<p class="selection-warning">
              Select at least one type to continue. You can change this later.
            </p>{/if}
          {#if wantsJava}
            {#if wantsBedrock}<p class="family-label">JAVA</p>{/if}
            <div class="family-list">
              {#each javaFamilies as family, index}<div
                  class="family-row"
                  style={`--row-index: ${index}`}
                >
                  <span class="family-icon" style={`color: ${family[3]}`}>{family[2]}</span><strong
                    >{family[0]}</strong
                  ><span class="family-separator">·</span><span>{family[1]}</span>
                </div>{/each}
            </div>
            <p class="crossplay-note">
              <span>●●●</span> Java Edition players always. Bedrock, mobile, and console can join standard
              servers via Geyser crossplay (set up per server).
            </p>
          {/if}
          {#if wantsBedrock}<p class="family-label">BEDROCK</p>
            <div class="family-row bedrock-row">
              <span class="family-icon" style="color:#4ade80">▥</span><strong>BDS</strong><span
                class="family-separator">·</span
              ><span>Official Mojang Bedrock server. Runs in a built-in VM, no Docker needed.</span>
            </div>
            <p class="crossplay-note bedrock-note">
              <span>●●●</span> Mobile (iOS/Android), console (Xbox, PlayStation, Switch), and Windows
              Bedrock Edition. Java Edition players cannot join.
            </p>{/if}
        </section>
      {:else if setupPage === 2}
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon blue">▰</span>
            <div>
              <h3>Servers Root Folder</h3>
              <p>All your servers will live inside this folder.</p>
            </div>
          </div>
          <div class="field-row">
            <span class:ok={rootStatus === 'ready'} class="status-dot"
              >{rootStatus === 'ready' ? '✓' : '·'}</span
            ><input
              aria-label="Servers root folder"
              value={serversRoot}
              oninput={(event) => {
                serversRoot = event.currentTarget.value;
                rootStatus = 'unknown';
              }}
            /><button type="button" disabled={rootPickerBusy} onclick={() => void chooseRoot()}
              >{rootPickerBusy ? 'Choosing…' : 'Browse…'}</button
            >
          </div>
          {#if rootStatus === 'unavailable' || rootStatus === 'unknown'}<p
              class="inline-message warning"
            >
              {rootMessage || 'Enter an absolute folder path and let the agent validate it.'}
            </p>{/if}
        </section>
        {#if wantsJava}<section class="setup-card">
            <div class="card-heading">
              <span class="card-icon orange">☕</span>
              <div>
                <h3>Java Executable</h3>
                <p>
                  Java servers require JDK 21 or later. Point to your binary or let the agent find
                  it on PATH.
                </p>
              </div>
            </div>
            <div class="field-row">
              <span class:ok={javaStatus === 'found'} class="status-dot"
                >{javaStatus === 'found' ? '✓' : '·'}</span
              ><input
                aria-label="Java executable"
                value={javaPath}
                oninput={(event) => {
                  javaPath = event.currentTarget.value;
                  javaStatus = 'unknown';
                }}
              /><button
                type="button"
                disabled={javaPickerBusy || javaStatus === 'checking'}
                onclick={() => void browseJava()}>{javaPickerBusy ? 'Choosing…' : 'Browse…'}</button
              ><button
                type="button"
                disabled={javaPickerBusy || javaStatus === 'checking'}
                onclick={() => void probeJava(javaPath)}
                >{javaStatus === 'checking' ? 'Checking…' : 'Check for Java'}</button
              ><button
                type="button"
                disabled={javaPickerBusy || javaStatus === 'checking'}
                onclick={() => {
                  javaPath = 'java';
                  void probeJava('java');
                }}>Use PATH</button
              >
            </div>
            <p class="probe-status" class:success={javaStatus === 'found'}>
              {javaStatus === 'checking'
                ? 'Checking…'
                : javaStatus === 'found'
                  ? `Found at ${javaPath}`
                  : javaMessage || (javaStatus === 'not-found' ? 'Not found' : 'Not checked yet')}
            </p>
            {#if javaStatus === 'not-found'}<p class="inline-message warning">
                Install the current Temurin LTS or choose an existing JDK 21+ executable, then check
                again.
              </p>{/if}
          </section>{/if}
        {#if wantsBedrock}<section class="setup-card">
            <div class="card-heading">
              <span class="card-icon green">▥</span>
              <div>
                <h3>Bedrock — Built In</h3>
                <p>
                  No extra software needed. MSC runs Bedrock Dedicated Server in a built-in virtual
                  machine.
                </p>
              </div>
            </div>
            <p class="inline-message success">
              ✓ {bedrockProvisioning
                ? 'Ready. The built-in runtime will be prepared when you create your first Bedrock server.'
                : 'Ready. The selected agent reports a usable Bedrock runtime.'}
            </p>
          </section>{/if}
      {:else if setupPage === 3}
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon purple">⌁</span>
            <div>
              <h3>What is playit.gg?</h3>
              <p>
                A free tunneling service that lets friends connect to your server without port
                forwarding.
              </p>
            </div>
          </div>
          <ul>
            <li><span class="feature-icon green">✓</span>No router configuration required</li>
            <li>
              <span class="feature-icon green">✓</span>Works on any network, including strict NAT
            </li>
            <li>
              <span class="feature-icon green">✓</span>MSC sets up tunnels automatically after you
              sign in
            </li>
          </ul>
        </section>
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon purple">♙</span>
            <div>
              <h3>Create a playit.gg Account</h3>
              <p>
                A free account is required. MSC will handle tunnel setup after you’ve signed in.
              </p>
            </div>
          </div>
          <a
            href="https://playit.gg/login"
            target="_blank"
            rel="noreferrer"
            onclick={(event) => openExternalLink(event, 'https://playit.gg/login')}
            >Sign up at playit.gg →</a
          >
          <p class="inline-message warning">
            Free accounts include 1 agent and up to 3 tunnels. You can sign in after your first
            server is created.
          </p>
        </section>
        <p class="setup-note">You can skip this step and set it up later.</p>
      {:else if setupPage === 4}
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon green">⌘</span>
            <div>
              <h3>What is Xbox Broadcast?</h3>
              <p>The most reliable way for console players to find and join your server.</p>
            </div>
          </div>
          <ul>
            <li>
              <span class="feature-icon green">▰</span>Console players see your server in the Xbox
              Friends tab — no IP address needed
            </li>
            <li>
              <span class="feature-icon green">▯</span>Works for Java servers (via Geyser) and
              Bedrock servers
            </li>
            <li>
              <span class="feature-icon blue">↓</span>MSC downloads the broadcast tool automatically
            </li>
          </ul>
        </section>
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon orange">!</span>
            <div>
              <h3>Use a Dedicated Microsoft Account</h3>
              <p>
                We recommend not using your personal Microsoft or Xbox account for broadcasting.
              </p>
            </div>
          </div>
          <p>
            Xbox Broadcast may be against Microsoft’s Terms of Service per its own GitHub
            repository. Use a separate account to keep your personal account safe.
          </p>
          <p>
            Creating a new Outlook account gives you a fresh Xbox Live identity — free and takes
            under a minute.
          </p>
          <a
            class="orange-link"
            href="https://signup.live.com"
            target="_blank"
            rel="noreferrer"
            onclick={(event) => openExternalLink(event, 'https://signup.live.com')}
            >Create a new Microsoft / Outlook account →</a
          >
        </section>
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon blue">↓</span>
            <div>
              <h3>Broadcast Helper</h3>
              <p>The broadcast tool is downloaded once and shared across all your servers.</p>
            </div>
          </div>
          <div class="helper-row">
            <span class:success={xboxStatus === 'installed'}
              >{xboxStatus === 'installed'
                ? '✓ Installed and ready.'
                : xboxStatus === 'downloading'
                  ? 'Downloading…'
                  : xboxStatus === 'not-installed'
                    ? 'Not downloaded yet'
                    : 'Unavailable'}</span
            ><button
              type="button"
              onclick={() => void downloadXbox()}
              disabled={xboxStatus === 'downloading' || xboxStatus === 'installed'}
              >Download Now</button
            >
          </div>
          {#if xboxFilename}<p class="probe-status success">Verified file: {xboxFilename}</p>{/if}
          {#if xboxMessage}<p class="inline-message warning">{xboxMessage}</p>{/if}
        </section>
        <p class="inline-message info">
          When you first start a server with Xbox Broadcast enabled, MSC will prompt you to sign in
          with your Microsoft account in a private session.
        </p>
      {:else if setupPage === 5}
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon blue">◎</span>
            <div>
              <h3>What is Tailscale?</h3>
              <p>A private mesh VPN that connects your devices no matter where they are.</p>
            </div>
          </div>
          <ul>
            <li>
              <span class="feature-icon green">✓</span>Access your host’s servers from your phone,
              another computer, or anywhere
            </li>
            <li>
              <span class="feature-icon green">✓</span>Free for personal use — takes about a minute
              to set up
            </li>
            <li>
              <span class="feature-icon green">✓</span>Works alongside playit.gg — they solve
              different problems
            </li>
          </ul>
        </section>
        <section class="setup-card">
          <div class="card-heading">
            <span class="card-icon blue">◎</span>
            <div>
              <h3>Tailscale · Optional</h3>
              <p>Check whether Tailscale is already installed.</p>
            </div>
          </div>
          <div class="helper-row">
            <span class:success={tailscaleStatus === 'installed'}
              >{tailscaleChecking
                ? 'Checking…'
                : tailscaleStatus === 'installed'
                  ? '✓ Installed'
                  : tailscaleStatus === 'not-installed'
                    ? 'Not installed'
                    : tailscaleStatus === 'unavailable'
                      ? 'Check unavailable'
                      : 'Not checked yet'}</span
            ><button
              type="button"
              disabled={tailscaleChecking}
              onclick={() => void checkTailscale()}
              >{tailscaleChecking ? 'Checking…' : 'Check'}</button
            >
          </div>
          {#if tailscaleStatus === 'not-installed'}<p class="inline-message info">
              Tailscale isn’t installed. <a
                href="https://tailscale.com/download"
                target="_blank"
                rel="noreferrer"
                onclick={(event) => openExternalLink(event, 'https://tailscale.com/download')}
                >Download it free from tailscale.com →</a
              >
            </p>{:else if tailscaleStatus === 'installed'}<p class="inline-message success">
              Tailscale is installed. Enable it and join your tailnet to access servers remotely.
            </p>{:else if tailscaleMessage}<p class="inline-message warning">
              {tailscaleMessage}
            </p>{/if}
        </section>
      {:else}
        <section class="done-page">
          <div class="done-check">✓</div>
          <h3>You’re All Set</h3>
          <p>
            MSC is configured and ready. Click “Get Started” to create your first Minecraft server.
          </p>
          <div class="summary-card">
            <div>
              <span class="summary-icon blue">▰</span><strong>Servers root</strong><code
                >{serversRoot || 'Not set'}</code
              >
            </div>
            {#if wantsJava}<div>
                <span class="summary-icon orange">☕</span><strong>Java</strong><code
                  >{javaPath || 'Not configured'}</code
                >
              </div>{/if}
            <div>
              <span class="summary-icon purple">▣</span><strong>Server types</strong><code
                >{[wantsJava ? 'Java' : null, wantsBedrock ? 'Bedrock' : null]
                  .filter(Boolean)
                  .join(' + ')}</code
              >
            </div>
          </div>
        </section>
      {/if}
    </div>
  {/key}

  {#if setupPage === 0}<p class="setup-time">This setup takes about 2 minutes.</p>{/if}
  <div class="setup-actions">
    {#if setupPage > 0 && setupPage < 6}<ActionButton
        kind="quiet"
        label="Back"
        onclick={() => (setupPage -= 1)}>‹ Back</ActionButton
      >{/if}
    <span class="action-spacer"></span>
    {#if optionalPage}<ActionButton kind="quiet" label="Skip" onclick={skipOptional}
        >Skip</ActionButton
      >{/if}
    <ActionButton
      label={setupPage === 6 ? 'Get Started' : setupPage === 5 ? 'Continue' : 'Next'}
      disabled={setupPage === 1 && !wantsJava && !wantsBedrock}
      onclick={nextSetupPage}
      >{setupPage === 6
        ? 'Get Started'
        : setupPage === 5
          ? 'Continue'
          : 'Next'}{#if setupPage < 6}<span aria-hidden="true"> →</span>{/if}</ActionButton
    >
  </div>
</div>

<style>
  .setup-intro {
    overflow: hidden;
    margin: -1.5rem;
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface-raised);
  }
  .setup-heading {
    display: grid;
    gap: 1rem;
    padding: 1.5rem;
    color: white;
    background: linear-gradient(135deg, var(--msc-accent), #19723a);
  }
  .setup-heading-body {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .setup-track {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .track-dot {
    width: 0.4rem;
    height: 0.4rem;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.25);
    transition: all 180ms ease;
  }
  .track-dot.active {
    width: 0.6rem;
    height: 0.6rem;
    background: var(--msc-accent-strong);
  }
  .track-dot.complete {
    background: rgba(255, 255, 255, 0.65);
  }
  .track-line {
    flex: 1;
    min-width: 0.9rem;
    height: 1px;
    background: rgba(255, 255, 255, 0.18);
  }
  .track-line.complete {
    background: var(--msc-accent-strong);
  }
  .setup-heading-icon,
  .card-icon,
  .summary-icon {
    display: grid;
    place-items: center;
    flex: 0 0 auto;
    border-radius: 0.75rem;
    font-weight: 900;
  }
  .setup-heading-icon {
    width: 3rem;
    height: 3rem;
    background: rgba(255, 255, 255, 0.16);
    font-size: 1.5rem;
  }
  .setup-heading h2,
  .setup-heading p,
  .compact h3,
  .compact > p {
    margin: 0;
  }
  .setup-heading h2 {
    color: white;
  }
  .setup-heading .eyebrow {
    color: rgba(255, 255, 255, 0.78);
  }
  .setup-subtitle {
    color: var(--msc-muted);
  }
  .setup-heading .setup-subtitle {
    color: rgba(255, 255, 255, 0.82);
  }
  .setup-page {
    animation: setup-page-in 260ms ease both;
  }
  @keyframes setup-page-in {
    from {
      opacity: 0;
      transform: translateX(1rem);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
  .setup-card {
    margin: 1.25rem 1.5rem 0;
    padding: 1rem;
    border-radius: var(--msc-radius-md);
    background: var(--msc-surface);
  }
  .card-heading {
    display: flex;
    gap: 0.7rem;
    align-items: flex-start;
  }
  .card-heading h3 {
    margin: 0;
    font-size: 1rem;
  }
  .card-heading p {
    margin: 0.2rem 0 0;
    color: var(--msc-muted);
    font-size: 0.85rem;
  }
  .card-heading > div {
    min-width: 0;
  }
  .card-icon {
    width: 2rem;
    height: 2rem;
    font-size: 1rem;
  }
  .blue {
    color: #60a5fa;
    background: rgba(59, 130, 246, 0.18);
  }
  .green {
    color: #4ade80;
    background: rgba(34, 197, 94, 0.18);
  }
  .purple {
    color: #c084fc;
    background: rgba(168, 85, 247, 0.18);
  }
  .orange {
    color: #fb923c;
    background: rgba(249, 115, 22, 0.18);
  }
  ul {
    display: grid;
    gap: 0.55rem;
    margin: 0.9rem 0 0;
    padding: 0;
    list-style: none;
    color: var(--msc-muted);
    font-size: 0.9rem;
  }
  li {
    display: flex;
    gap: 0.6rem;
    align-items: flex-start;
    line-height: 1.35;
  }
  .feature-icon {
    min-width: 1.1rem;
    font-size: 0.75rem;
    text-align: center;
  }
  .accent-choices {
    display: flex;
    flex-wrap: wrap;
    gap: 0.55rem;
    margin-top: 0.9rem;
  }
  .accent-choice,
  .custom-accent {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    border: 2px solid transparent;
    border-radius: 50%;
    color: white;
    background: var(--choice-color);
    cursor: pointer;
  }
  .accent-choice.selected {
    border-color: white;
    box-shadow: 0 0 0 2px var(--choice-color);
  }
  .custom-accent {
    position: relative;
    background: conic-gradient(#22c85a, #3b82f6, #8b5cf6, #ef4444, #22c85a);
  }
  .custom-accent input {
    position: absolute;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .type-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
  }
  .server-type-page {
    padding: 1.25rem 1.5rem 0;
  }
  .type-card {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    padding: 0.8rem;
    border: 1.5px solid transparent;
    border-radius: var(--msc-radius-md);
    color: var(--msc-text);
    background: var(--msc-surface);
    text-align: left;
    cursor: pointer;
  }
  .type-card.java.on {
    border-color: rgba(249, 115, 22, 0.7);
    background: rgba(249, 115, 22, 0.12);
  }
  .type-card.bedrock.on {
    border-color: rgba(34, 197, 94, 0.7);
    background: rgba(34, 197, 94, 0.12);
  }
  .type-card.disabled {
    cursor: not-allowed;
    opacity: 0.65;
  }
  .type-icon {
    display: grid;
    place-items: center;
    width: 2.25rem;
    height: 2.25rem;
    border-radius: 0.7rem;
    color: white;
    background: #f97316;
    font-size: 1.1rem;
  }
  .bedrock .type-icon {
    background: #22c55e;
  }
  .type-card strong,
  .type-card small {
    display: block;
  }
  .type-card small {
    margin-top: 0.15rem;
    color: var(--msc-muted);
    font-size: 0.78rem;
  }
  .type-check {
    margin-left: auto;
    color: var(--msc-accent);
    font-size: 1.1rem;
  }
  .selection-warning,
  .crossplay-note,
  .inline-message {
    margin: 0.75rem 0 0;
    padding: 0.7rem;
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
    background: rgba(59, 130, 246, 0.12);
    font-size: 0.8rem;
    line-height: 1.4;
  }
  .crossplay-note {
    background: rgba(59, 130, 246, 0.14);
  }
  .bedrock-note {
    background: rgba(34, 197, 94, 0.12);
  }
  .family-label {
    margin: 1rem 0 0.45rem;
    color: var(--msc-subtle);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.1em;
  }
  .family-list {
    display: grid;
    gap: 0.25rem;
  }
  .family-row {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-height: 2.55rem;
    padding: 0.55rem 0.7rem;
    border-radius: var(--msc-radius-sm);
    color: var(--msc-muted);
    background: rgba(232, 238, 242, 0.07);
    font-size: 0.82rem;
    animation: family-row-in 220ms ease both;
    animation-delay: calc(var(--row-index, 0) * 55ms);
  }
  .family-row strong {
    color: var(--msc-text);
  }
  .family-icon {
    width: 1.1rem;
    text-align: center;
  }
  .family-separator {
    color: var(--msc-subtle);
  }
  .bedrock-row {
    background: rgba(34, 197, 94, 0.1);
  }
  @keyframes family-row-in {
    from {
      opacity: 0;
      transform: translateY(0.35rem);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .field-row,
  .helper-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.9rem;
  }
  .field-row input {
    min-width: 0;
    flex: 1;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-sm);
    color: var(--msc-text);
    background: var(--msc-surface-raised);
    font:
      0.82rem ui-monospace,
      SFMono-Regular,
      Menlo,
      monospace;
  }
  .field-row button,
  .helper-row button {
    white-space: nowrap;
  }
  .status-dot {
    color: var(--msc-muted);
  }
  .status-dot.ok,
  .success {
    color: #4ade80;
  }
  .probe-status {
    margin: 0.55rem 0 0 1.55rem;
    color: var(--msc-muted);
    font-size: 0.82rem;
  }
  .inline-message.warning {
    background: rgba(249, 115, 22, 0.12);
  }
  .inline-message.success {
    background: rgba(34, 197, 94, 0.12);
  }
  .inline-message.info {
    background: rgba(59, 130, 246, 0.12);
  }
  .inline-message a,
  .setup-card a {
    color: var(--msc-accent);
    font-weight: 700;
  }
  .orange-link {
    color: #fb923c !important;
  }
  .setup-note,
  .setup-time {
    margin: 1rem 1.5rem;
    color: var(--msc-subtle);
    font-size: 0.85rem;
    text-align: center;
  }
  .done-page {
    display: grid;
    justify-items: center;
    gap: 0.75rem;
    padding: 2rem 1.5rem;
    text-align: center;
  }
  .done-page h3 {
    margin: 0;
    font-size: 1.45rem;
  }
  .done-page > p {
    max-width: 26rem;
    margin: 0;
    color: var(--msc-muted);
  }
  .done-check {
    display: grid;
    place-items: center;
    width: 5rem;
    height: 5rem;
    border-radius: 50%;
    color: white;
    background: var(--msc-accent);
    box-shadow: 0 0 0 1rem color-mix(in srgb, var(--msc-accent) 14%, transparent);
    font-size: 2.5rem;
  }
  .summary-card {
    display: grid;
    gap: 0.65rem;
    width: min(100%, 34rem);
    margin-top: 0.75rem;
    padding: 1rem;
    border-radius: var(--msc-radius-lg);
    background: var(--msc-surface);
    text-align: left;
  }
  .summary-card > div {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }
  .summary-icon {
    width: 1.35rem;
    height: 1.35rem;
    font-size: 0.75rem;
  }
  .summary-card code {
    overflow: hidden;
    margin-left: auto;
    color: var(--msc-muted);
    font-size: 0.78rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .setup-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 1rem 1.5rem 1.5rem;
  }
  .action-spacer {
    flex: 1;
  }
  .compact {
    margin: 0;
    overflow: visible;
    background: transparent;
  }
  .compact .setup-card,
  .compact .setup-time,
  .compact .setup-note {
    margin-inline: 0;
  }
  .compact .server-type-page {
    padding-inline: 0;
  }
  .compact .setup-actions {
    margin-inline: 0;
    margin-bottom: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .setup-page,
    .family-row {
      animation: none;
    }
    .track-dot,
    .track-line {
      transition: none;
    }
  }
  @media (max-width: 520px) {
    .setup-heading {
      padding: 1.1rem;
    }
    .type-grid {
      grid-template-columns: 1fr;
    }
    .setup-card,
    .setup-note,
    .setup-time,
    .setup-actions {
      margin-inline: 1rem;
    }
    .field-row {
      flex-wrap: wrap;
    }
    .field-row input {
      flex-basis: 100%;
      order: -1;
    }
    .summary-card code {
      max-width: 55%;
    }
  }
</style>
