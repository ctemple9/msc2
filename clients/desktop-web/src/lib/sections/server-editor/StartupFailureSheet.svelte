<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import StartupFailurePanel from './StartupFailurePanel.svelte';
  import type { Schema, ScreenApi } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverName = 'Server';
  export let serverRunning = false;
  export let operationKind: 'initiate' | 'start' = 'start';
  export let errorCode = '';
  export let failureMessage = '';
  export let problems: Schema['StartupProblemDTO'][] = [];
  export let visible = false;
  export let onClose: () => void = () => {};
  export let onRetry: () => void | Promise<void> = () => {};

  $: title = errorCode === 'geyser_plugin_failed'
    ? `${problems[0]?.offenderName ?? 'Geyser'} did not load`
    : `${serverName} startup issue`;
</script>

<Sheet {title} size="md" {visible} {onClose}>
  <StartupFailurePanel
    {api}
    {serverName}
    {operationKind}
    {serverRunning}
    {errorCode}
    {failureMessage}
    {problems}
    {onRetry}
  />
</Sheet>
