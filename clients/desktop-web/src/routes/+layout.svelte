<script lang="ts">
  import ApplicationShell from '../lib/components/ApplicationShell.svelte';
  import type { SectionDescriptor } from '../lib/navigation/types';

  export let hostLabel = 'No host selected';
  export let serverLabel = 'No server selected';
  export let sections: readonly SectionDescriptor[] = [];
  export let activeSection = '';
</script>

<ApplicationShell {hostLabel} {serverLabel} {sections} {activeSection}>
  <slot />
</ApplicationShell>
