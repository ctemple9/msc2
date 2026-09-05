<script lang="ts">
  import { onMount } from 'svelte';
  import Sheet from '../components/base/Sheet.svelte';
  import Button from '../components/base/Button.svelte';
  import Card from '../components/base/Card.svelte';
  import StatusDot from '../components/base/StatusDot.svelte';
  import type { Capabilities } from '../navigation/types';
  import { getPlatform, openExternal } from '../platform';
  import type { PlatformKind } from '../platform/types';
  import { errorMessage, mutate } from '../sections/shared/types';
  import type { Schema, ScreenApi } from '../sections/shared/types';
  import { pollOperation, serverEditorPaths } from '../sections/server-editor/model';
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
    ['Paper', 'Plugin-based. Fast, stable, largest ecosystem.'],
    ['Purpur', 'Paper + extra gameplay tweaks. All Paper plugins work.'],
    ['Vanilla', 'No plugins. Fully authentic Mojang experience.'],
    ['Fabric', 'Lightweight mods. Fast updates, great for optimization.'],
    ['Forge', 'Classic modding platform. Widest mod selection.'],
    ['NeoForge', 'Forge’s modern successor. More active development.'],
  ] as const;

  const javaInstallOptions = [
    {
      major: 25,
      title: 'Java 25',
      minecraftRange: 'Minecraft 26.1 (latest) and newer',
      recommended: true,
    },
    {
      major: 21,
      title: 'Java 21',
      minecraftRange: 'Minecraft 1.20.5 – 1.21.x',
      recommended: false,
    },
    { major: 17, title: 'Java 17', minecraftRange: 'Minecraft 1.17 – 1.20.4', recommended: false },
    { major: 8, title: 'Java 8', minecraftRange: 'Minecraft 1.16.5 and older', recommended: false },
  ] as const;

  const pageMeta = [
    { title: 'First-time Setup', subtitle: 'Let’s get Minecraft Server Controller configured.' },
    { title: 'Server Type', subtitle: 'Choose which platform you’ll host servers on.' },
    { title: 'Server Setup', subtitle: 'Where your servers live and how to run them.' },
    { title: 'playit.gg', subtitle: 'Optional · Let friends join without port forwarding.' },
    {
      title: 'Xbox Broadcast',
      subtitle: 'Optional · Console players see your server in Friends.',
    },
    { title: 'Tailscale', subtitle: 'Optional · Remote access from any network.' },
    { title: 'You’re All Set', subtitle: 'Create your first server to get started.' },
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
  let showJavaInstallPicker = false;
  let selectedJavaMajor: number = javaInstallOptions[0].major;
  let javaInstallBusy = false;
  let javaInstallStatusLine = '';
  let javaInstallFailure = '';
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
  let completionBusy = false;
  let completionMessage = '';
  let rootTone: 'ok' | 'warn' | 'error' = 'warn';
  let javaTone: 'ok' | 'warn' | 'error' = 'warn';
  let xboxTone: 'ok' | 'warn' | 'error' = 'warn';
  let tailscaleTone: 'ok' | 'warn' | 'error' = 'warn';

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
  $: rootTone = rootStatus === 'ready' ? 'ok' : rootStatus === 'unavailable' ? 'error' : 'warn';
  $: rootLabel =
    rootStatus === 'ready'
      ? 'Ready'
      : rootStatus === 'unavailable'
        ? 'Unavailable'
        : rootStatus === 'checking'
          ? 'Checking…'
          : 'Not verified';
  $: javaTone = javaStatus === 'found' ? 'ok' : javaStatus === 'checking' ? 'warn' : 'error';
  $: javaLabel =
    javaStatus === 'found'
      ? `Found at ${javaPath}`
      : javaStatus === 'checking'
        ? 'Checking…'
        : javaStatus === 'not-found'
          ? 'Not found'
          : 'Unavailable';
  $: xboxTone = xboxStatus === 'installed' ? 'ok' : xboxStatus === 'unavailable' ? 'error' : 'warn';
  $: xboxLabel =
    xboxStatus === 'installed'
      ? 'Installed'
      : xboxStatus === 'downloading'
        ? 'Downloading…'
        : xboxStatus === 'checking'
          ? 'Checking…'
          : xboxStatus === 'not-installed'
            ? 'Not downloaded'
            : 'Unavailable';
  $: tailscaleTone =
    tailscaleStatus === 'installed' ? 'ok' : tailscaleStatus === 'unavailable' ? 'error' : 'warn';
  $: tailscaleLabel = tailscaleChecking
    ? 'Checking…'
    : tailscaleStatus === 'installed'
      ? 'Installed'
      : tailscaleStatus === 'not-installed'
        ? 'Not installed'
        : tailscaleStatus === 'unavailable'
          ? 'Check unavailable'
          : 'Not checked';

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

  async function installJava(): Promise<void> {
    if (!api || javaInstallBusy) return;
    javaInstallBusy = true;
    javaInstallFailure = '';
    javaInstallStatusLine = 'Starting install…';
    try {
      const result = await mutate<Schema['JavaRuntimeInstallResultDTO']>(
        api,
        serverEditorPaths.javaRuntimeInstall,
        { major: selectedJavaMajor },
      );
      const operation = await pollOperation(api, result.operationId, (tick) => {
        javaInstallStatusLine = tick.statusLine ?? javaInstallStatusLine;
      });
      if (!operation || operation.state !== 'succeeded') {
        javaInstallFailure =
          operation?.error?.message ?? result.message ?? 'The install did not complete.';
        javaInstallBusy = false;
        return;
      }
      showJavaInstallPicker = false;
      javaInstallBusy = false;
      javaInstallStatusLine = '';
      await probeJava();
      if (javaStatus !== 'found') {
        javaMessage = 'Java was installed. Click Check for Java to refresh the detected runtimes.';
      }
    } catch (caught) {
      javaInstallFailure = errorMessage(caught);
      javaInstallBusy = false;
    }
  }

  function closeJavaInstallPicker(): void {
    if (javaInstallBusy) return;
    showJavaInstallPicker = false;
    javaInstallStatusLine = '';
    javaInstallFailure = '';
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
      const installed = xboxStatus === ('installed' as typeof xboxStatus);
      xboxMessage = installed
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
    if (tailscaleChecking) return;
    if (!api) {
      tailscaleStatus = 'unavailable';
      tailscaleMessage = 'Connect to an agent before checking Tailscale.';
      return;
    }
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

  async function finishHostSetup(): Promise<void> {
    if (!api || completionBusy) return;
    completionBusy = true;
    completionMessage = '';
    try {
      await api.post<{ complete: boolean }>('/v1/config/host-setup/complete');
      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(
          'msc.server-types',
          JSON.stringify({ java: wantsJava, bedrock: wantsBedrock }),
        );
      }
      onComplete();
    } catch {
      completionMessage = 'The agent could not finish host setup. Try again.';
    } finally {
      completionBusy = false;
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
    void finishHostSetup();
  }

  function skipOptional(): void {
    if (setupPage < 6) setupPage += 1;
  }
</script>

<div class="setup-intro" class:compact>
  {#if !compact}
    <header class="setup-header">
      <div class="step-track" aria-label={`Setup step ${setupPage + 1} of 7`}>
        {#each Array(7) as _, index}
          {#if index > 0}<span class="track-line" class:done={index <= setupPage}></span>{/if}
          <span class="track-dot" class:done={index < setupPage} class:current={index === setupPage}
          ></span>
        {/each}
      </div>
      <p class="msc2-type-overline">
        {setupPage === 6 ? 'Setup complete' : `Step ${setupPage + 1} of 7`}
      </p>
      <h2 id={headingId} class="setup-title">{pageMeta[setupPage].title}</h2>
      <p class="setup-subtitle">{pageMeta[setupPage].subtitle}</p>
    </header>
  {:else}
    <h3 id={headingId} class="setup-title compact-title">
      {setupPage === 6 ? 'You’re All Set' : 'First-time Setup'}
    </h3>
    <p class="setup-subtitle">Continue through the setup steps to configure this host.</p>
  {/if}

  {#key setupPage}
    <div class="setup-page">
      {#if setupPage === 0}
        <Card>
          <p class="card-title">What is Minecraft Server Controller?</p>
          <p class="card-desc">MSC helps you run and manage Minecraft servers on your computer.</p>
          <ul class="feature-list">
            <li>Start and stop Java and Bedrock servers with one click</li>
            <li>Invite friends via tunnels, port forwarding, or Tailscale</li>
            <li>Install plugins, mods, and resource packs from Modrinth</li>
            <li>Schedule backups and manage multiple worlds</li>
          </ul>
        </Card>
        <Card>
          <p class="card-title">Pick an Accent Color</p>
          <p class="card-desc">
            Tints the app shell and overlays. Change it anytime in Preferences.
          </p>
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
        </Card>
        <p class="setup-time">This setup takes about 2 minutes.</p>
      {:else if setupPage === 1}
        <div class="type-grid">
          <button
            class="type-card"
            class:on={wantsJava}
            type="button"
            aria-pressed={wantsJava}
            onclick={() => toggleServerType('java')}
          >
            <span class="type-text"
              ><strong>Java Servers</strong><small>Plugins, mods &amp; crossplay</small></span
            ><span class="type-check">{wantsJava ? '✓' : '○'}</span>
          </button>
          <button
            class="type-card"
            class:on={wantsBedrock}
            class:disabled={!bedrockReady}
            type="button"
            aria-pressed={wantsBedrock}
            disabled={!bedrockReady}
            onclick={() => toggleServerType('bedrock')}
          >
            <span class="type-text"
              ><strong>Bedrock Servers</strong><small
                >{bedrockReady
                  ? bedrockProvisioning
                    ? 'Built in · prepared on first use'
                    : 'Mobile, console & Windows'
                  : 'Unavailable on this host'}</small
              ></span
            ><span class="type-check">{wantsBedrock ? '✓' : '○'}</span>
          </button>
        </div>
        {#if !bedrockReady}<p class="hint">{bedrockReason}</p>{/if}
        {#if !wantsJava && !wantsBedrock}<p class="hint warn">
            Select at least one type to continue. You can change this later.
          </p>{/if}
        {#if wantsJava}
          <p class="msc2-type-overline">{wantsBedrock ? 'Java' : 'Java flavors'}</p>
          <Card padding="0">
            {#each javaFamilies as family, index}
              <div class="family-row" class:last={index === javaFamilies.length - 1}>
                <span class="family-name">{family[0]}</span>
                <span class="family-desc">{family[1]}</span>
              </div>
            {/each}
          </Card>
          <p class="hint">
            Java Edition players always. Bedrock, mobile, and console can join standard servers via
            Geyser crossplay (set up per server).
          </p>
        {/if}
        {#if wantsBedrock}
          <p class="msc2-type-overline">{wantsJava ? 'Bedrock' : 'Bedrock flavors'}</p>
          <Card padding="0">
            <div class="family-row last">
              <span class="family-name">BDS</span>
              <span class="family-desc"
                >Official Mojang Bedrock server. Runs in a built-in VM, no Docker needed.</span
              >
            </div>
          </Card>
          <p class="hint">
            Mobile (iOS/Android), console (Xbox, PlayStation, Switch), and Windows Bedrock Edition.
            Java Edition players cannot join.
          </p>
        {/if}
      {:else if setupPage === 2}
        <p class="msc2-type-overline">Servers Root Folder</p>
        <Card>
          <p class="card-desc">
            {platformKind === 'tauri'
              ? 'All your servers will live inside this folder.'
              : 'This path is on the computer running the selected agent, not on this browser device.'}
          </p>
          <div class="field-row">
            <input
              class="field-input"
              aria-label="Servers root folder"
              value={serversRoot}
              oninput={(event) => {
                serversRoot = event.currentTarget.value;
                rootStatus = 'unknown';
              }}
            />
            {#if platformKind === 'tauri'}
              <Button
                variant="secondary"
                size="sm"
                disabled={rootPickerBusy}
                onclick={() => void chooseRoot()}>{rootPickerBusy ? 'Choosing…' : 'Browse…'}</Button
              >
            {/if}
          </div>
          <StatusDot tone={rootTone} label={rootLabel} />
          {#if rootStatus === 'unavailable' || rootStatus === 'unknown'}<p class="hint">
              {rootMessage || 'Enter an absolute folder path and let the agent validate it.'}
            </p>{/if}
        </Card>
        {#if wantsJava}
          <p class="msc2-type-overline">Java Executable</p>
          <Card>
            <p class="card-desc">
              Java servers require JDK 21 or later. Point to your binary or let the agent find it on
              PATH.{platformKind === 'browser' ? ' This executable path is on the agent host.' : ''}
            </p>
            <div class="field-row">
              <input
                class="field-input"
                aria-label="Java executable"
                value={javaPath}
                oninput={(event) => {
                  javaPath = event.currentTarget.value;
                  javaStatus = 'unknown';
                }}
              />
              {#if platformKind === 'tauri'}
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={javaPickerBusy || javaStatus === 'checking'}
                  onclick={() => void browseJava()}
                  >{javaPickerBusy ? 'Choosing…' : 'Browse…'}</Button
                >
              {/if}
              <Button
                variant="secondary"
                size="sm"
                disabled={javaPickerBusy || javaStatus === 'checking'}
                onclick={() => void probeJava(javaPath)}
                >{javaStatus === 'checking' ? 'Checking…' : 'Check for Java'}</Button
              >
              <Button
                variant="secondary"
                size="sm"
                disabled={javaPickerBusy || javaStatus === 'checking'}
                onclick={() => {
                  javaPath = 'java';
                  void probeJava('java');
                }}>Use PATH</Button
              >
            </div>
            <div class="field-row">
              <Button
                variant="secondary"
                size="sm"
                disabled={!api || javaInstallBusy}
                onclick={() => (showJavaInstallPicker = true)}>Install Java…</Button
              >
            </div>
            <StatusDot tone={javaTone} label={javaLabel} />
            {#if javaStatus === 'not-found'}<p class="hint warn">
                {javaMessage} Install the current Temurin LTS or choose an existing JDK 21+ executable,
                then check again.
              </p>{:else if javaMessage}<p class="hint warn">{javaMessage}</p>{/if}
          </Card>
        {/if}
        {#if wantsBedrock}
          <p class="msc2-type-overline">Bedrock — Built In</p>
          <Card>
            <p class="card-desc">
              No extra software needed. MSC runs Bedrock Dedicated Server in a built-in virtual
              machine.
            </p>
            <StatusDot
              tone="ok"
              label={bedrockProvisioning ? 'Ready · prepared on first use' : 'Ready'}
            />
          </Card>
        {/if}
      {:else if setupPage === 3}
        <Card>
          <p class="card-title">What is playit.gg?</p>
          <p class="card-desc">
            A free tunneling service that lets friends connect to your server without port
            forwarding.
          </p>
          <ul class="feature-list">
            <li>No router configuration required</li>
            <li>Works on any network, including strict NAT</li>
            <li>MSC sets up tunnels automatically after you sign in</li>
          </ul>
        </Card>
        <Card>
          <p class="card-title">Create a playit.gg Account</p>
          <p class="card-desc">
            A free account is required. MSC will handle tunnel setup after you’ve signed in.
          </p>
          <a
            class="link-button"
            href="https://playit.gg/login"
            target="_blank"
            rel="noreferrer"
            onclick={(event) => openExternalLink(event, 'https://playit.gg/login')}
            >Sign up at playit.gg →</a
          >
          <p class="hint">
            Free accounts include 1 agent and up to 3 tunnels. You can sign in after your first
            server is created.
          </p>
        </Card>
        <p class="setup-note">You can skip this step and set it up later.</p>
      {:else if setupPage === 4}
        <Card>
          <p class="card-title">What is Xbox Broadcast?</p>
          <p class="card-desc">
            The most reliable way for console players to find and join your server.
          </p>
          <ul class="feature-list">
            <li>Console players see your server in the Xbox Friends tab — no IP address needed</li>
            <li>Works for Java servers (via Geyser) and Bedrock servers</li>
            <li>MSC downloads the broadcast tool automatically</li>
          </ul>
        </Card>
        <Card>
          <p class="card-title">Use a Dedicated Microsoft Account</p>
          <p class="card-desc">
            We recommend not using your personal Microsoft or Xbox account for broadcasting.
          </p>
          <p class="hint">
            Xbox Broadcast may be against Microsoft’s Terms of Service per its own GitHub
            repository. Use a separate account to keep your personal account safe. Creating a new
            Outlook account gives you a fresh Xbox Live identity — free and takes under a minute.
          </p>
          <a
            class="link-button"
            href="https://signup.live.com"
            target="_blank"
            rel="noreferrer"
            onclick={(event) => openExternalLink(event, 'https://signup.live.com')}
            >Create a new Microsoft / Outlook account →</a
          >
        </Card>
        <Card>
          <p class="card-title">Broadcast Helper</p>
          <p class="card-desc">
            The broadcast tool is downloaded once and shared across all your servers.
          </p>
          <div class="field-row">
            <StatusDot tone={xboxTone} label={xboxLabel} />
            <span class="action-spacer"></span>
            <Button
              variant="secondary"
              size="sm"
              onclick={() => void downloadXbox()}
              disabled={xboxStatus === 'downloading' || xboxStatus === 'installed'}
              >Download Now</Button
            >
          </div>
          {#if xboxFilename}<p class="hint">Verified file: {xboxFilename}</p>{/if}
          {#if xboxMessage}<p class="hint warn">{xboxMessage}</p>{/if}
        </Card>
        <p class="setup-note">
          When you first start a server with Xbox Broadcast enabled, MSC will prompt you to sign in
          with your Microsoft account in a private session.
        </p>
      {:else if setupPage === 5}
        <Card>
          <p class="card-title">What is Tailscale?</p>
          <p class="card-desc">
            A private mesh VPN that connects your devices no matter where they are.
          </p>
          <ul class="feature-list">
            <li>Access your host’s servers from your phone, another computer, or anywhere</li>
            <li>Free for personal use — takes about a minute to set up</li>
            <li>Works alongside playit.gg — they solve different problems</li>
          </ul>
        </Card>
        <Card>
          <p class="card-title">Tailscale · Optional</p>
          <p class="card-desc">Check whether Tailscale is already installed.</p>
          <div class="field-row">
            <StatusDot tone={tailscaleTone} label={tailscaleLabel} />
            <span class="action-spacer"></span>
            <Button
              variant="secondary"
              size="sm"
              disabled={tailscaleChecking}
              onclick={() => void checkTailscale()}
              >{tailscaleChecking ? 'Checking…' : 'Check'}</Button
            >
          </div>
          {#if tailscaleStatus === 'not-installed'}<p class="hint">
              Tailscale isn’t installed.
              <a
                class="link-inline"
                href="https://tailscale.com/download"
                target="_blank"
                rel="noreferrer"
                onclick={(event) => openExternalLink(event, 'https://tailscale.com/download')}
                >Download it free from tailscale.com →</a
              >
            </p>{:else if tailscaleStatus === 'installed'}<p class="hint">
              Tailscale is installed. Enable it and join your tailnet to access servers remotely.
            </p>{:else if tailscaleMessage}<p class="hint warn">{tailscaleMessage}</p>{/if}
        </Card>
      {:else}
        <div class="done-page">
          <div class="done-check" aria-hidden="true">✓</div>
          <h3>You’re All Set</h3>
          <p>
            MSC is configured and ready. Click “Get Started” to create your first Minecraft server.
          </p>
          <div class="summary-card">
            <Card padding="0">
              <div class="summary-row">
                <span class="summary-label">Servers root</span>
                <code class="summary-value">{serversRoot || 'Not set'}</code>
              </div>
              {#if wantsJava}
                <div class="summary-row">
                  <span class="summary-label">Java</span>
                  <code class="summary-value">{javaPath || 'Not configured'}</code>
                </div>
              {/if}
              <div class="summary-row last">
                <span class="summary-label">Server types</span>
                <code class="summary-value"
                  >{[wantsJava ? 'Java' : null, wantsBedrock ? 'Bedrock' : null]
                    .filter(Boolean)
                    .join(' + ')}</code
                >
              </div>
            </Card>
          </div>
        </div>
      {/if}
    </div>
  {/key}

  {#if showJavaInstallPicker}
    <Sheet
      title="Install Java"
      size="sm"
      onClose={javaInstallBusy ? undefined : closeJavaInstallPicker}
    >
      <p class="explain">
        Pick the version that matches your Minecraft version. MSC downloads and installs it for this
        host.
      </p>
      <div class="list" role="radiogroup" aria-label="Java version to install">
        {#each javaInstallOptions as option (option.major)}
          <button
            type="button"
            class="install-option"
            class:selected={selectedJavaMajor === option.major}
            disabled={javaInstallBusy}
            onclick={() => (selectedJavaMajor = option.major)}
          >
            <span class="runtime-info">
              <span class="runtime-heading">
                <span class="runtime-version">{option.title}</span>
                {#if option.recommended}<span class="tag">Recommended</span>{/if}
              </span>
              <span class="runtime-path">{option.minecraftRange}</span>
            </span>
          </button>
        {/each}
      </div>
      {#if javaInstallStatusLine && javaInstallBusy}<p class="explain">
          {javaInstallStatusLine}
        </p>{/if}
      {#if javaInstallFailure}<p class="explain warn">{javaInstallFailure}</p>{/if}
      <div class="footer">
        <Button variant="secondary" disabled={javaInstallBusy} onclick={closeJavaInstallPicker}
          >Close</Button
        >
        <Button
          variant="primary"
          disabled={javaInstallBusy || !api}
          onclick={() => void installJava()}>{javaInstallBusy ? 'Installing…' : 'Install'}</Button
        >
      </div>
    </Sheet>
  {/if}

  {#if completionMessage}<p class="hint warn" role="alert">{completionMessage}</p>{/if}
  <div class="setup-actions">
    {#if setupPage > 0 && setupPage < 6}
      <Button variant="secondary" onclick={() => (setupPage -= 1)}>Back</Button>
    {/if}
    <span class="action-spacer"></span>
    {#if optionalPage}
      <Button variant="secondary" onclick={skipOptional}>Skip</Button>
    {/if}
    <Button
      variant="primary"
      disabled={completionBusy || (setupPage === 1 && !wantsJava && !wantsBedrock)}
      onclick={nextSetupPage}
      >{setupPage === 6 && completionBusy
        ? 'Finishing…'
        : setupPage === 6
          ? 'Get Started'
          : setupPage === 5
            ? 'Continue'
            : 'Next'}</Button
    >
  </div>
</div>

<style>
  .setup-intro {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    margin: -1.5rem;
    border-radius: 20px;
    background: var(--msc2-tier-chrome);
  }
  .setup-header {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 20px 24px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .step-track {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 6px;
  }
  .track-dot {
    width: 6px;
    height: 6px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: var(--msc2-neutral-muted);
    transition: all 150ms ease;
  }
  .track-dot.done {
    background: rgba(255, 255, 255, 0.65);
  }
  .track-dot.current {
    width: 8px;
    height: 8px;
    background: var(--msc2-text-primary);
  }
  .track-line {
    flex: 1;
    min-width: 10px;
    height: 1px;
    background: var(--msc2-hairline);
  }
  .track-line.done {
    background: rgba(255, 255, 255, 0.4);
  }
  .setup-title,
  .setup-subtitle,
  .card-title,
  .card-desc {
    margin: 0;
  }
  .setup-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .compact-title {
    font-size: 15px;
  }
  .setup-subtitle {
    color: var(--msc2-text-secondary);
    font-size: 13px;
  }
  .setup-page {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 24px;
    overflow-y: auto;
    animation: setup-page-in 180ms ease both;
  }
  @keyframes setup-page-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .card-desc {
    margin-top: 3px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .feature-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 10px 0 0;
    padding: 0 0 0 16px;
    color: var(--msc2-text-secondary);
    font-size: 13px;
    line-height: 1.4;
  }
  .feature-list li::marker {
    color: var(--msc2-text-tertiary);
  }
  .accent-choices {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }
  .accent-choice,
  .custom-accent {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 2px solid transparent;
    border-radius: 50%;
    color: white;
    background: var(--choice-color);
    cursor: pointer;
  }
  .accent-choice.selected {
    border-color: rgba(255, 255, 255, 0.85);
  }
  .custom-accent {
    position: relative;
    background: conic-gradient(#22c85a, #3b82f6, #8b5cf6, #ef4444, #22c85a);
  }
  .custom-accent input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }
  .type-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }
  .type-card {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--msc2-hairline);
    border-radius: 10px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-content);
    text-align: left;
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease;
  }
  .type-card.on {
    border-color: rgba(255, 255, 255, 0.4);
    background: var(--msc2-neutral-elevated);
  }
  .type-card.disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
  .type-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .type-text strong {
    font-size: 13px;
    font-weight: 500;
  }
  .type-text small {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .type-check {
    margin-left: auto;
    color: var(--msc2-text-tertiary);
    font-size: 14px;
  }
  .type-card.on .type-check {
    color: var(--msc2-status-ok);
  }
  .family-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
    font-size: 12px;
  }
  .family-row.last {
    border-bottom: none;
  }
  .family-name {
    flex: 0 0 auto;
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .family-desc {
    color: var(--msc2-text-tertiary);
  }
  .field-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    flex-wrap: wrap;
  }
  .field-input {
    box-sizing: border-box;
    min-width: 0;
    flex: 1;
    font-family: inherit;
    font-size: 13px;
    color: #fff;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    padding: 7px 10px;
    outline: none;
  }
  .field-input:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }
  :global(.setup-page .status-dot) {
    margin-top: 10px;
  }
  .hint {
    margin: 8px 0 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }
  .explain {
    margin: 0 0 12px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.5;
  }
  .explain.warn {
    color: var(--msc2-status-warn);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .install-option {
    display: flex;
    width: 100%;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .install-option:hover,
  .install-option.selected {
    border-color: var(--msc2-hairline);
    background: var(--msc2-tier-content);
  }
  .runtime-info {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 3px;
  }
  .runtime-heading {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .runtime-version {
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 500;
  }
  .runtime-path {
    overflow: hidden;
    color: var(--msc2-text-tertiary);
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    color: var(--msc2-text-tertiary);
    font-size: 10px;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
  .link-button {
    display: inline-flex;
    align-items: center;
    margin-top: 10px;
    padding: 5px 12px;
    border: 1px solid var(--msc2-hairline);
    border-radius: 7px;
    color: rgba(255, 255, 255, 0.9);
    font-size: 12px;
    font-weight: 500;
    text-decoration: none;
  }
  .link-button:hover {
    background: rgba(255, 255, 255, 0.06);
  }
  .link-inline {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .setup-note,
  .setup-time {
    margin: 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    text-align: center;
  }
  .done-page {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 12px 0 4px;
    text-align: center;
  }
  .done-page h3 {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 18px;
    font-weight: 600;
  }
  .done-page > p {
    max-width: 26rem;
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 13px;
  }
  .done-check {
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-status-ok);
    font-size: 20px;
  }
  .summary-card {
    width: 100%;
    margin-top: 6px;
    text-align: left;
  }
  .summary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .summary-row.last {
    border-bottom: none;
  }
  .summary-label {
    color: var(--msc2-text-tertiary);
    font-size: 12px;
  }
  .summary-value {
    overflow: hidden;
    color: var(--msc2-text-secondary);
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .setup-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 24px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .action-spacer {
    flex: 1;
  }
  .compact {
    margin: 0;
    overflow: visible;
    border-radius: 0;
    background: transparent;
  }
  .compact .setup-page {
    padding: 12px 0 0;
    overflow-y: visible;
  }
  .compact .setup-actions {
    padding: 12px 0 0;
    border-top: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .setup-page {
      animation: none;
    }
    .track-dot,
    .track-line {
      transition: none;
    }
  }
  @media (max-width: 520px) {
    .setup-header,
    .setup-page,
    .setup-actions {
      padding-inline: 16px;
    }
    .type-grid {
      grid-template-columns: 1fr;
    }
    .field-row {
      flex-wrap: wrap;
    }
    .field-input {
      flex-basis: 100%;
      order: -1;
    }
  }
</style>
