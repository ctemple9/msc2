<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import StorageCleanup from './StorageCleanup.svelte';
  import { loadSavedRemoteHosts } from '../../hosts/saved';
  import {
    loadStorageBridge,
    type StorageBridge,
    type StorageSnapshot,
    type StorageItem,
    type StorageScan,
    type StorageSummary,
    type InventoryReply,
    type ScanAccepted,
    type LocalFootprint,
    type StorageCandidates,
    type StorageReview,
    type StorageSelection,
    type StorageResult,
  } from '../../platform/storage';
  import {
    storageBytesLabel,
    categories,
    total,
    bySize,
    grouped,
    pendingCleanups,
    rememberCleanup,
    forgetCleanup,
    recentResults,
  } from './storage-model';
  import { errorMessage, type ScreenApi } from '../shared/types';
  export let bridge: StorageBridge | undefined = undefined;
  export let api: ScreenApi | undefined = undefined;
  export let hostName = 'Selected host';
  export let onBack: () => void = () => {};
  type Computer = {
    id: string;
    agentId: string | null;
    name: string;
    local: boolean;
    snapshot: StorageSnapshot | null;
    summary: StorageSummary | null;
    historical: boolean;
    fresh: boolean;
    busy: boolean;
    scan: string | null;
    localScan: boolean;
    problem: string;
    progress: string;
    result: StorageResult | null;
    cleanup: string | null;
    unsupported: boolean;
    paused: boolean;
  };
  let computers: Computer[] = [];
  let selected = '';
  let category = '';
  let server = '';
  let group = '';
  let local: LocalFootprint | null = null;
  let alive = true;
  let loading = true;
  let loadProblem = '';
  let action = '';
  let candidates: StorageCandidates | null = null;
  let selection: string[] = [];
  let review: StorageReview | null = null;
  let reviewName = '';
  let confirmation = '';
  let actionBusy = false;
  let actionProblem = '';
  let reviewGeneration = 0;
  let timer: ReturnType<typeof setInterval> | undefined;
  let polling = false;
  $: computer = computers.find((c) => c.id === selected);
  $: snapshot = computer?.snapshot;
  $: categoryItems = snapshot?.items.filter((i) => i.category === category) ?? [];
  $: serverItems = categoryItems.filter((i) =>
    server === 'unassigned' ? i.serverId === null : i.serverId === server,
  );
  $: visibleItems = bySize(
    server ? (group ? serverItems.filter((i) => i.group === group) : serverItems) : categoryItems,
  );
  $: reviewAllowed =
    !!computer?.fresh &&
    !!computer.agentId &&
    !computer.busy &&
    !computer.cleanup &&
    !computer.scan &&
    !computer.localScan;
  $: title = review
    ? 'Review removal'
    : action
      ? 'Select files'
      : server
        ? (serverItems[0]?.label.split(' / ')[0] ?? 'Server')
        : category
          ? (categories.find((c) => c[0] === category)?.[1] ?? category)
          : (computer?.name ?? 'Storage');
  function update(c: Computer) {
    computers = computers.map((item) => (item.id === c.id ? c : item));
  }
  function groupLabel(id: string): string {
    const labels: Record<string, string> = {
      worlds: 'Worlds',
      backups: 'Backups',
      components: 'Mods & server files',
      other: 'Logs & other files',
    };
    return labels[id] ?? id;
  }
  function dated(time: number) {
    return time ? new Date(time * 1000).toLocaleString() : 'Not scanned';
  }
  function size(bytes: number | null | undefined) {
    return bytes == null ? 'Unknown' : storageBytesLabel(bytes);
  }
  function overviewOf(c: Computer) {
    return c.snapshot ?? c.summary;
  }
  function summary(c: Computer) {
    const data = overviewOf(c);
    return data
      ? `${data.partial || (c.local && !c.fresh) ? 'At least ' : ''}${size(data.logicalBytes)}`
      : 'Unknown';
  }
  function itemSize(item: StorageItem) {
    return item.partial
      ? item.logicalBytes
        ? `At least ${size(item.logicalBytes)}`
        : 'Unknown'
      : size(item.logicalBytes);
  }
  function clearReview() {
    reviewGeneration++;
    action = '';
    candidates = null;
    selection = [];
    review = null;
    confirmation = '';
    actionProblem = '';
    actionBusy = false;
  }
  function backToSelection() {
    reviewGeneration++;
    review = null;
    confirmation = '';
    actionBusy = false;
  }
  function back() {
    if (review) {
      backToSelection();
    } else if (action) {
      clearReview();
    } else if (group) {
      group = '';
    } else if (server) {
      server = '';
    } else if (category) {
      category = '';
    } else if (selected) {
      selected = '';
    } else {
      onBack();
    }
  }
  async function choose(id: string) {
    clearReview();
    selected = id;
    category = '';
    server = '';
    group = '';
    const c = computers.find((c) => c.id === id);
    if (c && !c.local && c.agentId && bridge) {
      try {
        c.snapshot = await bridge.cached(c.agentId);
        update(c);
      } catch (e) {
        c.problem = errorMessage(e);
        update(c);
      }
    }
  }
  async function initialize() {
    try {
      bridge ??= await loadStorageBridge();
      local = await bridge.local('cached');
      const remotes = loadSavedRemoteHosts().filter((h) => h.id !== local?.agentHostId);
      computers = [
        {
          id: 'this-computer',
          agentId: local.agentHostId,
          name: 'This computer',
          local: true,
          snapshot: local.snapshot,
          summary: null,
          historical: true,
          fresh: false,
          busy: false,
          scan: null,
          localScan: !!local.scanId && !local.done,
          problem: local.problem ?? '',
          progress: '',
          result: null,
          cleanup: null,
          unsupported: false,
          paused: false,
        },
        ...remotes.map((h) => ({
          id: h.id,
          agentId: h.id,
          name: h.displayName,
          local: false,
          snapshot: null,
          summary: null,
          historical: true,
          fresh: false,
          busy: false,
          scan: null,
          localScan: false,
          problem: '',
          progress: '',
          result: null,
          cleanup: null,
          unsupported: false,
          paused: false,
        })),
      ];
      // Only bounded credential-bound metadata reads occur on entry.
      const summaries = await bridge.overview(remotes.map((h) => h.id));
      for (const c of computers) {
        if (c.local && c.agentId) {
          try {
            c.snapshot = (await bridge.composed(c.agentId)) ?? c.snapshot;
          } catch (e) {
            c.problem = errorMessage(e);
          }
        }
        c.summary = summaries.find((s) => s.hostId === c.agentId) ?? null;
        c.result = recentResults.get(c.id) ?? null;
        update(c);
      }
      const scans = await bridge.running();
      for (const c of computers) {
        c.scan = c.agentId ? (scans[c.agentId] ?? null) : null;
        const pending = pendingCleanups().find((p) => p.hostId === c.agentId);
        c.cleanup = pending?.operationId ?? null;
        update(c);
      }
      timer = setInterval(() => void poll(), 1000);
      void poll();
    } catch (e) {
      loadProblem = errorMessage(e);
    } finally {
      loading = false;
    }
  }
  onMount(() => {
    void initialize();
  });
  onDestroy(() => {
    alive = false;
    clearInterval(timer);
  });
  async function loadInventory(c: Computer) {
    if (!bridge || !c.agentId) return;
    const reply = await bridge.remote<InventoryReply>(c.agentId, 'inventory');
    if (reply.hostId !== c.agentId) throw Error('Computer identity changed.');
    c.snapshot = c.local
      ? ((await bridge.composed(c.agentId)) ?? local?.snapshot ?? reply.snapshot)
      : reply.snapshot;
    c.historical = reply.historical;
  }
  async function refresh(c: Computer) {
    if (!bridge || c.busy || c.scan || c.localScan || c.cleanup) return;
    clearReview();
    c.busy = true;
    c.problem = '';
    c.unsupported = false;
    c.fresh = false;
    update(c);
    try {
      if (c.local) {
        local = await bridge.local('refresh');
        c.localScan = !local.done;
        c.snapshot = local.snapshot ?? c.snapshot;
        c.problem = local.problem ?? '';
      }
      if (c.agentId) {
        try {
          const accepted = await bridge.remote<ScanAccepted>(c.agentId, 'refresh');
          if (accepted.hostId !== c.agentId) throw Error('Computer identity changed.');
          c.scan = accepted.operationId;
        } catch (e) {
          const message = errorMessage(e);
          c.unsupported = message.includes('404') || message.includes('405');
          c.problem = c.unsupported
            ? 'This agent does not support storage inventory. Update the agent to scan and manage files.'
            : `Agent unavailable: ${message}. Dated results remain historical.`;
        }
      } else
        c.problem =
          'Desktop footprint only. Pair this computer’s agent to include its server data.';
    } catch (e) {
      c.problem = errorMessage(e);
    } finally {
      c.busy = false;
      update(c);
      void poll();
    }
  }
  async function poll() {
    if (!bridge || polling || !alive) return;
    polling = true;
    try {
      for (const c of computers) {
        if (!alive) break;
        if (c.localScan) {
          try {
            local = await bridge.local('status');
            c.localScan = !local.done;
            c.progress = `Desktop: ${local.progress.entries.toLocaleString()} entries inspected`;
            if (local.done) {
              c.snapshot = local.snapshot ?? c.snapshot;
              if (c.agentId) c.snapshot = (await bridge.composed(c.agentId)) ?? c.snapshot;
              c.problem = local.problem ?? c.problem;
            }
          } catch (e) {
            c.localScan = false;
            c.problem = errorMessage(e);
          }
        }
        if (c.scan && c.agentId && !c.paused) {
          try {
            const state = await bridge.remote<StorageScan>(c.agentId, 'status', c.scan);
            if (state.hostId !== c.agentId) throw Error('Computer identity changed.');
            c.progress = `${state.progress.entries.toLocaleString()} entries inspected · ${size(state.progress.logicalBytesVisited)} visited`;
            if (!['queued', 'running'].includes(state.state)) {
              c.scan = null;
              await loadInventory(c);
              c.fresh =
                !c.historical &&
                !!c.snapshot &&
                !['cancelled', 'failed', 'interrupted'].includes(state.state);
              if (state.message) c.problem = state.message;
            }
          } catch (e) {
            c.paused = true;
            c.problem = `Scan status unavailable: ${errorMessage(e)}. Check again after reconnecting; no scan was restarted.`;
          }
        }
        if (c.cleanup && c.agentId && !c.paused) {
          try {
            const result = await bridge.result(c.agentId, c.cleanup);
            if (result.hostId !== c.agentId) throw Error('Computer identity changed.');
            c.result = result;
            recentResults.set(c.id, result);
            if (result.state !== 'running') {
              c.cleanup = null;
              forgetCleanup(c.agentId);
              c.fresh = false;
              c.historical = true;
              c.problem = 'Files changed. Refresh this computer before another removal.';
            }
          } catch (e) {
            c.paused = true;
            c.problem = `Removal status unavailable: ${errorMessage(e)}. Operation ${c.cleanup} remains on its original computer.`;
          }
        }
        update(c);
      }
    } finally {
      polling = false;
    }
  }
  async function cancel(c: Computer) {
    if (!bridge) return;
    try {
      if (c.localScan) await bridge.local('cancel');
      const id = c.cleanup ?? c.scan;
      if (id && c.agentId) await bridge.remote(c.agentId, 'cancel', id);
      c.progress = 'Cancellation requested; waiting for a safe boundary.';
    } catch (e) {
      c.problem = errorMessage(e);
    }
    update(c);
  }
  function request(c: Computer, ids: string[]): StorageSelection {
    if (!c.agentId) throw Error('No agent paired.');
    return { hostId: c.agentId, category, action, itemIds: ids };
  }
  async function inspectAction(next: string) {
    if (!bridge || !computer || !reviewAllowed || server === 'unassigned') return;
    clearReview();
    action = next;
    const generation = reviewGeneration;
    const target = computer;
    const scope = request(target, []);
    actionBusy = true;
    try {
      const reply = await bridge.candidates(scope);
      if (reply.hostId !== scope.hostId) throw Error('Computer identity changed.');
      if (alive && generation === reviewGeneration && selected === target.id)
        candidates = {
          ...reply,
          items: server ? reply.items.filter((i) => i.serverId === server) : reply.items,
        };
    } catch (e) {
      if (generation === reviewGeneration) actionProblem = errorMessage(e);
    } finally {
      if (generation === reviewGeneration) actionBusy = false;
    }
  }
  function toggle(id: string) {
    selection = selection.includes(id)
      ? selection.filter((i) => i !== id)
      : ['remove-world', 'remove-server'].includes(action)
        ? [id]
        : [...selection, id];
  }
  async function makeReview() {
    if (!bridge || !computer || !selection.length || !reviewAllowed) return;
    const target = computer;
    const generation = reviewGeneration;
    const scope = request(target, [...selection]);
    actionBusy = true;
    actionProblem = '';
    try {
      const reply = await bridge.review(scope);
      if (reply.hostId !== scope.hostId) throw Error('Computer identity changed.');
      if (alive && generation === reviewGeneration && selected === target.id) {
        review = reply;
        reviewName = target.name;
        confirmation = '';
      }
    } catch (e) {
      if (generation === reviewGeneration) actionProblem = errorMessage(e);
    } finally {
      if (generation === reviewGeneration) actionBusy = false;
    }
  }
  async function apply() {
    if (!bridge || !review || confirmation !== review.confirmation) return;
    const reviewed = review;
    const generation = reviewGeneration;
    const target = computers.find((c) => c.agentId === reviewed.hostId);
    if (!target || target.cleanup) return;
    actionBusy = true;
    actionProblem = '';
    try {
      const accepted = await bridge.apply({
        hostId: reviewed.hostId,
        previewToken: reviewed.previewToken,
        confirmation,
      });
      if (accepted.hostId !== reviewed.hostId) throw Error('Computer identity changed.');
      target.cleanup = accepted.operationId;
      target.fresh = false;
      rememberCleanup({
        hostId: reviewed.hostId,
        name: target.name,
        operationId: accepted.operationId,
      });
      if (generation === reviewGeneration && selected === target.id) clearReview();
      update(target);
      void poll();
    } catch (e) {
      if (generation === reviewGeneration) {
        review = null;
        confirmation = '';
        actionProblem = `${errorMessage(e)} Create a fresh review before trying again.`;
      }
    } finally {
      if (generation === reviewGeneration || (selected === target.id && !action))
        actionBusy = false;
    }
  }
  async function reveal(item: StorageItem) {
    if (!bridge || !computer?.local) return;
    try {
      try {
        await bridge.reveal(item.id);
      } catch (error) {
        if (computer.agentId) await bridge.reveal(item.id, computer.agentId);
        else throw error;
      }
    } catch (e) {
      computer.problem = errorMessage(e);
      update(computer);
    }
  }
