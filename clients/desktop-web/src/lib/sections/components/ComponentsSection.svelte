<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import TransferPanel from '../transfers/TransferPanel.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { addonPaths, demoAddons } from '../addons/model';

  export let api: ScreenProps['api'] = undefined;
  let components: Schema['ComponentsStatusDTO'] = { components: [], restartRequiredToApply: false };
  let notice = '';

  onMount(async () => {
    components = await call(api, components, '/v1/components');
  });
  async function update(component: string): Promise<void> {
    try {
      const result = await mutate<Schema['ComponentUpdateResultDTO']>(api, addonPaths.update, {
        component,
      });
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function exportClient(): Promise<void> {
    try {
      const result = api
        ? await api.get<Schema['ClientExportResponseDTO']>(addonPaths.export)
        : {
            exportKind: 'unavailable',
            isPaperLike: false,
            items: [],
            selectedCount: 0,
            serverType: 'unknown',
          };
      notice = result.stagedDownloadId
        ? 'Client export staged for download.'
        : (result.note ?? 'Client export prepared.');
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Installed components"
    title="Components"
    description="System components expose their real state and provenance; unavailable provider checks remain visible instead of becoming fake update controls."
    status={`${components.components.length} components`}
    statusTone="positive"
    actionLabel="Export client pack"
    onAction={exportClient}
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  <section class="screen-card">
    <table class="data-table">
      <thead
        ><tr
          ><th>Component</th><th>Installed</th><th>Latest</th><th>State</th><th class="actions"
            >Actions</th
          ></tr
        ></thead
      ><tbody
        >{#each components.components as item (item.name)}<tr
            ><td><strong>{item.name}</strong></td><td>{item.installedVersion ?? 'Not installed'}</td
            ><td>{item.latestVersion ?? 'Not advertised'}</td><td
              >{#if item.note}<span class="tag">{item.note}</span>{:else}{item.isUpToDate
                  ? 'Up to date'
                  : 'Update available'}{/if}</td
            ><td class="actions"
              ><ActionButton
                kind="quiet"
                label={`Update ${item.name}`}
                disabled={!item.updatable}
                onclick={() => update(item.name)}>Update</ActionButton
              ></td
            ></tr
          >{:else}<tr><td colspan="5" class="empty-row">No system components reported.</td></tr
          >{/each}</tbody
      >
    </table>
  </section>
  <section class="screen-card">
    <h3>Local add-on completion</h3>
    <p>
      When a provider requires a manual browser download, stage the file through a bounded token
      before the agent installs it.
    </p>
    <TransferPanel {api} purpose="addon-local-file" label="Local add-on JAR" />
  </section>
  {#if components.restartRequiredToApply}<p class="capability-notice">
      A restart is required to apply one or more component changes.
    </p>{/if}
</div>
