<script lang="ts">
  // Dev-only component gallery — not a product screen. Renders the S0 base
  // kit for visual comparison against docs/msc2/renderings/*.html. Never
  // linked from the shipped app; opened directly via gallery.html in dev.
  import Card from '../lib/components/base/Card.svelte';
  import Button from '../lib/components/base/Button.svelte';
  import SegmentedControl from '../lib/components/base/SegmentedControl.svelte';
  import Toggle from '../lib/components/base/Toggle.svelte';
  import Field from '../lib/components/base/Field.svelte';
  import NumberField from '../lib/components/base/NumberField.svelte';
  import Select from '../lib/components/base/Select.svelte';
  import Badge from '../lib/components/base/Badge.svelte';
  import ListRow from '../lib/components/base/ListRow.svelte';
  import EmptyState from '../lib/components/base/EmptyState.svelte';
  import StatusDot from '../lib/components/base/StatusDot.svelte';
  import Sheet from '../lib/components/base/Sheet.svelte';

  let difficulty = 'normal';
  let pvp = true;
  let hardcore = false;
  let sheetOpen = false;
</script>

<div class="page">
  <h1 class="msc2-type-page">S0 component gallery</h1>
  <p class="msc2-type-body intro">
    Compare each group below against its locked reference in
    <code>docs/msc2/renderings/</code>.
  </p>

  <section>
    <h2 class="msc2-type-overline">Card + status dot</h2>
    <div class="row">
      <Card>
        <div class="card-head">
          <span class="card-icon" aria-hidden="true"></span>
          <span class="msc2-type-card">Components</span>
        </div>
        <StatusDot tone="ok" label="OK" />
        <p class="msc2-type-meta meta-line">1 component — up to date</p>
      </Card>
      <Card>
        <div class="card-head">
          <span class="card-icon" aria-hidden="true"></span>
          <span class="msc2-type-card">Port</span>
        </div>
        <StatusDot tone="warn" label="Warn" />
        <p class="msc2-type-meta meta-line">Port 25565 (TCP)</p>
      </Card>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Buttons — filled</h2>
    <div class="row">
      <Button variant="primary">Save changes</Button>
      <Button variant="start">Start</Button>
      <Button variant="stop">Stop</Button>
    </div>
    <h2 class="msc2-type-overline">Buttons — quiet</h2>
    <div class="row">
      <Button variant="secondary">Manage…</Button>
      <Button variant="destructive">Delete</Button>
      <Button variant="ghost-icon" label="Refresh">↻</Button>
      <Button variant="secondary" size="sm">Logs</Button>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Type — 7 roles</h2>
    <div class="type-rows">
      <span class="msc2-type-page">Survival</span>
      <span class="msc2-type-section">Server health</span>
      <span class="msc2-type-card">Connection info</span>
      <span class="msc2-type-body"
        >The background agent keeps servers running after this window closes.</span
      >
      <span class="msc2-type-meta">/Users/cameron/MinecraftServers/java/test · port 25565</span>
      <span class="msc2-type-overline">Server controls</span>
      <span class="msc2-type-mono">[02:11:43 INFO]: Done (5.231s)! For help, type "help"</span>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Segmented control</h2>
    <SegmentedControl
      options={[
        { value: 'peaceful', label: 'Peaceful' },
        { value: 'easy', label: 'Easy' },
        { value: 'normal', label: 'Normal' },
        { value: 'hard', label: 'Hard' },
      ]}
      bind:value={difficulty}
    />
  </section>

  <section>
    <h2 class="msc2-type-overline">Toggles</h2>
    <div class="toggle-row">
      <span class="msc2-type-body">PvP</span>
      <Toggle bind:checked={pvp} label="PvP" />
    </div>
    <div class="toggle-row">
      <span class="msc2-type-body">Hardcore</span>
      <Toggle bind:checked={hardcore} label="Hardcore" />
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Field · number · select</h2>
    <div class="field-col">
      <Field value="Test" />
      <div class="row">
        <NumberField value={20} />
        <Select
          options={[
            { value: 'normal', label: 'Normal' },
            { value: 'hard', label: 'Hard' },
          ]}
          value="normal"
        />
      </div>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Badges</h2>
    <div class="row">
      <Badge variant="category">Java</Badge>
      <Badge variant="category">Paper</Badge>
      <Badge variant="status" tone="ok">Active</Badge>
      <Badge variant="status" tone="ok">Up to date</Badge>
      <Badge variant="status" tone="error">Missing</Badge>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">List rows (divided, not carded)</h2>
    <div class="narrow">
      <Card padding="0">
        <ListRow title="config" subtitle="2 minutes ago" />
        <ListRow title="server.properties" subtitle="2 minutes ago · 1 KB" last />
      </Card>
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Empty state</h2>
    <div class="narrow">
      <EmptyState title="No resource packs" message="Add a .zip pack, or drag one here." />
    </div>
  </section>

  <section>
    <h2 class="msc2-type-overline">Sheet</h2>
    <Button variant="secondary" onclick={() => (sheetOpen = true)}>Open sheet</Button>
    {#if sheetOpen}
      <Sheet title="Manage servers" size="md" onClose={() => (sheetOpen = false)}>
        <p class="msc2-type-body">Sheet body content goes here.</p>
      </Sheet>
    {/if}
  </section>
</div>

<style>
  .page {
    background: var(--msc2-tier-atmosphere);
    min-height: 100vh;
    padding: 32px;
    color: var(--msc2-text-primary);
  }

  .intro {
    color: var(--msc2-text-secondary);
    margin-bottom: 32px;
  }

  section {
    margin-bottom: 32px;
  }

  section h2 {
    margin: 0 0 12px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 9px;
  }

  .card-icon {
    width: 17px;
    height: 17px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.45);
  }

  .meta-line {
    margin: 4px 0 0;
  }

  .type-rows {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    max-width: 220px;
    margin-bottom: 10px;
  }

  .field-col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 320px;
  }

  .narrow {
    max-width: 280px;
  }
</style>
