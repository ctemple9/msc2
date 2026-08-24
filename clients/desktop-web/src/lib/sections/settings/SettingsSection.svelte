<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  let settings: Schema['SettingsResponseDTO'] = {
    editable: false,
    sections: [],
    serverName: 'Survival',
    serverRunning: false,
    serverType: 'paper',
  };
  let changes: Record<string, string> = {};
  let javaPath = '';
  let ramMax = 8;
  let geyserAddress = '';
  let geyserPort = 19132;
  let notice = '';

  onMount(async () => {
    settings = await call(api, settings, '/v1/settings');
    const java = await call<Schema['JavaConfigResponseDTO']>(api, {}, '/v1/config/java-runtime');
    javaPath = java.executablePath ?? '';
    const ram = await call<Schema['RAMConfigResponseDTO']>(
      api,
      {
        hasActiveServer: false,
        maxRamGB: 8,
        minRamGB: 2,
        physicalRAMGB: 16,
        recommendedMaxGB: 12,
        serverName: settings.serverName,
        serverRunning: settings.serverRunning,
        serverType: settings.serverType,
      },
      '/v1/config/ram',
    );
    ramMax = ram.maxRamGB;
    const geyser = await call<Schema['GeyserConfigResponseDTO']>(
      api,
      {
        configFileExists: false,
        isGeyserInstalled: false,
        serverName: settings.serverName,
        serverType: settings.serverType,
      },
      '/v1/config/geyser',
    );
    geyserAddress = geyser.address ?? '';
    geyserPort = geyser.port ?? geyserPort;
  });

  async function saveSettings(): Promise<void> {
    try {
      const result = await mutate<Schema['SettingsUpdateResultDTO']>(api, '/v1/settings', {
        changes,
      });
      notice = result.message;
      if (result.sections) settings = { ...settings, sections: result.sections };
      changes = {};
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function saveJava(): Promise<void> {
    try {
      notice = (
        await mutate<Schema['JavaConfigResponseDTO']>(api, '/v1/config/java-runtime', {
          executablePath: javaPath,
        })
      ).executablePath
        ? 'Java executable saved.'
        : 'Java executable cleared.';
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function saveRam(): Promise<void> {
    try {
      notice =
        (
          await mutate<Schema['RAMConfigUpdateResultDTO']>(api, '/v1/config/ram', {
            maxRamGB: ramMax,
          })
        ).message ?? 'RAM settings saved.';
    } catch (error) {
      notice = errorMessage(error);
    }
  }
  async function saveGeyser(): Promise<void> {
    try {
      notice = (
        await mutate<Schema['GeyserConfigUpdateResultDTO']>(api, '/v1/config/geyser', {
          address: geyserAddress,
          port: geyserPort,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Server configuration"
    title="Settings"
    description="Fields come from the agent's schema. The client does not carry a closed list of server.properties keys."
    status={settings.editable ? 'Editable' : 'Read-only'}
    statusTone={settings.editable ? 'positive' : 'warning'}
    actionLabel="Save changes"
    onAction={saveSettings}
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  {#if settings.note}<CapabilityNotice message={settings.note} />{/if}
  {#each settings.sections as section (section.id)}<section class="screen-card">
      <h3>{section.title}</h3>
      <div class="form-grid" style="margin-top: .7rem">
        {#each section.fields as field (field.key)}<div class="field">
            <label for={`setting-${field.key}`}>{field.label}</label
            >{#if field.options?.length}<select
                id={`setting-${field.key}`}
                value={changes[field.key] ?? field.value}
                onchange={(event) =>
                  (changes = {
                    ...changes,
                    [field.key]: (event.currentTarget as HTMLSelectElement).value,
                  })}
                >{#each field.options as option}<option value={option.value}>{option.label}</option
                  >{/each}</select
              >{:else}<input
                id={`setting-${field.key}`}
                value={changes[field.key] ?? field.value}
                type={field.type === 'int' || field.type === 'number' ? 'number' : 'text'}
                maxlength={field.maxLength}
                min={field.minInt}
                max={field.maxInt}
                oninput={(event) =>
                  (changes = {
                    ...changes,
                    [field.key]: (event.currentTarget as HTMLInputElement).value,
                  })}
              />{/if}{#if field.helpId}<small class="field-help">Help: {field.helpId}</small>{/if}
          </div>{/each}
      </div>
    </section>{:else}<CapabilityNotice
      title="No setting schema loaded"
      message="Connect to an agent to receive the current server.properties fields and their validation bounds."
    />{/each}
  <div class="screen-grid">
    <section class="screen-card">
      <h3>Java executable</h3>
      <p>Host-wide Java selection is separate from each server's Java family.</p>
      <div class="inline-form">
        <div class="field">
          <label for="java-path">Executable path</label><input
            id="java-path"
            bind:value={javaPath}
            placeholder="Agent default"
          />
        </div>
        <ActionButton label="Save Java" onclick={saveJava}>Save</ActionButton>
      </div>
    </section>
    <section class="screen-card">
      <h3>RAM allocation</h3>
      <div class="inline-form">
        <div class="field">
          <label for="ram-max">Maximum GB</label><input
            id="ram-max"
            type="number"
            min="1"
            bind:value={ramMax}
          />
        </div>
        <ActionButton label="Save RAM" onclick={saveRam}>Save</ActionButton>
      </div>
    </section>
  </div>
  <section class="screen-card">
    <h3>Geyser listener</h3>
    <p>
      Only the managed top-level listener fields are shown; the rest of Geyser's YAML remains
      Geyser-owned.
    </p>
    <div class="form-grid">
      <div class="field">
        <label for="geyser-address">Address</label><input
          id="geyser-address"
          bind:value={geyserAddress}
        />
      </div>
      <div class="field">
        <label for="geyser-port">Port</label><input
          id="geyser-port"
          type="number"
          bind:value={geyserPort}
        />
      </div>
    </div>
    <ActionButton label="Save Geyser" onclick={saveGeyser}>Save</ActionButton>
  </section>
</div>
