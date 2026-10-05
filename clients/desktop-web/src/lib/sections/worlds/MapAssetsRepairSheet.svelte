<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ApiError } from '../../api/client';
  import { save } from '@tauri-apps/plugin-dialog';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import {
    sameBinding,
    sameCaptureRequest,
    matchingCaptureReport,
    confirmedCaptureRepair,
    captureRemedy,
  } from './map-capture-repair';

  export let api: ScreenApi;
  export let serverId: string;
  export let slotId: string;
  export let dimension: string;
  export let area: Schema['MapAssetsAreaDTO'];
  export let onClose: () => void;
  export let onChanged: (generation?: string) => Promise<boolean>;

  // Capture one host before native inspection or transfer can outlive a host switch.
  const client = api.bindHost?.() ?? api;
  const base = `/v1/worlds/${encodeURIComponent(slotId)}/map-assets`;
  const query = `serverId=${encodeURIComponent(serverId)}`;
  let alive = true;
  let busy = false;
  let line = 'Reading rendering evidence…';
  let error = '';
  let report: Schema['MapAssetsReportDTO'] | undefined;
  let rendering: Schema['MapRenderingStatusDTO'] | undefined;
  let selection: Schema['MapAssetsSelectionDTO'] | undefined;
  let preview: Schema['MapClientInspectionDTO'] | undefined;
  let packs: string[] = [];
  let modOrder: string[] = [];
  let confirmModOrder = false;
  let operationId: string | undefined;
  let transfer: AbortController | undefined;
  let uploadId: string | undefined;
  let supportedActions: string[] = [];
  type CaptureRequest = Schema['MapCaptureRequestDTO'];
  type PreparedCapture = {
    token: string;
    prepared: { root: string; game: string; instanceId: string; request: CaptureRequest };
    gameLaunched: boolean;
  };
  type Helper = {
    loaderFamily: string;
    minecraft: string;
    loader: string;
    java: number;
    api?: string;
  };
  type CaptureOutput = {
    token: string;
    size: number;
    sha256: string;
    manifest: Schema['MapCaptureManifestDTO'];
  };
  let instance = '';
  let gameJar = '';
  let java = '';
  let memoryMiB: number | undefined;
  let launcher = '';
  let helpers: { directory: string; manifest: { helpers: Helper[] } } | undefined;
  let captureRequest: CaptureRequest | undefined;
  let capture: PreparedCapture | undefined;
  let captureToken: string | undefined;
  let launched = false;
  let stopped = false;
  let recoveryToken = '';
  let retainedCaptures: PreparedCapture[] = [];
  let cancelled = false;
  let evidenceRead = 0;
  let captureMin = area.min.join(' ');
  let captureMax = area.max.join(' ');
  $: matchingHelper = helpers?.manifest.helpers.find(
    (helper) =>
      helper.loaderFamily === captureRequest?.loader &&
      helper.minecraft === captureRequest?.minecraftVersion &&
      helper.loader === captureRequest?.loaderVersion,
  );
  const hostIdentity = client.hostIdentity?.() ?? 'Selected host';
  function active(): void {
    if (!alive || cancelled) throw new Error('Action cancelled; the existing map is retained.');
  }
  function newToken(): string {
    return [...crypto.getRandomValues(new Uint8Array(16))]
      .map((v) => v.toString(16).padStart(2, '0'))
      .join('');
  }
  function requestedArea(): Schema['MapAssetsAreaDTO'] {
    const coordinates = (value: string): number[] => {
      const result = value.trim().split(/[ ,]+/).map(Number);
      if (
        result.length !== 3 ||
        result.some((n) => !Number.isSafeInteger(n) || n < -2147483648 || n > 2147483647)
      )
        throw new Error('Enter three whole block coordinates for each bound.');
      return result;
    };
    const min = coordinates(captureMin),
      max = coordinates(captureMax);
    if (min.some((n, i) => n > max[i]))
      throw new Error('Minimum coordinates must be below maximum coordinates.');
    return { min, max };
  }
  async function chooseCaptureInput(
    kind: 'instance' | 'gameJar' | 'java' | 'launcher',
  ): Promise<void> {
    if (busy || capture) return;
    const platform = await getPlatform();
    try {
      const selected =
        kind === 'instance'
          ? await platform.pickFolder(
              'Select the matching Prism instance (containing mmc-pack.json)',
            )
          : await platform.pickFilePath({
              label:
                kind === 'gameJar'
                  ? 'Select the matching Minecraft client JAR'
                  : kind === 'java'
                    ? 'Select the Java executable'
                    : 'Select the installed Prism Launcher executable',
              extensions: kind === 'gameJar' ? ['jar'] : undefined,
            });
      if (!alive || !selected) return;
      if (kind === 'instance') instance = selected;
      if (kind === 'gameJar') gameJar = selected;
      if (kind === 'java') java = selected;
      if (kind === 'launcher') launcher = selected;
    } catch (e) {
      if (alive) error = describeError(e);
    }
  }
  async function previewCapture(): Promise<void> {
    if (busy) return;
    busy = true;
    cancelled = false;
    error = '';
    captureRequest = undefined;
    try {
      const expectedRevision = await currentRevision();
      active();
      const request = await client.post<CaptureRequest>(`${base}/capture-request`, {
        serverId,
        expectedRevision,
        dimension,
        area: requestedArea(),
      });
      active();
      captureRequest = request;
      helpers = await invoke('map_capture_helpers');
      active();
      line = 'Review the saved area and exact helper version, then prepare a private client.';
    } catch (e) {
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function prepareCapture(): Promise<void> {
    if (busy || !captureRequest || !matchingHelper || !client.captureContext) return;
    busy = true;
    cancelled = false;
    error = '';
    const requested = captureRequest;
    const token = newToken();
    captureToken = token;
    let ready: PreparedCapture | undefined;
    try {
      line = 'Downloading the authorized saved area from the selected host…';
      const contextFile = await client.captureContext(
        slotId,
        {
          serverId,
          expectedRevision: requested.binding.revision,
          dimension: requested.dimension,
          area: requested.area,
        },
        token,
      );
      active();
      line = 'Checking inputs and preparing a private copy; Minecraft has not started…';
      ready = await invoke<PreparedCapture>('prepare_map_capture', {
        token,
        input: {
          instance,
          context: contextFile,
          gameJar,
          java,
          helpers: helpers!.directory,
          destination: '',
          maxMemoryMib: memoryMiB,
        },
      });
      active();
      const actual = ready.prepared.request;
      if (!sameCaptureRequest(actual, requested))
        throw new Error(
          'The saved context changed since preview. Discard this preparation and preview a fresh area.',
        );
      capture = ready;
      recoveryToken = token;
      retainedCaptures = [...retainedCaptures.filter((item) => item.token !== token), ready];
      line = 'Private client prepared. Launch only when you are ready to render this saved area.';
    } catch (e) {
      if (ready) await invoke('discard_map_capture', { token }).catch(() => {});
      if (alive) {
        captureToken = undefined;
        error = describeError(e);
      }
    } finally {
      await invoke('discard_map_capture_context', { token }).catch(() => {});
      if (alive) busy = false;
    }
  }
  async function launchCapture(): Promise<void> {
    if (busy || !capture || !launcher) return;
    busy = true;
    cancelled = false;
    error = '';
    try {
      await invoke('launch_map_capture', { token: capture.token, launcher });
      active();
      launched = true;
      stopped = false;
      line =
        'Open the private msc-capture save. Load the requested area, run /mscmapcapture prepare, then /mscmapcapture export.';
    } catch (e) {
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function resumeCapture(): Promise<void> {
    if (busy) return;
    busy = true;
    cancelled = false;
    error = '';
    let result: PreparedCapture | undefined;
    try {
      result = await invoke<PreparedCapture>('resume_map_capture', { token: recoveryToken.trim() });
      active();
      const request = result.prepared.request;
      const status = await client.get<Schema['MapAssetsStatusDTO']>(`${base}/status?${query}`);
      active();
      if (!sameBinding(request.binding, status.binding) || request.dimension !== dimension)
        throw new Error(
          'This retained capture belongs to a different or changed host/world. Open its original saved scope or discard it.',
        );
      capture = result;
      captureToken = result.token;
      captureRequest = request;
      launched = result.gameLaunched;
      stopped = false;
      line =
        'Retained private output is available. Inspection and import will revalidate it; Minecraft has not restarted.';
    } catch (e) {
      if (result) await invoke('cancel_map_capture', { token: result.token }).catch(() => {});
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function discardCapture(): Promise<void> {
    if (busy || !captureToken) return;
    busy = true;
    error = '';
    try {
      await invoke('discard_map_capture', { token: captureToken });
      if (alive) {
        retainedCaptures = retainedCaptures.filter((item) => item.token !== captureToken);
        capture = undefined;
        captureToken = undefined;
        captureRequest = undefined;
        launched = false;
        line = 'Private capture discarded. The displayed map is retained.';
      }
    } catch (e) {
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function discardRetainedCapture(): Promise<void> {
    if (busy || !retainedCaptures.some((item) => item.token === recoveryToken)) return;
    busy = true;
    error = '';
    try {
      const token = recoveryToken;
      await invoke('resume_map_capture', { token });
      await invoke('discard_map_capture', { token });
      if (alive) {
        retainedCaptures = retainedCaptures.filter((item) => item.token !== token);
        recoveryToken = '';
      }
    } catch (e) {
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function applyCapture(): Promise<void> {
    if (busy || !capture || !client.uploadFile) return;
    const prepared = capture;
    let output: CaptureOutput | undefined;
    busy = true;
    cancelled = false;
    error = '';
    transfer = new AbortController();
    try {
      const request = prepared.prepared.request;
      line = 'Checking the same saved blocks before applying captured geometry…';
      const checked = await client.post<Schema['MapAssetsCheckStartedDTO']>(`${base}/check`, {
        serverId,
        expectedRevision: request.binding.revision,
        dimension: request.dimension,
        area: request.area,
      });
      if (!alive || cancelled) {
        await client.post(`/v1/operations/${checked.operationId}/cancel`);
        return;
      }
      await wait(checked.operationId);
      active();
      await readEvidence();
      active();
      const before = report;
      if (
        !before ||
        before.operationId !== checked.operationId ||
        before.geometryGenerationId !== request.geometryGenerationId
      )
        throw new Error(
          'The capture base changed or its saved-area check is unavailable. Prepare a fresh saved context.',
        );
      line = 'Validating the private exported geometry and saved context…';
      output = await invoke<CaptureOutput>('inspect_map_capture_output', { token: prepared.token });
      active();
      const expectedRevision = await currentRevision();
      active();
      if (expectedRevision !== prepared.prepared.request.binding.revision)
        throw new Error('The selected world changed. Preview and prepare a fresh saved context.');
      const inspected = output;
      const uploaded = await client.uploadFile(
        'map-client-assets',
        {
          name: 'minecraft-capture.zip',
          size: inspected.size,
          readChunk: async (offset, maxBytes) =>
            new Uint8Array(
              await invoke<ArrayBuffer>('read_map_client_resources', {
                token: inspected.token,
                offset,
                maxBytes,
              }),
            ),
          close: async () => {},
        },
        {
          operationId: expectedRevision,
          fileId: slotId,
          fileName: 'minecraft-capture.zip',
          signal: transfer.signal,
          onProgress: (progress) => {
            if (alive && !cancelled)
              line = `Transferring checked capture: ${progress.bytesUploaded.toLocaleString()} / ${progress.totalBytes.toLocaleString()} bytes`;
          },
        },
      );
      uploadId = uploaded.stagedUploadId;
      active();
      if (uploaded.sha256 !== inspected.sha256)
        throw new Error('Capture transfer checksum mismatch; the previous map is retained.');
      const started = await client.post<Schema['MapAssetsCheckStartedDTO']>(`${base}/import`, {
        serverId,
        expectedRevision,
        dimension: request.dimension,
        area: request.area,
        stagedUploadId: uploadId,
        sha256: inspected.sha256,
      });
      uploadId = undefined;
      if (!alive || cancelled) {
        await client.post(`/v1/operations/${started.operationId}/cancel`);
        return;
      }
      await wait(started.operationId);
      active();
      await readEvidence();
      active();
      if (!report || !matchingCaptureReport(report, before, request, started.operationId))
        throw new Error(
          'No matching adopted capture report was returned; refresh the rendering evidence.',
        );
      if (!(await onChanged(report.geometryGenerationId ?? undefined)))
        throw new Error(
          'The host adopted the checked capture, but the viewer could not display it. Your previous scene is retained; reopen or refresh the map to retry.',
        );
      active();
      line = confirmedCaptureRepair(report, before, request, started.operationId)
        ? 'Saved-area repair validated and adopted. Compare these blocks and their neighbors with Minecraft; visual confirmation is still pending.'
        : 'Checked capture adopted. Read the remaining affected-block issues and compare the saved frame with Minecraft; a complete repair is not established.';
    } catch (e) {
      await cancel();
      if (alive) error = describeError(e);
    } finally {
      if (output)
        await invoke('discard_map_client_resources', { token: output.token }).catch(() => {});
      if (alive) {
        busy = false;
        transfer = undefined;
      }
    }
  }

  $: failures =
    report?.diagnostics.filter(
      (d) =>
        !['model_resolved', 'intentional_empty', 'captured_appearance'].includes(d.classification),
    ) ?? [];
  $: unsupported = failures.some((d) =>
    [
      'unsupported_loader',
      'unsupported_material',
      'missing_context',
      'unsupported_renderer_namespace',
    ].includes(d.classification),
  );
  $: sourceInputRequired =
    !!rendering?.reasonCode &&
    /matching_|client_bundle_checksum|invalid_client_resource/.test(rendering.reasonCode);
  $: canRepair =
    !sourceInputRequired &&
    (!!rendering?.retryable ||
      !!rendering?.requiredSources.some((s) =>
        ['download_failed', 'provider_unavailable', 'checksum_mismatch', 'offline'].includes(
          s.code,
        ),
      ) ||
      (!unsupported &&
        !rendering?.prerequisites.length &&
        !rendering?.requiredSources.length &&
        failures.some((d) =>
          ['missing_model', 'missing_texture', 'invalid_model'].includes(d.classification),
        )));
  $: canRebuild =
    !!rendering?.stale ||
    !!rendering?.retryable ||
    error.includes('renderer_tile_failed') ||
    error.includes('invalid_renderer_artifact') ||
    error.includes('renderer_atlas_changed') ||
    error.includes('renderer_atlas_failed');
  $: shownArea = report?.dimension === dimension ? report.area : area;
  $: mismatch =
    !!preview &&
    !!context &&
    (preview.manifest.minecraftVersion !== context.minecraftVersion ||
      preview.manifest.loader !== context.loader ||
      (!!context.loaderVersion && preview.manifest.loaderVersion !== context.loaderVersion));
  $: previewConflicts = preview ? resourceConflicts(preview.manifest) : [];
  function resourceConflicts(manifest: Schema['MapClientBundleManifestDTO']): string[] {
    if (manifest.modOrder) return [];
    const seen = new Map<string, string>();
    const conflicts = new Set<string>();
    for (const layer of manifest.layers.filter((l) => l.kind === 'mod')) {
      for (const [name, sha] of Object.entries(layer.resources)) {
        const prior = seen.get(name);
        if (prior && prior !== sha) conflicts.add(name);
        seen.set(name, sha);
      }
    }
    return [...conflicts];
  }
  let context: Schema['MapAssetsClientContextDTO'] | undefined;
  $: available = new Set(supportedActions.map((action) => action.split('.')[2]));

  async function readEvidence(): Promise<void> {
    const values = await Promise.allSettled([
      client.get<Schema['MapAssetsReportDTO']>(`${base}/report?${query}`),
      client.get<Schema['MapRenderingStatusDTO']>(
        `${base}/rendering?${query}&dimension=${encodeURIComponent(dimension)}`,
      ),
      client.get<Schema['MapAssetsSelectionDTO']>(`${base}/selection?${query}`),
      client.get<Schema['MapAssetsClientContextDTO']>(`${base}/client-context?${query}`),
      client.get<Schema['MapAssetsCapabilitiesDTO']>('/v1/worlds/map-assets/capabilities'),
    ]);
    if (!alive) return;
    if (values[0].status === 'fulfilled' && values[0].value.dimension === dimension)
      report = values[0].value;
    if (values[1].status === 'fulfilled') rendering = values[1].value;
    if (values[2].status === 'fulfilled') {
      selection = values[2].value;
      packs = selection.manifest?.selectedPacks.slice() ?? [];
      modOrder =
        selection.manifest?.modOrder?.slice() ??
        selection.manifest?.layers.filter((l) => l.kind === 'mod').map((l) => l.id) ??
        [];
      confirmModOrder = !!selection.manifest?.modOrder;
    }
    if (values[3].status === 'fulfilled') context = values[3].value;
    if (values[4].status === 'fulfilled') supportedActions = values[4].value.actions;
    line = report
      ? `Last check: ${report.outcome.replaceAll('_', ' ')}`
      : 'Check the saved blocks around the camera to identify a remedy.';
  }

  const describeError = (e: unknown) =>
    e instanceof ApiError
      ? `${e.error.code}: ${e.message}`
      : e instanceof Error
        ? e.message
        : String(e);

  async function cancel(): Promise<void> {
    cancelled = true;
    transfer?.abort();
    if (captureToken) {
      try {
        await invoke('cancel_map_capture', { token: captureToken });
        launched = false;
        stopped = true;
      } catch (e) {
        if (alive) error = describeError(e);
      }
    }
    if (operationId)
      await client.post(`/v1/operations/${encodeURIComponent(operationId)}/cancel`).catch(() => {});
    if (uploadId) await client.cancelUpload?.(uploadId).catch(() => {});
  }
  function close(): void {
    void cancel();
    onClose();
  }
  onDestroy(() => {
    alive = false;
    void cancel();
    if (preview)
      void invoke('discard_map_client_resources', { token: preview.token }).catch(() => {});
  });
  onMount(() => {
    void readEvidence().catch((e) => {
      if (alive) error = String(e);
    });
  });

  async function currentRevision(): Promise<string> {
    const current = await client.get<Schema['MapAssetsStatusDTO']>(`${base}/status?${query}`);
    if (!alive) throw new Error('The map was closed before this action started.');
    return current.binding.revision;
  }
  async function wait(id: string): Promise<void> {
    operationId = id;
    for (;;) {
      if (!alive) {
        await cancel();
        return;
      }
      const tick = await client.get<Schema['OperationDTO']>(
        `/v1/operations/${encodeURIComponent(id)}`,
      );
      if (!alive) {
        await cancel();
        return;
      }
      line = tick.statusLine ?? 'Validating affected saved terrain…';
      if (['succeeded', 'failed', 'cancelled'].includes(tick.state)) {
        operationId = undefined;
        if (tick.state !== 'succeeded')
          throw new Error(
            tick.error?.message ?? `Rendering action ${tick.state}; the prior scene is retained.`,
          );
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 900));
    }
  }
  async function run(action: string, extra: object = {}): Promise<void> {
    if (busy) return;
    busy = true;
    error = '';
    line = 'Starting affected-area validation…';
    try {
      const expectedRevision = await currentRevision();
      const started = await client.post<Schema['MapAssetsCheckStartedDTO']>(`${base}/${action}`, {
        serverId,
        expectedRevision,
        dimension,
        area: action === 'check' ? area : shownArea,
        ...extra,
      });
      if (!alive) {
        await client.post(`/v1/operations/${encodeURIComponent(started.operationId)}/cancel`);
        return;
      }
      await wait(started.operationId);
      if (!alive) return;
      await readEvidence();
      active();
      if (report?.operationId !== started.operationId)
        throw new Error(
          'This action has no matching adopted report. The previous report is retained.',
        );
      if (action !== 'check') await onChanged();
      if (!alive) return;
      line = `Affected-area result: ${report.outcome.replaceAll('_', ' ')}. Visual confirmation remains yours.`;
    } catch (e) {
      if (alive) error = describeError(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function inspect(folder: boolean): Promise<void> {
    if (busy) return;
    busy = true;
    cancelled = false;
    error = '';
    try {
      const platform = await getPlatform();
      const source = folder
        ? await platform.pickFolder('Select the matching Minecraft client instance')
        : await platform.pickFilePath({
            label: 'Select client resources or an exported MSC bundle',
            extensions: ['zip', 'mrpack', 'jar'],
          });
      if (!source || !alive) return;
      line = 'Inspecting local resource files; preparing a resource-only bundle…';
      const result = await invoke<Schema['MapClientInspectionDTO']>(
        'inspect_map_client_resources',
        { source, context },
      );
      if (!alive || cancelled) {
        await invoke('discard_map_client_resources', { token: result.token });
        return;
      }
      if (preview) await invoke('discard_map_client_resources', { token: preview.token });
      preview = result;
      if (folder) instance = source;
      line = 'Review the version, selection and mod evidence before importing.';
    } catch (e) {
      if (alive) error = String(e);
    } finally {
      if (alive) busy = false;
    }
  }
  async function importResources(): Promise<void> {
    if (!preview || !client.uploadFile || mismatch || busy) return;
    const inspected = preview;
    busy = true;
    cancelled = false;
    error = '';
    transfer = new AbortController();
    try {
      const expectedRevision = await currentRevision();
      active();
      const uploaded = await client.uploadFile(
        'map-client-assets',
        {
          name: 'client-resources.zip',
          size: inspected.size,
          readChunk: async (offset, maxBytes) =>
            new Uint8Array(
              await invoke<ArrayBuffer>('read_map_client_resources', {
                token: inspected.token,
                offset,
                maxBytes,
              }),
            ),
          close: async () => {},
        },
        {
          operationId: expectedRevision,
          fileId: slotId,
          fileName: 'client-resources.zip',
          signal: transfer.signal,
          onProgress: (progress) => {
            if (alive)
              line = `Transferring resources: ${progress.bytesUploaded.toLocaleString()} / ${progress.totalBytes.toLocaleString()} bytes`;
          },
        },
      );
      uploadId = uploaded.stagedUploadId;
      if (!alive || cancelled || transfer.signal.aborted) {
        await cancel();
        return;
      }
      if (uploaded.sha256 !== inspected.sha256)
        throw new Error('Transfer checksum mismatch; resources were not applied.');
      const started = await client.post<Schema['MapAssetsCheckStartedDTO']>(`${base}/import`, {
        serverId,
        expectedRevision,
        dimension,
        area: shownArea,
        stagedUploadId: uploadId,
        sha256: inspected.sha256,
      });
      uploadId = undefined;
      if (!alive || cancelled) {
        await client.post(`/v1/operations/${started.operationId}/cancel`);
        return;
      }
      await wait(started.operationId);
      if (!alive) return;
      await readEvidence();
      active();
      if (report?.operationId !== started.operationId)
        throw new Error('Import has no matching adopted affected-area report.');
      await onChanged();
      if (alive)
        line = `Affected-area result: ${report.outcome.replaceAll('_', ' ')}. Visual confirmation remains yours.`;
    } catch (e) {
      await cancel();
      if (alive) error = describeError(e);
    } finally {
      if (alive) {
        busy = false;
        transfer = undefined;
      }
    }
  }
  async function exportBundle(): Promise<void> {
    if (!preview) return;
    try {
      const destination = await save({
        defaultPath: 'client-resources.zip',
        filters: [{ name: 'MSC resource bundle', extensions: ['zip'] }],
      });
      if (destination && alive)
        await invoke('export_map_client_resources', { token: preview.token, destination });
    } catch (e) {
      if (alive) error = String(e);
    }
  }
  async function exportReport(): Promise<void> {
    if (!report) return;
    try {
      const destination = await save({
        defaultPath: 'map-rendering-report.json',
        filters: [{ name: 'Rendering report', extensions: ['json'] }],
      });
      if (destination && alive)
        await invoke('export_map_rendering_report', {
          report: JSON.stringify(report),
          destination,
        });
    } catch (e) {
      if (alive) error = String(e);
    }
  }
  function togglePack(id: string, enabled: boolean): void {
    packs = enabled ? [...packs, id] : packs.filter((p) => p !== id);
  }
  function moveMod(index: number, direction: number): void {
    const next = [...modOrder];
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    modOrder = next;
  }
  function movePack(index: number, direction: number): void {
    const next = [...packs];
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    packs = next;
  }
</script>

<Sheet title="Check rendering" size="md" onClose={close}>
  <div class="repair-body">
    <p class="lede">{line}</p>
    <p class="quiet">Host {hostIdentity} · server {serverId} · world {slotId}</p>
    <p class="quiet">
      {dimension} · saved blocks {shownArea.min.join(', ')} to {shownArea.max.join(', ')}
    </p>
    {#if error}<p class="error" role="alert">{error}</p>
      {#if captureRemedy(error)}<p>{captureRemedy(error)}</p>{/if}
    {/if}
    {#if busy}
      <Button size="sm" onclick={() => void cancel()}>Cancel action</Button>
    {:else}
      <div class="actions">
        {#if canRepair && available.has('repair')}<Button
            size="sm"
            variant="primary"
            onclick={() => void run('repair')}>Repair map assets</Button
          >{/if}
        {#if available.has('check')}<Button size="sm" onclick={() => void run('check')}
            >Check camera area</Button
          >{/if}
        {#if available.has('import')}
          <Button size="sm" onclick={() => void inspect(true)}>Import client instance</Button>
          <Button
            size="sm"
            variant={canRepair ? 'secondary' : 'primary'}
            onclick={() => void inspect(false)}>Import client assets</Button
          >
        {/if}
      </div>
    {/if}
    {#if sourceInputRequired}
      <p>
        The game, mod or resource inputs changed ({rendering?.reasonCode}). Select the matching
        client release and reimport it; repeating preparation cannot supply that missing input.
      </p>
    {/if}
    {#each (rendering?.prerequisites ?? []).slice(0, 6) as prerequisite}
      <p class="quiet">Required input: {prerequisite.replaceAll('_', ' ')}</p>
    {/each}
    {#if unsupported}
      <p>
        These blocks need Minecraft's rendering or saved world context. Use Minecraft rendering
        below to capture a supported matching client. The preview lists exact supported versions;
        unsupported shaders or alternate renderers remain explicit refusals.
      </p>
    {/if}
    {#if available.has('capture_context') && available.has('capture_request') && available.has('import') && client.captureContext}
      <h3>Use Minecraft rendering</h3>
      <p>
        Capture a bounded saved area in a private Prism instance on this computer. Import and apply
        the matching client instance above first, including its actual pack order. Your running
        server and original saves remain unchanged.
      </p>
      {#if !capture}
        <label>Minimum block coordinates <input bind:value={captureMin} disabled={busy} /></label>
        <label>Maximum block coordinates <input bind:value={captureMax} disabled={busy} /></label>
        <Button size="sm" disabled={busy} onclick={() => void previewCapture()}
          >Preview saved capture</Button
        >
      {/if}
      {#if captureRequest}
        <p>
          {captureRequest.minecraftVersion} · {captureRequest.loader}
          {captureRequest.loaderVersion}<br />
          {captureRequest.dimension} · {captureRequest.area.min.join(', ')} to {captureRequest.area.max.join(
            ', ',
          )}<br />
          Saved snapshot {captureRequest.snapshotId.slice(0, 12)} · one-block neighbor context included
        </p>
        {#if helpers && !matchingHelper && !capture}
          <p class="error">
            No helper matches this exact version. A nearby release cannot capture this world. Keep
            the current map and export the rendering report with this version for adapter support.
          </p>
          <p class="quiet">
            Available helpers: {helpers.manifest.helpers
              .map(
                (h) =>
                  `${h.minecraft} ${h.loaderFamily} ${h.loader}${h.api ? `, Fabric API ${h.api}` : ''}`,
              )
              .join(' · ')}
          </p>
        {/if}
        {#if matchingHelper}<p class="quiet">
            Java {matchingHelper.java}{matchingHelper.api
              ? ` · Fabric API ${matchingHelper.api}`
              : ''} required. Saved-frame geometry supports checked textures, tint, transparency and light.
            Custom shaders, unsupported material behavior and alternate Fabric renderers are refused;
            animation is a saved frame.
          </p>{/if}
      {/if}
      {#if captureRequest && matchingHelper && !capture}
        <div class="capture-input">
          <span>Matching Prism instance<br /><small>{instance || 'Not selected'}</small></span>
          <Button size="sm" disabled={busy} onclick={() => void chooseCaptureInput('instance')}
            >Choose instance</Button
          >
        </div>
        <div class="capture-input">
          <span>Minecraft client JAR<br /><small>{gameJar || 'Not selected'}</small></span>
          <Button size="sm" disabled={busy} onclick={() => void chooseCaptureInput('gameJar')}
            >Choose client JAR</Button
          >
        </div>
        <div class="capture-input">
          <span>Java executable<br /><small>{java || 'Not selected'}</small></span>
          <Button size="sm" disabled={busy} onclick={() => void chooseCaptureInput('java')}
            >Choose Java</Button
          >
        </div>
        <label
          >Private client memory (MiB, optional)<input
            type="number"
            min="512"
            max="65536"
            step="1"
            bind:value={memoryMiB}
            disabled={busy}
          /></label
        >
        <p class="quiet">
          Leave empty to use the selected instance's allocation. This changes only the private
          client.
        </p>
        <Button
          size="sm"
          disabled={busy ||
            !instance ||
            !gameJar ||
            !java ||
            (memoryMiB !== undefined &&
              (!Number.isInteger(memoryMiB) || memoryMiB < 512 || memoryMiB > 65536))}
          onclick={() => void prepareCapture()}>Prepare private client</Button
        >
      {/if}
      {#if capture}
        <p class="quiet">
          Private files: {capture.prepared.root}<br />Recovery identifier: {capture.token}
        </p>
        <p>
          Sign in to this separate Prism directory if required. Open its msc-capture save, load the
          requested dimension and chunks, then run <code>/mscmapcapture prepare</code> followed by
          <code>/mscmapcapture export</code>. Return here to apply the checked output.
        </p>
        {#if !launched}
          <div class="capture-input">
            <span>Prism Launcher executable<br /><small>{launcher || 'Not selected'}</small></span>
            <Button
              size="sm"
              disabled={busy}
              onclick={async () => {
                try {
                  const path = await (
                    await getPlatform()
                  ).pickFilePath({
                    label: 'Select the installed Prism Launcher executable',
                  });
                  if (alive && path) launcher = path;
                } catch (e) {
                  if (alive) error = describeError(e);
                }
              }}>Choose Prism</Button
            >
          </div>
          <Button
            size="sm"
            disabled={busy || !launcher || stopped}
            onclick={() => void launchCapture()}>Launch private Minecraft</Button
          >
        {/if}
        {#if stopped}<Button size="sm" disabled={busy} onclick={() => void resumeCapture()}
            >Reopen stopped session</Button
          >{/if}
        <div class="actions">
          <Button size="sm" variant="primary" disabled={busy} onclick={() => void applyCapture()}
            >Validate and apply captured rendering</Button
          >
          <Button size="sm" disabled={busy} onclick={() => void cancel()}
            >Stop private client</Button
          >
          <Button size="sm" disabled={busy} onclick={() => void discardCapture()}
            >Discard private files</Button
          >
        </div>
        <p class="quiet">
          Closing this sheet stops its private client and retains prepared files for retry. Save the
          recovery identifier before closing. Expired or changed context requires a fresh preview.
        </p>
      {:else}
        <details>
          <summary>Reopen retained capture</summary>
          {#each retainedCaptures as retained}
            <label
              ><input
                type="radio"
                bind:group={recoveryToken}
                value={retained.token}
                disabled={busy}
              />
              {retained.prepared.request.minecraftVersion}
              {retained.prepared.request.loader} · host {retained.prepared.request.binding
                .agentHostId} · world {retained.prepared.request.binding.slotId}<br />
              <span class="quiet">{retained.prepared.root}</span></label
            >
          {/each}
          <label>Recovery identifier <input bind:value={recoveryToken} disabled={busy} /></label>
          <Button
            size="sm"
            disabled={busy || !/^[a-fA-F0-9]{32}$/.test(recoveryToken.trim())}
            onclick={() => void resumeCapture()}>Reopen private output</Button
          >
          {#if retainedCaptures.some((item) => item.token === recoveryToken)}
            <Button size="sm" disabled={busy} onclick={() => void discardRetainedCapture()}
              >Discard selected private files</Button
            >
          {/if}
        </details>
      {/if}
    {/if}
    {#if failures.some((d) => d.classification === 'missing_saved_chunk')}
      <p>
        Some chunks have no saved terrain. Visit and save those chunks in Minecraft, then use
        Refresh terrain in the map. Resource downloads cannot generate them.
      </p>
    {/if}
    {#each (rendering?.requiredSources ?? []).slice(0, 8) as source}
      <p class="quiet">{source.file}: {source.identity} · {source.code}</p>
    {/each}
    {#if preview}
      <h3>Inspected client resources</h3>
      <p>
        {preview.manifest.minecraftVersion} · {preview.manifest.loader}{preview.manifest
          .loaderVersion
          ? ` ${preview.manifest.loaderVersion}`
          : ''} · {(preview.size / 1048576).toFixed(1)} MiB
      </p>
      {#if mismatch}<p class="error">
          This source does not match the selected server's game or loader version. Choose its
          matching client release.
        </p>{/if}
      <p class="quiet">
        Local bytes were hashed. {preview.manifest.selectionKnown
          ? 'Pack selection is recorded.'
          : 'Pack selection is unknown; choose the intended order after import.'} Mod overlap with unknown
        order remains a reported conflict.
      </p>
      {#if previewConflicts.length}
        <p>
          {previewConflicts.length} conflicting mod resources have unknown priority. Import retains these
          conflicts until you supply the actual client's mod resource order.
        </p>
        <p class="quiet">{previewConflicts.slice(0, 4).join(' · ')}</p>
      {/if}
      <details>
        <summary>Version and mod evidence ({preview.manifest.layers.length} layers)</summary>
        {#each preview.manifest.layers.slice(0, 40) as layer}
          <p>
            {layer.label} · {layer.kind}{#each layer.sources as source}{#each source.declaredMods as mod}
                · {mod.id} {mod.version ?? 'version unknown'}{/each}{/each}
          </p>
        {/each}
        {#if preview.manifest.layers.length > 40}<p class="quiet">
            {preview.manifest.layers.length - 40} more layers in the exported bundle.
          </p>{/if}
      </details>
      <div class="actions">
        <Button
          size="sm"
          variant="primary"
          disabled={busy || mismatch || !context}
          onclick={() => void importResources()}>Apply matching resources</Button
        >
        <Button size="sm" disabled={busy} onclick={() => void exportBundle()}
          >Export resource bundle</Button
        >
      </div>
    {/if}
    {#if selection?.manifest && available.has('selection')}
      <details open={rendering?.prerequisites.includes('client_pack_selection_required')}>
        <summary>Select resource packs</summary>
        <p class="quiet">
          Low to high priority. A higher pack overrides a lower pack. Only imported pack layers are
          listed.
        </p>
        {#each selection.manifest.layers.filter((l) => l.kind === 'pack') as layer}
          <label
            ><input
              type="checkbox"
              checked={packs.includes(layer.id)}
              disabled={busy}
              onchange={(event) => togglePack(layer.id, event.currentTarget.checked)}
            />
            {layer.label}</label
          >
        {/each}
        {#each packs as id, index}<div class="pack-order">
            <span>{selection.manifest.layers.find((l) => l.id === id)?.label ?? id}</span><Button
              size="sm"
              disabled={busy || index === 0}
              onclick={() => movePack(index, -1)}>Lower</Button
            ><Button
              size="sm"
              disabled={busy || index === packs.length - 1}
              onclick={() => movePack(index, 1)}>Higher</Button
            >
          </div>{/each}
        {#if modOrder.length > 1}
          <p class="quiet">
            Overlapping mod resources need the matching client's actual priority. Do not guess from
            filenames.
          </p>
          <label
            ><input type="checkbox" bind:checked={confirmModOrder} disabled={busy} /> Record this known
            mod resource order</label
          >
          {#if confirmModOrder}
            {#each modOrder as id, index}<div class="pack-order">
                <span>{selection.manifest.layers.find((l) => l.id === id)?.label ?? id}</span
                ><Button size="sm" disabled={busy || index === 0} onclick={() => moveMod(index, -1)}
                  >Lower</Button
                ><Button
                  size="sm"
                  disabled={busy || index === modOrder.length - 1}
                  onclick={() => moveMod(index, 1)}>Higher</Button
                >
              </div>{/each}
          {/if}
        {/if}
        <Button
          size="sm"
          disabled={busy}
          onclick={() =>
            void run('selection', {
              expectedSelectionRevision: selection?.selectionRevision,
              selectedPacks: packs,
              modOrder: confirmModOrder ? modOrder : null,
            })}>Apply pack selection</Button
        >
      </details>
    {/if}
    {#if report}
      <details>
        <summary>Affected blocks and evidence ({failures.length} issues)</summary>
        <p class="quiet">
          Snapshot {report.snapshotId.slice(0, 12)} · {report.inspectedBlocks.toLocaleString()} saved
          blocks · {report.scope}
        </p>
        {#each failures.slice(0, 12) as issue}<p>
            <strong>{issue.originalId}</strong> · {issue.classification.replaceAll('_', ' ')}<br
            />{issue.detail}<br /><span class="quiet"
              >{issue.samples
                .slice(0, 3)
                .map((s) => s.join(', '))
                .join(' / ')}</span
            >
          </p>{/each}
        <p class="quiet">
          {Math.max(0, failures.length - 12) + report.omittedIssues} additional issues · {report.omittedSamples}
          omitted samples. Model resolution does not establish correct visual appearance.
        </p>
        {#if report.repair}<p class="quiet">
            Resources {report.repair.sourceGenerationId.slice(0, 12)} → {report.repair.targetGenerationId.slice(
              0,
              12,
            )} · {report.repair.sameSavedArea
              ? 'same saved area compared'
              : 'snapshot or area changed; no repair comparison claimed'}
          </p>{/if}
      </details>
      <div class="actions">
        {#if canRebuild && report.geometryGenerationId && available.has('rebuild')}<Button
            size="sm"
            disabled={busy}
            onclick={() => void run('rebuild')}>Rebuild affected terrain</Button
          >{/if}
        {#if selection?.previousGeneration && available.has('restore')}<Button
            size="sm"
            disabled={busy}
            onclick={() => void run('restore', { generation: selection?.previousGeneration })}
            >Restore previous map resources</Button
          >{/if}
        <Button size="sm" disabled={busy} onclick={() => void exportReport()}>Export report</Button>
      </div>
    {/if}
  </div>
</Sheet>

<style>
  .repair-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 24px 24px;
  }
  p {
    margin: 0;
    font-size: 13px;
    line-height: 1.55;
  }
  .lede {
    font-size: 15px;
    font-weight: 500;
  }
  .quiet {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .error {
    color: var(--msc2-status-error);
  }
  .actions,
  .pack-order,
  .capture-input {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .capture-input span,
  .pack-order span {
    flex: 1;
    overflow-wrap: anywhere;
  }
  input:not([type='checkbox']) {
    background: var(--msc2-bg-content);
    color: var(--msc2-text-primary);
    border: 1px solid var(--msc2-border);
    padding: 6px 8px;
    width: 100%;
  }
  small {
    color: var(--msc2-text-secondary);
    overflow-wrap: anywhere;
  }
  h3 {
    margin: 8px 0 0;
    font-size: 13px;
    font-weight: 500;
  }
  details {
    font-size: 13px;
  }
  summary {
    cursor: pointer;
    padding: 6px 0;
    font-weight: 500;
  }
  details p,
  details label,
  details .pack-order {
    margin: 8px 0;
  }
  label {
    display: block;
  }
</style>
