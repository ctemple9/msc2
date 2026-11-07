<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Select from '../../components/base/Select.svelte';
  import {
    bytesLabel,
    errorMessage,
    dateLabel,
    type Schema,
    type ScreenApi,
  } from '../shared/types';
  import { contentRoot, contentOperation, type ContentSnapshot } from './content-model';
  import { reportTransferFailure } from '../shared/transfer-notices';
  export let api: ScreenApi;
  export let serverId: string;
  export let onClose: () => void;
  interface File {
    path: string;
    sha512: string;
    byteSize: number;
    optional: boolean;
    enabled: boolean;
  }
  interface Issue {
    audience: string;
    path?: string | null;
    required: boolean;
    reason: string;
  }
  interface Candidate {
    audience: string;
    path: string;
    sha512: string;
    byteSize: number;
    optional: boolean;
    configuration: boolean;
    restricted: boolean;
    redistributionEvidence?: string | null;
  }
  interface Review {
    reviewId: string;
    collectionRevision: number;
    versionName?: string;
    files: { audience: string; file: File }[];
    candidates: Candidate[];
    contentBytes: number;
    complete: boolean;
    issues: Issue[];
    exclusions: Issue[];
    guideStatus: string;
  }
  interface Summary {
    publicationId: string;
    available: boolean;
    publicationRevision: number;
    createdAt: string;
    versionName?: string;
    audiences: string[];
    complete: boolean;
    contentBytes: number;
    error?: string;
  }
  interface History {
    items: Summary[];
    total: number;
    unpublishedChanges: boolean;
    comparisonError?: string;
  }
  interface Publication {
    id: string;
    contentFingerprint?: string;
    createdAt: string;
    publicationRevision: number;
    audiences: string[];
    versionName?: string;
    issues: Issue[];
    exclusions: Issue[];
    guideVersion: string;
    playerFiles: Record<string, File>;
    serverFiles: Record<string, File>;
  }
  interface Eligibility {
    supported: boolean;
    reasons: string[];
    warnings: string[];
    complete: boolean;
  }
  interface Datapack {
    slotId: string;
    path: string;
    sha512: string;
    byteSize: number;
  }
  const connection = api.snapshot?.() ?? api;
  const root = contentRoot(serverId);
  let disposed = false,
    busy = false,
    dirty = true;
  let error = '',
    status = '',
    operationId = '',
    worldError = '';
  let transferAbort: AbortController | undefined;
  let snapshot: ContentSnapshot | undefined;
  let history: History | undefined;
  let historyOffset = 0;
  let preparationPercent: number | undefined;
  let audience: 'players' | 'server' = 'players';
  let versionName = '',
    selectedId = 'new';
  let review: Review | undefined;
  let publication: Publication | undefined;
  let eligibility: Eligibility | undefined;
  let incompleteConsent = false,
    includeMrpack = false;
  let destination: 'local' | 'host' = 'local';
  let hostPath = 'content-export.zip';
  let hostInfo: Schema['ContentHostDestinationResultDTO'] | undefined;
  let confirmedHostHash: string | undefined;
  let evidence: Record<string, string> = {};
  let reviewedConfig = new Set<string>(),
    excluded = new Set<string>();
  let additionalServerPaths = '',
    playerLoader = '',
    playerLoaderVersion = '';
  let worlds: Schema['WorldSlotsResponseDTO'] | undefined;
  let slotId = '';
  let datapacks: Datapack[] = [];
  let datapacksReady = true;
  let datapackEvidence: Record<string, string> = {};
  const current = () => !disposed && connection.isCurrent?.() !== false;
  const key = (item: { audience: string; path: string }) => `${item.audience}:${item.path}`;
  function changed(): void {
    dirty = true;
    incompleteConsent = false;
  }
  async function historyPage(offset = 0): Promise<void> {
    const value = await connection.get<History>(`${root}/publications?offset=${offset}&limit=50`);
    if (current()) {
      history = value;
      historyOffset = offset;
    }
  }
  const settingsPending = new Set<string>();
  let settingsCompleted = 0;
  async function settingsRead<T>(url: string, label: string): Promise<T> {
    settingsPending.add(label);
    if (current()) status = `Loading ${[...settingsPending].join(' and ')}…`;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        connection.get<T>(url),
        new Promise<never>((_, reject) => {
          timer = setTimeout(
            () =>
              reject(
                new Error(
                  `The agent did not return ${label} within 30 seconds. Retry preparation.`,
                ),
              ),
            30_000,
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
      settingsPending.delete(label);
      settingsCompleted += 1;
      if (current()) {
        preparationPercent = Math.min(12, settingsCompleted * 4);
        if (settingsPending.size) status = `Waiting for ${[...settingsPending].join(' and ')}…`;
      }
    }
  }
  async function initialize(): Promise<void> {
    busy = true;
    error = '';
    worldError = '';
    status = 'Loading export settings…';
    preparationPercent = 0;
    settingsCompleted = 0;
    settingsPending.clear();
    snapshot = undefined;
    worlds = undefined;
    slotId = '';
    try {
      const caps = await settingsRead<Schema['CapabilitiesDTO']>(
        '/v1/capabilities',
        'agent capabilities',
      );
      if (
        !caps.contentCatalog?.contentPublications ||
        !caps.contentCatalog?.contentZipExports ||
        !caps.contentCatalog?.contentPackExports
      )
        throw new Error('Update this agent to export full modpacks with your changes.');
      const results = await Promise.allSettled([
        settingsRead<ContentSnapshot>(`${root}/collection?metadataOnly=true`, 'content metadata'),
        settingsRead<Schema['WorldSlotsResponseDTO']>('/v1/worlds', 'world settings'),
      ]);
      if (!current()) return;
      if (results[0].status === 'fulfilled') snapshot = results[0].value;
      else
        error = `Current content cannot be reviewed: ${errorMessage(results[0].reason)}. Existing frozen revisions can still be selected.`;
      if (results[1].status === 'fulfilled') {
        worlds = results[1].value;
        slotId = worlds.activeSlotId ?? '';
      } else worldError = `World datapacks are unavailable: ${errorMessage(results[1].reason)}`;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        if (error) status = '';
      }
    }
  }
  async function loadDatapacks(): Promise<void> {
    datapacks = [];
    datapacksReady = worlds !== undefined && !slotId;
    if (worlds) worldError = '';
    changed();
    if (!worlds) {
      error =
        worldError ||
        'The active world could not be checked. Retry preparation to include its datapacks.';
      return;
    }
    if (!slotId) return;
    busy = true;
    status = 'Inspecting world datapacks…';
    error = '';
    try {
      const response = await connection.get<{ items: Datapack[] }>(
        `${root}/publication-datapacks/${encodeURIComponent(slotId)}`,
      );
      if (current()) {
        datapacks = response.items;
        datapacksReady = true;
      }
    } catch (cause) {
      if (current()) {
        worldError = `World datapacks could not be included: ${errorMessage(cause)}`;
        error = worldError;
      }
    } finally {
      if (current()) busy = false;
    }
  }
  async function refreshCurrent(): Promise<void> {
    if (busy) return;
    review = undefined;
    evidence = {};
    reviewedConfig = new Set();
    changed();
    await initialize();
  }
  async function prepare(): Promise<void> {
    if (!snapshot || busy || !current()) return;
    if (!datapacksReady) {
      error = worldError || 'Refresh world datapacks before exporting.';
      return;
    }
    busy = true;
    preparationPercent = 15;
    status = 'Checking server content…';
    dirty = true;
    error = '';
    publication = undefined;
    eligibility = undefined;
    includeMrpack = false;
    try {
      const approvals = (review?.candidates ?? [])
        .filter(
          (item) =>
            evidence[key(item)]?.trim() ||
            (item.configuration && reviewedConfig.has(key(item)) && item.redistributionEvidence),
        )
        .map((item) => ({
          audience: item.audience,
          path: item.path,
          sha512: item.sha512,
          redistributionEvidence: evidence[key(item)]?.trim() || item.redistributionEvidence,
          reviewedConfiguration: reviewedConfig.has(key(item)),
        }));
      const result = await contentOperation(
        connection,
        `${root}/publication-reviews`,
        {
          packExport: true,
          audiences: [audience],
          collectionRevision: snapshot.collection.revision,
          versionName: versionName.trim() || undefined,
          ...(playerLoader
            ? {
                playerSetup: {
                  loader: playerLoader,
                  ...(playerLoaderVersion.trim()
                    ? { loaderVersion: playerLoaderVersion.trim() }
                    : {}),
                },
              }
            : {}),
          approvals: approvals.filter((item) => item.audience === audience),
          excluded: [...excluded]
            .filter((value) => value.startsWith(`${audience}:`))
            .map((value) => ({ audience, path: value.slice(audience.length + 1) })),
          serverPaths: additionalServerPaths
            .split('\n')
            .map((value) => value.trim())
            .filter(Boolean),
          datapacks: datapacks.map((item) => ({
            slotId: item.slotId,
            path: item.path,
            sha512: item.sha512,
            redistributionEvidence: datapackEvidence[item.path]?.trim() ?? '',
          })),
        },
        (line, amount) => {
          if (!current()) return;
          status = line;
          if (amount && amount.total > 0)
            preparationPercent = Math.max(
              preparationPercent ?? 0,
              Math.min(100, 15 + Math.floor((amount.current * 85) / amount.total)),
            );
        },
        trackOperation,
      );
      if (typeof result.reviewId !== 'string')
        throw new Error('Publication review identity is missing.');
      const value = await connection.get<Review>(
        `${root}/publication-reviews/${encodeURIComponent(result.reviewId)}`,
      );
      if (!value.candidates)
        throw new Error('Update this agent to display exact per-file publication approvals.');
      if (current()) {
        review = value;
        dirty = false;
        incompleteConsent = false;
        preparationPercent = 100;
        status = 'Export contents ready.';
      }
    } catch (cause) {
      if (current()) {
        error = errorMessage(cause);
        status = '';
      }
    } finally {
      if (current()) {
        busy = false;
        operationId = '';
      }
    }
  }
  async function inspectPublication(id: string): Promise<void> {
    const value = await connection.get<Publication>(
      `${root}/publications/${encodeURIComponent(id)}`,
    );
    if (current()) {
      publication = value;
      includeMrpack = false;
      incompleteConsent = false;
    }
    try {
      const supported = await connection.get<Eligibility>(
        `${root}/publications/${encodeURIComponent(id)}/mrpack-eligibility?audience=${audience}`,
      );
      if (current()) eligibility = supported;
    } catch (cause) {
      if (current())
        eligibility = {
          supported: false,
          reasons: [errorMessage(cause)],
          warnings: [],
          complete: false,
        };
    }
  }
  async function selectPublication(): Promise<void> {
    publication = undefined;
    eligibility = undefined;
    includeMrpack = false;
    incompleteConsent = false;
    error = '';
    if (selectedId === 'new') return;
    busy = true;
    try {
      await inspectPublication(selectedId);
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  async function publish(): Promise<void> {
    if (!review || dirty || busy || (!review.complete && !incompleteConsent)) return;
    busy = true;
    error = '';
    try {
      const result = await contentOperation(
        connection,
        `${root}/publications`,
        { reviewId: review.reviewId, incompleteConsent },
        (line) => {
          if (current()) status = line;
        },
        trackOperation,
      );
      if (typeof result.publicationId !== 'string')
        throw new Error('Publication identity is missing.');
      if (!current()) return;
      selectedId = result.publicationId;
      review = undefined;
      await inspectPublication(selectedId);
      if (history)
        void historyPage().catch((cause) => {
          if (current()) error = errorMessage(cause);
        });
      if (current())
        status = 'Dated revision published. Later server changes cannot change these frozen files.';
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) {
        busy = false;
        operationId = '';
      }
    }
  }
  async function exportArchive(): Promise<void> {
    if (
      !publication ||
      busy ||
      !publication.audiences.includes(audience) ||
      (publication.issues.length && !incompleteConsent)
    )
      return;
    busy = true;
    error = '';
    let stagedId: string | undefined;
    try {
      if (destination === 'host') {
        const inspected = await connection.get<Schema['ContentHostDestinationResultDTO']>(
          `${root}/export-destination?path=${encodeURIComponent(hostPath.trim())}`,
        );
        if (!current()) return;
        hostInfo = inspected;
        if (inspected.existingSha512 && confirmedHostHash !== inspected.existingSha512) {
          confirmedHostHash = undefined;
          status =
            'This host file already exists or changed. Review the replacement checkbox before saving.';
          return;
        }
      } else if (!connection.saveNativeDownload)
        throw new Error('Update this desktop app to save exports through native transfer.');
      const result = await contentOperation(
        connection,
        `${root}/exports`,
        {
          publicationId: publication.id,
          audience,
          includeMrpack,
          allowIncomplete: incompleteConsent,
          ...(destination === 'host'
            ? {
                hostDestination: {
                  relativePath: hostPath.trim(),
                  ...(confirmedHostHash ? { replaceSha512: confirmedHostHash } : {}),
                },
              }
            : {}),
        },
        (line) => {
          if (current()) status = line;
        },
        trackOperation,
      );
      if (destination === 'host') {
        if (current()) {
          status = `Saved on selected host: ${String(result.hostPath ?? '')}`;
          confirmedHostHash = undefined;
          hostInfo = undefined;
        }
        return;
      }
      if (typeof result.stagedDownloadId !== 'string' || typeof result.fileName !== 'string')
        throw new Error('Export transfer identity is missing.');
      stagedId = result.stagedDownloadId;
      if (!current())
        throw new Error('Export remains on its original host; the save sheet was closed.');
      const size = Number(result.byteSize);
      if (!Number.isSafeInteger(size) || size < 0) throw new Error('Export size is invalid.');
      transferAbort = new AbortController();
      await connection.saveNativeDownload!(stagedId, result.fileName, size, {
        signal: transferAbort.signal,
        onProgress: (progress) => {
          if (current())
            status = `${bytesLabel(progress.bytesTransferred)} of ${bytesLabel(progress.totalBytes)} saved`;
        },
      });
      if (current()) status = 'Export saved on this computer.';
    } catch (cause) {
      if (current()) error = errorMessage(cause);
      else reportTransferFailure(connection, cause);
    } finally {
      if (stagedId) {
        try {
          if (!connection.delete) throw new Error('Export staging cleanup unavailable.');
          await connection.delete(`/v1/staged-downloads/${encodeURIComponent(stagedId)}`);
        } catch (cause) {
          reportTransferFailure(connection, cause);
          if (current()) error += ` Export staging cleanup unconfirmed: ${errorMessage(cause)}`;
        }
      }
      if (current()) {
        busy = false;
        operationId = '';
        transferAbort = undefined;
      }
    }
  }
  async function cancel(): Promise<void> {
    transferAbort?.abort();
    if (operationId) {
      try {
        await connection.post(`/v1/operations/${encodeURIComponent(operationId)}/cancel`, {});
      } catch (cause) {
        if (current()) error = errorMessage(cause);
      }
    }
  }
  async function reviewCurrent(): Promise<void> {
    await initialize();
    if (!current() || !snapshot) return;
    await loadDatapacks();
    await prepare();
  }
  async function switchMode(): Promise<void> {
    selectedId = 'new';
    review = undefined;
    changed();
    await reviewCurrent();
  }

  async function approveSharing(): Promise<void> {
    for (const item of review?.candidates ?? []) {
      if (item.restricted) continue;
      evidence[key(item)] =
        item.redistributionEvidence || 'Owner confirms permission to share these reviewed bytes';
      if (item.configuration) reviewedConfig.add(key(item));
    }
    reviewedConfig = new Set(reviewedConfig);
    for (const item of datapacks) {
      datapackEvidence[item.path] = 'Owner confirms permission to share these reviewed bytes';
    }
    await prepare();
  }
  async function saveExport(): Promise<void> {
    const requestedMrpack = includeMrpack;
    const consent = incompleteConsent;
    if (selectedId === 'new') {
      if (!datapacksReady) return;
      if (dirty) await prepare();
      if (!review || error || (!review.complete && !consent)) return;
      incompleteConsent = consent;
      await publish();
      if (!publication || error) return;
      incompleteConsent = consent;
    }
    if (requestedMrpack && !eligibility?.supported) {
      error = eligibility?.reasons.join(' ') || 'This content cannot be represented as a .mrpack.';
      return;
    }
    includeMrpack = requestedMrpack;
    await exportArchive();
  }
  function trackOperation(id: string): void {
    operationId = id;
    if (!current()) void cancel();
  }
  function closeSheet(): void {
    void cancel();
    onClose();
  }
  onMount(() => void reviewCurrent());
  onDestroy(() => {
    disposed = true;
    transferAbort?.abort();
  });
</script>

<Sheet title="Export modpack" size="lg" onClose={closeSheet}>
  <div class="content">
    <div class="modes" role="group" aria-label="Modpack contents">
      <label class="mode"
        ><input
          type="radio"
          bind:group={audience}
          value="players"
          disabled={busy}
          onchange={() => void switchMode()}
        /><span
          ><strong>Original modpack + your changes</strong><small
            >All retained player content, including client mods, shaders and resource packs, plus
            your additions. Removed mods stay out; disabled mods stay disabled.</small
          ></span
        ></label
      >
      <label class="mode"
        ><input
          type="radio"
          bind:group={audience}
          value="server"
          disabled={busy}
          onchange={() => void switchMode()}
        /><span
          ><strong>Server content only</strong><small
            >The mods and content currently on this server.</small
          ></span
        ></label
      >
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if error && !busy}<Button
        onclick={() => {
          selectedId = 'new';
          review = undefined;
          publication = undefined;
          void reviewCurrent();
        }}>Retry preparation</Button
      >{/if}
    {#if worldError && worldError !== error}<p class="issue">{worldError}</p>{/if}
    {#if !error && (status || busy)}<p role="status">
        {busy ? status || 'Preparing export contents…' : status}
      </p>{/if}
    {#if busy && selectedId === 'new' && preparationPercent !== undefined}
      <div class="preparation-progress">
        <progress max="100" value={preparationPercent} aria-label="Export preparation"></progress>
        <span>{preparationPercent}%</span>
      </div>
    {/if}
    {#if busy && (operationId || transferAbort)}<Button onclick={() => void cancel()}>Cancel</Button
      >{/if}
    {#if selectedId === 'new'}
      {#if review}
        <p>
          {review.files.length} files · {bytesLabel(review.contentBytes)} · dated ZIP with installation
          instructions
        </p>
        {#if review.issues.length}
          <details>
            <summary>Content needing attention ({review.issues.length})</summary>
            {#each review.issues as item}<p class="issue">
                {item.path ?? item.audience}: {item.reason}
              </p>{/each}
          </details>
          {#if review.issues.some((item) => item.reason.includes('Redistribution eligibility') || item.reason.includes('Redistribution permission') || item.reason.includes('Explicitly review this configuration'))}
            <Button disabled={busy} onclick={() => void approveSharing()}
              >I have permission to share these files and have reviewed the configuration</Button
            >
          {/if}
          <label
            ><input type="checkbox" bind:checked={incompleteConsent} disabled={busy} />Export an
            incomplete pack with these missing files listed in its instructions.</label
          >
        {/if}
        {#if review.exclusions.length}<details>
            <summary>Excluded content ({review.exclusions.length})</summary
            >{#each review.exclusions as item}<p>{item.path}: {item.reason}</p>{/each}
          </details>{/if}
        <details>
          <summary>Included files</summary>{#each review.files as item}<p>
              {item.file.path}{item.file.optional ? ' · optional' : ''}{item.file.enabled
                ? ''
                : ' · disabled'}
            </p>{/each}
        </details>
      {/if}
      <details>
        <summary>Export details</summary>
        {#if audience === 'players' && review?.issues.some((item) => item.reason
              .toLowerCase()
              .includes('loader'))}
          <label
            >Player loader<Select
              ariaLabel="Player loader"
              bind:value={playerLoader}
              options={[
                { value: '', label: 'Use server setup' },
                ...['vanilla', 'fabric', 'forge', 'neoforge', 'quilt'].map((value) => ({
                  value,
                  label: value,
                })),
              ]}
              disabled={busy}
              onchange={changed}
            /></label
          >
          <label
            >Player loader version<input
              bind:value={playerLoaderVersion}
              disabled={busy}
              oninput={changed}
            /></label
          >
        {/if}
        <label
          >Version name (optional)<input
            bind:value={versionName}
            disabled={busy}
            oninput={changed}
            placeholder="e.g. Weekend pack"
          /></label
        >
        <Button
          disabled={busy}
          onclick={() => void refreshCurrent().then(loadDatapacks).then(prepare)}
          >Refresh contents</Button
        >
      </details>
    {/if}
    {#if publication}
      <p>
        {dateLabel(publication.createdAt)} · revision {publication.publicationRevision} · {Object.keys(
          audience === 'players' ? publication.playerFiles : publication.serverFiles,
        ).length} files
      </p>
      {#if !publication.contentFingerprint?.startsWith('pack-v1:')}<p>
          This older export uses its previously selected content; it may not contain the full
          original pack.
        </p>{/if}
      {#each publication.issues as item}<p class="issue">
          {item.path ?? item.audience}: {item.reason}
        </p>{/each}
      {#if publication.issues.length}<label
          ><input type="checkbox" bind:checked={incompleteConsent} disabled={busy} />Export this
          incomplete pack with its missing-file instructions.</label
        >{/if}
    {/if}
    <label
      ><input
        type="checkbox"
        bind:checked={includeMrpack}
        disabled={busy || (publication !== undefined && !eligibility?.supported)}
      />Include a launcher-importable .mrpack inside the ZIP</label
    >
    {#each eligibility?.reasons ?? [] as reason}<p>.mrpack unavailable: {reason}</p>{/each}
    {#each eligibility?.warnings ?? [] as warning}<p>{warning}</p>{/each}
    <label
      >Save to<Select
        ariaLabel="Save to"
        value={destination}
        options={[
          { value: 'local', label: 'This computer' },
          { value: 'host', label: 'Selected host’s server exports folder' },
        ]}
        disabled={busy}
        onchange={(value) => (destination = value as 'local' | 'host')}
      /></label
    >
    {#if destination === 'host'}
      <label
        >ZIP filename inside exports/<input
          bind:value={hostPath}
          disabled={busy}
          oninput={() => {
            hostInfo = undefined;
            confirmedHostHash = undefined;
          }}
        /></label
      >
      {#if hostInfo}<p>{hostInfo.root}/{hostInfo.relativePath}</p>{/if}
      {#if hostInfo?.existingSha512}<label
          ><input
            type="checkbox"
            checked={confirmedHostHash === hostInfo.existingSha512}
            disabled={busy}
            onchange={(event) =>
              (confirmedHostHash = event.currentTarget.checked
                ? (hostInfo?.existingSha512 ?? undefined)
                : undefined)}
          />Replace the existing file at this destination.</label
        >{/if}
    {/if}
    <div class="export-action">
      <Button
        variant="primary"
        disabled={busy ||
          (!publication && !review) ||
          (review !== undefined && !review.complete && !incompleteConsent) ||
          (publication !== undefined &&
            (!publication.audiences.includes(audience) ||
              (publication.issues.length > 0 && !incompleteConsent)))}
        onclick={() => void saveExport()}>Export modpack</Button
      >
    </div>
    <details
      ontoggle={(event) => {
        if (event.currentTarget.open && !history)
          void historyPage().catch((cause) => (error = errorMessage(cause)));
      }}
    >
      <summary>Previous exports</summary>
      {#if !history}<p>Loading previous exports…</p>{:else if !history.items.length}<p>
          No previous exports.
        </p>{/if}
      <label
        >Dated export<Select
          ariaLabel="Dated export"
          bind:value={selectedId}
          options={[
            { value: 'new', label: 'Current modpack' },
            ...(history?.items ?? [])
              .filter((item) => item.available && item.audiences.includes(audience))
              .map((item) => ({
                value: item.publicationId,
                label: `${item.createdAt} · ${item.versionName ?? `revision ${item.publicationRevision}`}${item.complete ? '' : ' · incomplete'}`,
              })),
          ]}
          disabled={busy}
          onchange={() => {
            if (selectedId === 'new') void switchMode();
            else void selectPublication();
          }}
        /></label
      >
      {#if history && history.total > 50}<div class="actions">
          <Button
            disabled={busy || historyOffset === 0}
            onclick={() => void historyPage(Math.max(0, historyOffset - 50))}>Previous</Button
          ><Button
            disabled={busy || historyOffset + 50 >= history.total}
            onclick={() => void historyPage(historyOffset + 50)}>Next</Button
          >
        </div>{/if}
    </details>
  </div>
</Sheet>

<style>
  .content {
    display: grid;
    gap: 14px;
  }
  p,
  small {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    color: var(--msc2-text-secondary);
  }
  .modes {
    display: grid;
    gap: 14px;
  }
  .mode {
    display: flex;
    align-items: flex-start;
    gap: 9px;
  }
  .mode span {
    display: grid;
    gap: 4px;
  }
  strong {
    font-size: 13px;
    font-weight: 500;
  }
  .export-action {
    display: flex;
    justify-content: flex-end;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  label {
    display: grid;
    gap: 5px;
    font-size: 12px;
  }
  label:has(input[type='checkbox']) {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  input:not([type='checkbox'], [type='radio']) {
    min-width: 0;
    padding: 7px 9px;
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
  }
  .preparation-progress {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .preparation-progress span {
    min-width: 3ch;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    font-variant-numeric: tabular-nums;
  }
  progress {
    appearance: none;
    height: 4px;
    width: 100%;
    border: 0;
    border-radius: 2px;
    overflow: hidden;
    background: var(--msc2-hairline-subtle);
  }
  progress::-webkit-progress-bar {
    background: var(--msc2-hairline-subtle);
  }
  progress::-webkit-progress-value {
    background: var(--msc2-text-secondary);
  }
  progress::-moz-progress-bar {
    background: var(--msc2-text-secondary);
  }
  .error,
  .issue {
    color: var(--msc2-status-error);
  }
  summary {
    font-size: 12px;
    cursor: pointer;
    margin-bottom: 8px;
  }
  details > label,
  details > p {
    margin-bottom: 8px;
  }
</style>
