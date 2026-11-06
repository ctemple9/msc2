<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
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
  let selectedDatapacks = new Set<string>();
  let datapackEvidence: Record<string, string> = {};
  const current = () => !disposed && connection.isCurrent?.() !== false;
  const key = (item: { audience: string; path: string }) => `${item.audience}:${item.path}`;
  function changed(): void {
    dirty = true;
    incompleteConsent = false;
  }
  function toggle(set: Set<string>, value: string): Set<string> {
    const next = new Set(set);
    if (next.has(value)) next.delete(value);
    else next.add(value);
    changed();
    return next;
  }
  async function historyPage(offset = 0): Promise<void> {
    const value = await connection.get<History>(`${root}/publications?offset=${offset}&limit=50`);
    if (current()) {
      history = value;
      historyOffset = offset;
    }
  }
  async function initialize(): Promise<void> {
    busy = true;
    error = '';
    worldError = '';
    try {
      const caps = await connection.get<Schema['CapabilitiesDTO']>('/v1/capabilities');
      if (!caps.contentCatalog?.contentPublications || !caps.contentCatalog?.contentZipExports)
        throw new Error('Update this agent to publish and export content.');
      const results = await Promise.allSettled([
        connection.get<ContentSnapshot>(`${root}/collection`),
        connection.get<Schema['WorldSlotsResponseDTO']>('/v1/worlds'),
        historyPage(),
      ]);
      if (!current()) return;
      if (results[0].status === 'fulfilled') snapshot = results[0].value;
      else
        error = `Current content cannot be reviewed: ${errorMessage(results[0].reason)}. Existing frozen revisions can still be selected.`;
      if (results[1].status === 'fulfilled') worlds = results[1].value;
      else worldError = `World datapacks are unavailable: ${errorMessage(results[1].reason)}`;
      if (results[2].status === 'rejected')
        error += ` Publication history unavailable: ${errorMessage(results[2].reason)}`;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
    } finally {
      if (current()) busy = false;
    }
  }
  async function loadDatapacks(): Promise<void> {
    datapacks = [];
    selectedDatapacks = new Set();
    changed();
    if (!slotId) return;
    busy = true;
    error = '';
    try {
      const response = await connection.get<{ items: Datapack[] }>(
        `${root}/publication-datapacks/${encodeURIComponent(slotId)}`,
      );
      if (current()) datapacks = response.items;
    } catch (cause) {
      if (current()) error = errorMessage(cause);
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
    busy = true;
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
          datapacks: datapacks
            .filter((item) => selectedDatapacks.has(item.path))
            .map((item) => ({
              slotId: item.slotId,
              path: item.path,
              sha512: item.sha512,
              redistributionEvidence: datapackEvidence[item.path]?.trim() ?? '',
            })),
        },
        (line) => {
          if (current()) status = line;
        },
        (id) => (operationId = id),
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
        status = 'Review the included files, exclusions and unresolved content.';
      }
    } catch (cause) {
      if (current()) error = errorMessage(cause);
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
        (id) => (operationId = id),
      );
      if (typeof result.publicationId !== 'string')
        throw new Error('Publication identity is missing.');
      if (!current()) return;
      selectedId = result.publicationId;
      review = undefined;
      await inspectPublication(selectedId);
      await historyPage();
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
        (id) => (operationId = id),
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
  onMount(() => void initialize());
  onDestroy(() => {
    disposed = true;
    transferAbort?.abort();
  });
</script>

