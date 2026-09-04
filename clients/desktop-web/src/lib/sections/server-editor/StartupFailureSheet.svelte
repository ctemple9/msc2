<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import StartupFailurePanel from './StartupFailurePanel.svelte';
  import type { Schema, ScreenApi } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverName = 'Server';
  export let operationKind: 'initiate' | 'start' = 'start';
  export let errorCode = '';
  export let failureMessage = '';
  export let visible = false;
  export let onClose: () => void = () => {};
  export let onRetry: () => void | Promise<void> = () => {};

  $: title = `${serverName} startup issue`;
</script>

<Sheet {title} size="md" {visible} {onClose}>
  <StartupFailurePanel
    {api}
    {serverName}
    {operationKind}
    {errorCode}
    {failureMessage}
    onRetry={async () => {
      await onRetry();
      onClose();
    }}
  />
</Sheet>