</script>

<section class="storage-view" aria-label="Storage">
  <div class="toolbar">
    <Button size="sm" onclick={back}
      >‹ {review
        ? 'Selection'
        : action
          ? 'Inventory'
          : group
            ? 'Server'
            : server
              ? 'Servers'
              : category
                ? computer?.name
                : selected
                  ? 'Computers'
                  : 'Settings'}</Button
    >
    {#if computer && !review && !action}<Button
        size="sm"
        disabled={computer.busy || !!computer.scan || computer.localScan || !!computer.cleanup}
        onclick={() => void refresh(computer!)}>{computer.busy ? 'Starting…' : 'Refresh'}</Button
      >{/if}
  </div>
  <div class="heading">
    <h2>{title}</h2>
    <p class="quiet">
      {computer
        ? `${computer.name} · ${snapshot ? 'Logical size · ' + dated(snapshot.completedAt) : 'No inventory yet'}`
        : 'MSC files on your saved computers'}
    </p>
  </div>
  {#if loading}<p class="quiet">Reading dated inventories…</p>{:else if loadProblem}<p role="alert">
      {loadProblem}
    </p>{:else if !computer}
    <div class="column-head"><span>Computer</span><span>MSC usage</span><span>Available</span></div>
    {#each computers as c (c.id)}<button class="computer-row" onclick={() => choose(c.id)}
        ><span
          >{c.name}<small
            >{c.scan || c.localScan
              ? 'Scan in progress'
              : overviewOf(c)
                ? `${dated(overviewOf(c)!.completedAt)} · Historical${overviewOf(c)!.partial ? ' · Partial' : ''}`
                : 'Not scanned · Select to refresh'}</small
          ></span
        ><span class="number">{summary(c)}</span><span class="number quiet"
          >{overviewOf(c)?.volumes.length === 1
            ? size(overviewOf(c)!.volumes[0].availableBytes)
            : overviewOf(c)?.volumes.length
              ? `${overviewOf(c)!.volumes.length} volumes`
              : 'Unknown'} ›</span
        ></button
      >{/each}
    {#if computers.length === 1}<p class="quiet">No other computers saved.</p>{/if}
    <p class="footnote">
      Sizes describe known MSC files. Opening this view does not connect to or scan saved computers.
    </p>
  {:else}
    {#if computer.problem}<p class="notice" role="status">{computer.problem}</p>{/if}
    {#if computer.scan || computer.localScan || computer.cleanup}<div class="working">
        <span
          >{computer.cleanup
            ? 'Removing reviewed files'
            : computer.progress || 'Scan in progress'}<small
            >{computer.cleanup ?? computer.scan ?? ''}</small
          ></span
        >{#if computer.paused}<Button
            size="sm"
            onclick={() => {
              computer!.paused = false;
              update(computer!);
              void poll();
            }}>Check status</Button
          >{/if}<Button size="sm" onclick={() => void cancel(computer!)}
          >Cancel remaining work</Button
        >
      </div>{/if}
    {#if review}
      <p class="review-computer">{reviewName}</p>
      <p>
        Permanently remove {review.items.length} selected {review.items.length === 1
          ? 'item'
          : 'items'} from this computer?
      </p>
      <p class="quiet">
        Review expires in five minutes. Changed or active files are retained. Cancellation stops
        later items; earlier removals cannot be undone.
      </p>
      {#each review.items as item (item.id)}<div class="file-row">
          <span
            >{item.label}<small>{item.displayPath}</small><small>{item.reason}</small
            >{#each item.consequences as consequence}<small>{consequence}</small>{/each}</span
          ><span class="number">{size(item.logicalBytes)}</span>
        </div>{/each}
      <label class="confirm"
        >Type <strong>{review.confirmation}</strong><Field
          bind:value={confirmation}
          placeholder="Confirmation"
        /></label
      >
      <div class="actions">
        <Button size="sm" onclick={backToSelection}>Back to selection</Button><Button
          variant="destructive"
          size="sm"
          disabled={actionBusy || confirmation !== review.confirmation}
          onclick={() => void apply()}>{actionBusy ? 'Submitting…' : 'Remove permanently'}</Button
        >
      </div>
    {:else if action}
      <p class="quiet">
        {action === 'delete-backups'
          ? 'Select backups on this computer. The last verified backup for each server stays protected.'
          : ['remove-world', 'remove-server'].includes(action)
            ? 'Choose one item for a separate removal review.'
            : 'Select inactive files to review. Active or unverified files stay protected.'}
      </p>
      {#if actionBusy}<p class="quiet">Checking current ownership and use…</p>{/if}
      {#if candidates}{#each candidates.items as item (item.id)}<label class="candidate"
            ><input
              type="checkbox"
              checked={selection.includes(item.id)}
              disabled={!item.removable || actionBusy}
              onchange={() => toggle(item.id)}
            /><span
              >{item.label}<small>{item.reason}</small>{#if item.displayPath}<small
                  >{item.displayPath}</small
                >{/if}</span
            ><span class="number">{size(item.logicalBytes)}</span></label
          >{/each}{#if !candidates.items.length}<p class="quiet">
            No files owned by this action were found.
          </p>{/if}{/if}
      <div class="actions">
        <span class="quiet">{selection.length} selected</span><Button
          size="sm"
          variant="primary"
          disabled={!selection.length || actionBusy || !reviewAllowed}
          onclick={() => void makeReview()}>Review removal…</Button
        >
      </div>
    {:else if !category}
      <p class="measured">{summary(computer)} <span>measured</span></p>
      {#if snapshot}<p class="quiet">
          {computer.historical
            ? 'Historical results · Refresh before removal'
            : snapshot.partial
              ? 'Partial known-root coverage'
              : 'Complete known-root coverage'}{snapshot.allocatedBytes != null
            ? ` · ${size(snapshot.allocatedBytes)} allocated`
            : ' · Allocation unknown'}
        </p>
        <div class="proportion" aria-label="MSC logical bytes by category">
          {#each categories as cat}{@const amount = total(
              snapshot.items.filter((i) => i.category === cat[0]),
            )}<span
              title={`${cat[1]}: ${size(amount)}`}
              style={`flex-grow:${amount};display:${amount ? 'block' : 'none'}`}
            ></span>{/each}
        </div>
        <div class="bar-labels">
          {#each categories as cat}<span>{cat[1]}</span>{/each}
        </div>
        {#each snapshot.volumes as volume (volume.id)}<div class="volume">
            <span
              >{volume.label || 'Volume'}<small
                >Capacity checked {dated(volume.observedAt)}{volume.problem
                  ? ' · ' + volume.problem
                  : ''}</small
              ></span
            ><span class="number"
              >{size(volume.availableBytes)} available<small>of {size(volume.totalBytes)}</small
              ></span
            >
          </div>{/each}
        {#each categories as cat}<button
            class="inventory-row"
            onclick={() => {
              category = cat[0];
            }}
            ><span>{cat[1]}</span><span class="number"
              >{size(total(snapshot.items.filter((i) => i.category === cat[0])))} ›</span
            ></button
          >{/each}
        {#if snapshot.issues.length}<details>
            <summary>Coverage details ({snapshot.issues.length})</summary
            >{#each snapshot.issues as issue}<p class="quiet">{issue.reason}</p>{/each}
          </details>{/if}
      {:else}<p class="quiet">
          Refresh this computer to measure known MSC files. Desktop measurement works independently
          of its agent.
        </p>{/if}
    {:else if category === 'servers' && !server && snapshot}
      {#each grouped(snapshot, 'serverId', categoryItems) as entry (entry.id)}<button
          class="inventory-row"
          onclick={() => {
            server = entry.id;
          }}><span>{entry.label}</span><span class="number">{size(entry.bytes)} ›</span></button
        >{/each}
      <div class="actions">
        <Button
          size="sm"
          disabled={!reviewAllowed}
          onclick={() => void inspectAction('delete-backups')}>Manage backups…</Button
        ><Button
          size="sm"
          disabled={!reviewAllowed}
          onclick={() => void inspectAction('remove-world')}>Remove a saved world…</Button
        ><Button
          size="sm"
          disabled={!reviewAllowed}
          onclick={() => void inspectAction('remove-server')}>Remove a server…</Button
        >
      </div>
    {:else if server && !group && snapshot}
      <p class="measured">
        {size(total(serverItems))}
        <span>{server === 'unassigned' ? 'in unregistered files' : 'in this server'}</span>
      </p>
      {#each grouped(snapshot, 'group', serverItems) as entry (entry.id)}<button
          class="inventory-row"
          onclick={() => {
            group = entry.id;
          }}
          ><span>{groupLabel(entry.id)}</span><span class="number">{size(entry.bytes)} ›</span
          ></button
        >{/each}
      {#if server === 'unassigned'}
        <p class="quiet">
          These files are in known server folders but are not linked to a registered server. They
          may belong to more than one former server. MSC cannot verify a server removal for them, so
          they remain protected. Open Logs &amp; other files to inspect their locations.
        </p>
      {:else}
        <p class="quiet">
          Live worlds and server files are protected inventory. Manage saved slots, backups or the
          whole server through separate reviews.
        </p>
        <div class="actions">
          <Button
            size="sm"
            disabled={!reviewAllowed}
            onclick={() => void inspectAction('delete-backups')}>Manage backups…</Button
          ><Button
            size="sm"
            disabled={!reviewAllowed}
            onclick={() => void inspectAction('remove-world')}>Remove a saved world…</Button
          ><Button
            size="sm"
            disabled={!reviewAllowed}
            onclick={() => void inspectAction('remove-server')}>Remove this server…</Button
          >
        </div>
      {/if}
    {:else}
      <p class="measured">{size(total(visibleItems))} <span>logical size</span></p>
      {#each visibleItems as item (item.id)}<div class="file-row">
          <span
            >{item.label}<small
              >{item.protection}{item.partial ? ' · Partial' : ''}{item.absent
                ? ' · Absent'
                : ''}{item.sharedReferences
                ? ` · ${item.sharedReferences} shared references`
                : ''}</small
            ><small>{item.displayPath}</small
            >{#if computer.local && item.displayPath && item.domainId === null}<button
                class="text-action"
                onclick={() => void reveal(item)}>Open folder…</button
              >{/if}</span
          ><span class="number"
            >{itemSize(item)}<small
              >{item.allocatedBytes == null
                ? 'Allocation unknown'
                : `${size(item.allocatedBytes)} allocated`}</small
            ></span
          >
        </div>{/each}
      {#if category === 'cache'}<div class="actions">
          <Button
            size="sm"
            disabled={!reviewAllowed}
            onclick={() => void inspectAction('map-cache')}>Review map cache…</Button
          ><Button
            size="sm"
            disabled={!reviewAllowed}
            onclick={() => void inspectAction('old-downloads')}>Review old transfers…</Button
          >
        </div>
      {:else if category === 'agent'}<p class="quiet">
          Current agent files remain protected. Installation and repair are available from Agent… in
          the computer picker.
        </p>
        <Button
          size="sm"
          disabled={!reviewAllowed}
          onclick={() => void inspectAction('obsolete-builds')}>Review obsolete builds…</Button
        >
      {:else if category === 'desktopApp'}<p class="quiet">
          The current desktop app remains protected. Updates are available in MSC Settings.
        </p>
      {:else if category === 'sharedTools'}<p class="quiet">
          Installed runtimes and tools remain protected while dependency ownership is uncertain. Use
          the server editor to manage supported components.
        </p>{/if}
    {/if}
    {#if actionProblem}<p role="alert" class="notice">{actionProblem}</p>{/if}
    {#if !computer.fresh && !computer.scan && !computer.localScan && !review && !action}<p
        class="footnote"
      >
        Refresh this computer before reviewing removal. Unknown or unreadable files are never
        reported as zero.
      </p>{/if}
    {#if computer.result && !review && !action}<details open>
        <summary>Last removal · {computer.result.state}</summary>
        <p class="quiet">
          Operation {computer.result.operationId} · {dated(computer.result.observedAt)}
        </p>
        {#each computer.result.items as item (item.id)}<div class="file-row">
            <span>{item.label}<small>{item.state} · {item.message}</small></span><span
              class="number"
              >{item.removedLogicalBytes == null
                ? 'Unconfirmed'
                : `${size(item.removedLogicalBytes)} removed`}</span
            >
          </div>{/each}
        {#each computer.result.volumes as volume}<p class="quiet">
            {volume.label}: {size(volume.availableBytes)} available · checked {dated(
              volume.observedAt,
            )}{volume.problem ? ' · ' + volume.problem : ''}
          </p>{/each}
        <p class="quiet">
          Observed available space on the agent data volume: {size(computer.result.availableBytes)}.
          This change is not attributed to MSC removal.
        </p>
      </details>{/if}
    <p class="footnote">
      Sizes use decimal units (GB and MB). On macOS, available space includes storage the system can
      reclaim for your files. File sizes may differ from disk space released by removal. Hard links,
      sparse files and filesystem clones affect allocation.
    </p>
  {/if}
  <details class="legacy">
    <summary>Map cleanup on {hostName === 'Local agent' ? 'Local Agent' : hostName}</summary>
    {#key api}<StorageCleanup {api} {hostName} />{/key}
  </details>
</section>

<style>
  .storage-view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    color: var(--msc2-text-primary);
    font-size: 13px;
    min-height: 420px;
  }
  .toolbar,
  .actions,
  .working {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .heading {
    margin: 8px 0;
  }
  h2 {
    font-size: 20px;
    font-weight: 600;
    margin: 0 0 7px;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .quiet,
  .footnote,
  small {
    color: var(--msc2-text-tertiary);
    font-size: 12px;
  }
  .footnote {
    line-height: 1.5;
    margin-top: 6px;
  }
  small {
    display: block;
    margin-top: 4px;
    overflow-wrap: anywhere;
    line-height: 1.4;
  }
  .number {
    font-variant-numeric: tabular-nums;
    text-align: right;
    white-space: nowrap;
  }
  .column-head,
  .computer-row {
    display: grid;
    grid-template-columns: minmax(140px, 1fr) minmax(85px, auto) minmax(85px, auto);
    gap: 20px;
    align-items: start;
  }
  .column-head {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .column-head span:not(:first-child) {
    text-align: right;
  }
  .computer-row,
  .inventory-row {
    appearance: none;
    border: 0;
    border-bottom: 1px solid var(--msc2-hairline);
    background: transparent;
    color: inherit;
    text-align: left;
    font: inherit;
    padding: 14px 0;
    cursor: pointer;
    width: 100%;
  }
  .computer-row:hover,
  .inventory-row:hover {
    background: var(--msc2-tier-content);
  }
  .computer-row:focus-visible,
  .inventory-row:focus-visible,
  .text-action:focus-visible {
    outline: 2px solid var(--msc2-text-secondary);
    outline-offset: 3px;
  }
  .inventory-row,
  .file-row,
  .volume {
    display: flex;
    justify-content: space-between;
    align-items: start;
    gap: 20px;
  }
  .file-row,
  .volume {
    padding: 12px 0;
    border-bottom: 1px solid var(--msc2-hairline);
  }
  .file-row > span:first-child {
    min-width: 0;
  }
  .measured {
    font-size: 25px;
    font-weight: 500;
    margin-top: 4px;
  }
  .measured span {
    font-size: 13px;
    font-weight: 400;
    color: var(--msc2-text-tertiary);
  }
  .proportion {
    display: flex;
    gap: 2px;
    height: 8px;
    margin-top: 8px;
  }
  .proportion span {
    background: var(--msc2-text-secondary);
    min-width: 1px;
  }
  .proportion span:nth-child(even) {
    opacity: 0.55;
  }
  .bar-labels {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    gap: 8px;
  }
  .notice {
    padding: 10px 0;
    color: var(--msc2-text-secondary);
  }
  .working {
    padding: 10px 0;
    border-bottom: 1px solid var(--msc2-hairline);
  }
  .candidate {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: start;
    padding: 13px 0;
    border-bottom: 1px solid var(--msc2-hairline);
  }
  input[type='checkbox'] {
    margin: 2px 0;
    accent-color: var(--msc2-text-secondary);
  }
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 12px 0;
  }
  .confirm strong {
    font-weight: 500;
    overflow-wrap: anywhere;
  }
  .review-computer {
    font-size: 16px;
    font-weight: 500;
  }
  .text-action {
    background: none;
    border: 0;
    color: var(--msc2-text-secondary);
    padding: 6px 0;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  details {
    padding: 10px 0;
  }
  summary {
    cursor: pointer;
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .legacy {
    margin-top: 24px;
    border-top: 1px solid var(--msc2-hairline);
  }
  .legacy[open] > summary {
    margin-bottom: 12px;
  }
  @media (max-width: 560px) {
    .computer-row,
    .column-head {
      gap: 10px;
      grid-template-columns: minmax(100px, 1fr) auto auto;
    }
    .bar-labels {
      flex-wrap: wrap;
    }
    .file-row,
    .volume {
      gap: 10px;
    }
    .number {
      white-space: normal;
    }
  }
</style>