<Sheet title="Export content" size="lg" onClose={busy ? undefined : onClose}>
  <div class="content">
    <p>
      Export content as a local file. Each published revision is dated and keeps its reviewed files
      so it can be exported again later.
    </p>
    <div class="actions">
      <label
        >Audience<select
          bind:value={audience}
          disabled={busy}
          onchange={() => {
            changed();
            void selectPublication();
          }}><option value="players">Players</option><option value="server">Server</option></select
        ></label
      ><label
        >Revision<select
          bind:value={selectedId}
          disabled={busy}
          onchange={() => void selectPublication()}
          ><option value="new">Publish current content</option
          >{#each history?.items ?? [] as item (item.publicationId)}<option
              value={item.publicationId}
              disabled={!item.available}
              >{item.createdAt} · r{item.publicationRevision}
              {item.versionName ?? ''}{item.complete ? '' : ' · incomplete'}</option
            >{/each}</select
        ></label
      >
    </div>
    {#if history?.unpublishedChanges}<p>
        Current content has unpublished changes. Existing dated revisions remain unchanged.
      </p>{/if}
    {#if history?.comparisonError}<p>{history.comparisonError}</p>{/if}
    {#if history && history.total > 50}<div class="actions">
        <Button
          disabled={busy || historyOffset === 0}
          onclick={() =>
            void historyPage(Math.max(0, historyOffset - 50)).catch(
              (cause) => (error = errorMessage(cause)),
            )}>Previous revisions</Button
        ><Button
          disabled={busy || historyOffset + 50 >= history.total}
          onclick={() =>
            void historyPage(historyOffset + 50).catch((cause) => (error = errorMessage(cause)))}
          >Next revisions</Button
        >
      </div>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if status}<p role="status">{status}</p>{/if}
    {#if busy && (operationId || transferAbort)}<Button onclick={() => void cancel()}
        >Cancel current work</Button
      >{/if}
    {#if selectedId === 'new'}
      <Button disabled={busy} onclick={() => void refreshCurrent()}>Refresh current content</Button>
      <label
        >Version name (optional)<input
          bind:value={versionName}
          disabled={busy}
          oninput={changed}
          placeholder="e.g. Weekend pack"
        /></label
      >
      {#if audience === 'players'}<div class="actions">
          <label
            >Player loader<select bind:value={playerLoader} disabled={busy} onchange={changed}
              ><option value="">Infer from the server/pack</option
              >{#each ['vanilla', 'fabric', 'forge', 'neoforge', 'quilt'] as loader}<option
                  value={loader}>{loader}</option
                >{/each}</select
            ></label
          ><label
            >Exact loader version<input
              bind:value={playerLoaderVersion}
              disabled={busy}
              oninput={changed}
              placeholder="Required for an explicit mod loader"
            /></label
          >
        </div>{/if}
      {#if audience === 'server'}<label
          >Additional config/scripts (one exact relative path per line)<textarea
            bind:value={additionalServerPaths}
            disabled={busy}
            oninput={changed}
            placeholder="config/example.toml"
          ></textarea></label
        >{/if}
      <details>
        <summary>Include datapacks from a world slot</summary>{#if worldError}<p>
            {worldError}
          </p>{/if}<label
          >World<select bind:value={slotId} disabled={busy} onchange={() => void loadDatapacks()}
            ><option value="">No world datapacks</option
            >{#each worlds?.slots ?? [] as slot (slot.id)}<option value={slot.id}
                >{slot.name}</option
              >{/each}</select
          ></label
        >{#each datapacks as file (file.path)}<div class="row">
            <label
              ><input
                type="checkbox"
                checked={selectedDatapacks.has(file.path)}
                onchange={() => (selectedDatapacks = toggle(selectedDatapacks, file.path))}
                disabled={busy}
              />{file.path} · {bytesLabel(file.byteSize)}</label
            ><input
              aria-label={`Redistribution evidence for ${file.path}`}
              bind:value={datapackEvidence[file.path]}
              oninput={changed}
              disabled={busy}
              placeholder="License or redistribution permission"
            />
          </div>{/each}
      </details>
      <Button disabled={busy || !snapshot} onclick={() => void prepare()}
        >{review ? 'Refresh publication review' : 'Review current content'}</Button
      >
      {#if review}
        <h3>Included content · {bytesLabel(review.contentBytes)}</h3>
        {#each review.files as item}<p>
            {item.file.path} · {item.file.optional ? 'Optional' : 'Required'}{item.file.enabled
              ? ''
              : ' · inactive'}
          </p>{/each}
        <h3>Per-file inclusion and permission</h3>
        <p>
          Owner-selected optional content is included by default. Config/scripts need an explicit
          content review. Download access does not establish redistribution permission.
        </p>
        {#each review.candidates as item (key(item))}<div class="row">
            <label
              ><input
                type="checkbox"
                checked={!excluded.has(key(item))}
                onchange={() => (excluded = toggle(excluded, key(item)))}
                disabled={busy}
              />{item.path}{item.optional ? ' · optional' : ''} · {bytesLabel(item.byteSize)}</label
            >{#if item.restricted}<p>
                Provider redistribution restriction: this file cannot be embedded.
              </p>{:else}<label
                >Redistribution evidence<input
                  bind:value={evidence[key(item)]}
                  oninput={changed}
                  disabled={busy}
                  placeholder={item.redistributionEvidence ??
                    'License or permission for these exact bytes'}
                /></label
              >{/if}{#if item.configuration}<label
                ><input
                  type="checkbox"
                  checked={reviewedConfig.has(key(item))}
                  onchange={() => (reviewedConfig = toggle(reviewedConfig, key(item)))}
                  disabled={busy}
                />I reviewed this config/script file for sharing.</label
              >{/if}
          </div>{/each}
        {#each review.issues as item}<p class="issue">
            {item.path ?? item.audience}: {item.reason}
          </p>{/each}{#each review.exclusions as item}<p>
            Excluded: {item.path ?? item.audience} · {item.reason}
          </p>{/each}
        {#if review.guideStatus === 'textFallback'}<p>
            Text launcher instructions are available. Illustrated guides await the owner-supplied
            screenshots.
          </p>{/if}
        {#if !review.complete}<label
            ><input type="checkbox" bind:checked={incompleteConsent} disabled={busy} />Publish this
            explicitly incomplete revision with the listed missing/restricted content recorded.</label
          >{/if}
        <Button
          disabled={busy || dirty || (!review.complete && !incompleteConsent)}
          onclick={() => void publish()}>Publish dated revision</Button
        >
      {/if}
    {/if}
    {#if publication}
      <h3>
        {dateLabel(publication.createdAt)} · revision {publication.publicationRevision}{publication.versionName
          ? ` · ${publication.versionName}`
          : ''}
      </h3>
      {#if !publication.audiences.includes(audience)}<p>
          Choose an audience included in this revision before exporting.
        </p>{/if}
      {#each Object.values(audience === 'players' ? publication.playerFiles : publication.serverFiles) as file (file.path)}<p
        >
          {file.path} · {bytesLabel(file.byteSize)}{file.optional ? ' · optional' : ''}{file.enabled
            ? ''
            : ' · inactive'}
        </p>{/each}
      {#each publication.issues as item}<p class="issue">
          {item.path ?? item.audience}: {item.reason}
        </p>{/each}{#each publication.exclusions as item}<p>
          Excluded: {item.path ?? item.audience} · {item.reason}
        </p>{/each}
      {#if publication.issues.length}<label
          ><input type="checkbox" bind:checked={incompleteConsent} disabled={busy} />Export this
          explicitly incomplete revision with its warnings and missing-file instructions.</label
        >{/if}
      <label
        ><input
          type="checkbox"
          bind:checked={includeMrpack}
          disabled={busy || !eligibility?.supported}
        />Include a launcher-importable .mrpack inside the ZIP</label
      >
      {#each eligibility?.reasons ?? [] as reason}<p>
          .mrpack unavailable: {reason}
        </p>{/each}{#each eligibility?.warnings ?? [] as warning}<p>{warning}</p>{/each}
      <label
        >Save to<select bind:value={destination} disabled={busy}
          ><option value="local">This computer (native save dialog)</option><option value="host"
            >Selected host’s server exports folder</option
          ></select
        ></label
      >
      {#if destination === 'host'}<label
          >ZIP filename inside exports/<input
            bind:value={hostPath}
            disabled={busy}
            oninput={() => {
              hostInfo = undefined;
              confirmedHostHash = undefined;
            }}
          /></label
        >{#if hostInfo}<p>
            Host destination: {hostInfo.root}/{hostInfo.relativePath}
          </p>{/if}{#if hostInfo?.existingSha512}<label
            ><input
              type="checkbox"
              checked={confirmedHostHash === hostInfo.existingSha512}
              disabled={busy}
              onchange={(event) =>
                (confirmedHostHash = event.currentTarget.checked
                  ? (hostInfo?.existingSha512 ?? undefined)
                  : undefined)}
            />Replace the existing file at this reviewed destination.</label
          >{/if}
        <p>
          Host saving requires Add-ons and Admin permission. Other host paths are not exposed.
        </p>{/if}
      <Button
        disabled={busy ||
          !publication.audiences.includes(audience) ||
          (publication.issues.length > 0 && !incompleteConsent)}
        onclick={() => void exportArchive()}>Export ZIP</Button
      >
    {/if}
  </div>
</Sheet>

<style>
  .content {
    display: grid;
    gap: 12px;
  }
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  h3 {
    margin: 8px 0 0;
    font-size: 13px;
    font-weight: 500;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  label {
    display: grid;
    gap: 4px;
    font-size: 12px;
  }
  label:has(input[type='checkbox']) {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  input:not([type='checkbox']),
  select,
  textarea {
    min-width: 0;
    padding: 7px 9px;
    background: var(--msc2-tier-content);
    color: var(--msc2-text-primary);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
  }
  textarea {
    min-height: 60px;
    resize: vertical;
  }
  .row {
    display: grid;
    gap: 7px;
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .error,
  .issue {
    color: var(--msc2-status-error);
  }
  summary {
    font-size: 13px;
    cursor: pointer;
    margin-bottom: 8px;
  }
</style>
