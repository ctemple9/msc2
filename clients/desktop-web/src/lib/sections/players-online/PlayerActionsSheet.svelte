<script lang="ts">
  // These are Minecraft's built-in player commands across MSC's 1.20+ floor.
  // Edition-specific names and the active-server guard are resolved before dispatch;
  // a mod or plugin may still replace a command, so only submission is reported.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import { ApiError } from '../../api/client';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';

  type Action =
    | 'message'
    | 'teleport'
    | 'gamemode'
    | 'kick'
    | 'ban'
    | 'clear'
    | 'kill'
    | 'op'
    | 'deop'
    | 'accessAdd'
    | 'accessRemove';

  export let player: Schema['PlayerDTO'];
  export let onlinePlayers: readonly Schema['PlayerDTO'][] = [];
  export let serverId: string;
  export let serverType: string;
  export let minecraftVersion: string | undefined = undefined;
  export let api: ScreenApi | undefined = undefined;
  export let initialAction: Action | undefined = undefined;
  export let canSendCommands = true;
  export let canEditAccess = true;
  export let online = true;
  export let onClose: () => void;
  export let onAllowlistChanged: (entries: Schema['AllowlistEntryDTO'][]) => void = () => undefined;

  const actions: readonly { id: Action; label: string; description: string }[] = [
    { id: 'message', label: 'Message', description: 'Send a private message from the server' },
    { id: 'teleport', label: 'Teleport', description: 'Move to a player or coordinates' },
    {
      id: 'gamemode',
      label: 'Change game mode',
      description: 'Survival, Creative, Adventure, or Spectator',
    },
    { id: 'kick', label: 'Kick', description: 'Disconnect this player' },
    { id: 'ban', label: 'Ban', description: 'Block this Java player from rejoining' },
    { id: 'clear', label: 'Clear inventory', description: 'Remove every item this player carries' },
    { id: 'kill', label: 'Kill', description: 'Kill this player in game' },
    { id: 'op', label: 'Grant operator', description: 'Give server operator access' },
    { id: 'deop', label: 'Revoke operator', description: 'Remove server operator access' },
    {
      id: 'accessAdd',
      label: 'Add to access list',
      description: 'Allow this player to join when access is restricted',
    },
    {
      id: 'accessRemove',
      label: 'Remove from access list',
      description: 'Remove this player from the whitelist or allowlist',
    },
  ];
  const groups: readonly { label: string; ids: readonly Action[] }[] = [
    { label: 'Common', ids: ['message', 'teleport', 'gamemode'] },
    { label: 'Player state', ids: ['clear', 'kill'] },
    {
      label: 'Moderation and access',
      ids: ['kick', 'ban', 'op', 'deop', 'accessAdd', 'accessRemove'],
    },
  ];

  let selected: Action | undefined = initialAction;
  let message = '';
  let reason = '';
  let gameMode = 'survival';
  let destinationKind: 'player' | 'coordinates' = 'player';
  let destinationPlayer = '';
  let x = '';
  let y = '';
  let z = '';
  let confirming = false;
  let busy = false;
  let error = '';
  let notice = '';
  let safety: { token: string; message: string; command: string } | undefined;

  $: isBedrock = serverType === 'bedrock';
  $: accessLabel = isBedrock ? 'allowlist' : 'whitelist';
  $: destinations = onlinePlayers.filter((candidate) => candidate.name !== player.name);
  $: selectedLabel = labelFor(selected);

  function labelFor(action: Action | undefined): string {
    if (action === 'accessAdd') return `Add to ${accessLabel}`;
    if (action === 'accessRemove') return `Remove from ${accessLabel}`;
    return actions.find((candidate) => candidate.id === action)?.label ?? '';
  }

  function choose(action: Action): void {
    selected = action;
    confirming = false;
    error = '';
    notice = '';
    safety = undefined;
  }

  function target(name: string): string {
    // Java proxy players can have nonstandard names, while Bedrock gamertags
    // may contain spaces. Quote both instead of narrowing the target to a
    // vanilla Java username pattern; never allow selectors or another line.
    if (!name || name.startsWith('@') || /["\\\x00-\x1f]/.test(name))
      throw new Error('This player name cannot be used in a command.');
    return /^[A-Za-z0-9_]{3,16}$/.test(name) ? name : `"${name}"`;
  }

  function oneLine(value: string, label: string, maxLength: number): string {
    const trimmed = value.trim();
    if (!trimmed) throw new Error(`${label} is required.`);
    if (trimmed.length > maxLength || /[\x00-\x1f\x7f]/.test(trimmed))
      throw new Error(`${label} must be one line of at most ${maxLength} characters.`);
    return trimmed;
  }

  function coordinate(value: string, label: string): string {
    const trimmed = value.trim();
    if (!/^-?\d+(?:\.\d+)?$/.test(trimmed) || Math.abs(Number(trimmed)) > 30000000)
      throw new Error(`${label} must be a coordinate between -30000000 and 30000000.`);
    return trimmed;
  }

  function commandFor(action: Action): string {
    const name = target(player.name);
    switch (action) {
      case 'message':
        return `tell ${name} ${oneLine(message, 'Message', 256)}`;
      case 'teleport': {
        const destination =
          destinationKind === 'player'
            ? target(
                destinations.find((candidate) => candidate.name === destinationPlayer)?.name ?? '',
              )
            : `${coordinate(x, 'X')} ${coordinate(y, 'Y')} ${coordinate(z, 'Z')}`;
        return `tp ${name} ${destination}`;
      }
      case 'gamemode':
        return `gamemode ${gameMode} ${name}`;
      case 'kick':
      case 'ban':
        return `${action} ${name}${reason.trim() ? ` ${oneLine(reason, 'Reason', 160)}` : ''}`;
      case 'clear':
      case 'kill':
      case 'op':
      case 'deop':
        return `${action} ${name}`;
      case 'accessAdd':
      case 'accessRemove':
        return `${accessLabel} ${action === 'accessAdd' ? 'add' : 'remove'} ${name}`;
    }
  }

  function needsConfirmation(action: Action): boolean {
    return ['kick', 'ban', 'clear', 'kill', 'op', 'deop', 'accessRemove'].includes(action);
  }

  function requiresOnline(action: Action): boolean {
    return ['message', 'teleport', 'gamemode', 'kick', 'clear', 'kill'].includes(action);
  }

  async function send(): Promise<void> {
    if (!selected || busy) return;
    error = '';
    notice = '';
    if (!online && requiresOnline(selected)) {
      error = `${player.name} is no longer online.`;
      return;
    }
    if (selected === 'ban' && isBedrock) return;
    if (
      (selected === 'accessAdd' || selected === 'accessRemove') && isBedrock
        ? !canEditAccess
        : !canSendCommands
    ) {
      error = 'This account does not have permission for that action.';
      return;
    }
    let command: string;
    try {
      command = commandFor(selected);
    } catch (cause) {
      error = errorMessage(cause);
      return;
    }
    if (needsConfirmation(selected) && !confirming) {
      confirming = true;
      return;
    }

    busy = true;
    try {
      if (isBedrock && (selected === 'accessAdd' || selected === 'accessRemove')) {
        const result = await mutate<Schema['AllowlistMutationResultDTO']>(api, '/v1/allowlist', {
          action: selected === 'accessAdd' ? 'add' : 'remove',
          name: player.name,
          expectedActiveServerId: serverId,
        });
        onAllowlistChanged(result.entries);
        notice = `${player.name} ${selected === 'accessAdd' ? 'added to' : 'removed from'} the allowlist.`;
      } else {
        await mutate<Schema['CommandResult']>(api, '/v1/command', {
          command,
          expectedActiveServerId: serverId,
          ...(safety?.command === command ? { confirmation: safety.token } : {}),
        });
        notice = 'Command sent. Check the console for Minecraft’s result.';
      }
      confirming = false;
      safety = undefined;
    } catch (cause) {
      if (cause instanceof ApiError && cause.error.code === 'confirmation_required') {
        const details = cause.error.details as Record<string, unknown> | null | undefined;
        const prompt = details?.confirmation as Record<string, unknown> | undefined;
        if (typeof prompt?.acknowledgement === 'string' && typeof prompt.message === 'string') {
          safety = { token: prompt.acknowledgement, message: prompt.message, command };
        } else {
          error = errorMessage(cause);
        }
      } else {
        error = errorMessage(cause);
      }
    } finally {
      busy = false;
    }
  }
</script>

<Sheet title={`Player actions · ${player.name}`} size="sm" {onClose}>
  <div class="context">
    <span class="online-dot" aria-hidden="true"></span>
    <span>{online ? 'Online now' : 'Player left the server'}</span>
    <span class="edition"
      >{isBedrock ? 'Bedrock' : 'Java'}{minecraftVersion ? ` · ${minecraftVersion}` : ''}</span
    >
  </div>

  {#if !selected}
    {#each groups as group (group.label)}
      <div class="group-label msc2-type-overline">{group.label}</div>
      <div class="action-list">
        {#each actions.filter((action) => group.ids.includes(action.id) && (action.id !== 'ban' || !isBedrock)) as action (action.id)}
          <button
            type="button"
            class="action"
            disabled={(!online && requiresOnline(action.id)) ||
              ((action.id === 'accessAdd' || action.id === 'accessRemove') && isBedrock
                ? !canEditAccess
                : !canSendCommands)}
            onclick={() => choose(action.id)}
          >
            <span class="action-text"
              ><strong>{labelFor(action.id)}</strong><small>{action.description}</small></span
            >
            <span class="chevron" aria-hidden="true">›</span>
          </button>
        {/each}
      </div>
    {/each}
  {:else}
    <button type="button" class="back" onclick={() => (selected = undefined)}>‹ All actions</button>
    <h3>{selectedLabel}</h3>
    <p class="recipient">Player: {player.name}</p>

    {#if selected === 'message'}
      <span class="field-label">Message</span>
      <Field bind:value={message} multiline placeholder="Write a private message" />
    {:else if selected === 'teleport'}
      <label for="destination-kind">Destination</label>
      <select id="destination-kind" bind:value={destinationKind}>
        <option value="player">Another online player</option>
        <option value="coordinates">Coordinates</option>
      </select>
      {#if destinationKind === 'player'}
        <select aria-label="Destination player" bind:value={destinationPlayer}>
          <option value="">Choose a player</option>
          {#each destinations as destination (destination.name)}
            <option value={destination.name}>{destination.name}</option>
          {/each}
        </select>
      {:else}
        <div class="coordinates">
          <input aria-label="X coordinate" placeholder="X" bind:value={x} />
          <input aria-label="Y coordinate" placeholder="Y" bind:value={y} />
          <input aria-label="Z coordinate" placeholder="Z" bind:value={z} />
        </div>
      {/if}
    {:else if selected === 'gamemode'}
      <label for="player-gamemode">Game mode</label>
      <select id="player-gamemode" bind:value={gameMode}>
        <option value="survival">Survival</option>
        <option value="creative">Creative</option>
        <option value="adventure">Adventure</option>
        <option value="spectator">Spectator</option>
      </select>
    {:else if selected === 'kick' || selected === 'ban'}
      <span class="field-label">Reason (optional)</span>
      <Field bind:value={reason} placeholder="Reason shown to the player" />
    {/if}

    {#if selected === 'clear'}
      <p class="impact">This removes every item in {player.name}’s inventory.</p>
    {:else if selected === 'kill'}
      <p class="impact">This kills {player.name} in the game.</p>
    {:else if selected === 'op'}
      <p class="impact">This grants {player.name} operator access to server commands.</p>
    {:else if selected === 'accessRemove'}
      <p class="impact">This removes {player.name} from the {accessLabel}.</p>
    {/if}

    {#if confirming}<p class="confirm">
        Confirm {selectedLabel.toLowerCase()} for {player.name}.
      </p>{/if}
    {#if safety}<p class="confirm" role="alert">{safety.message}</p>{/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}
    <div class="footer">
      <Button
        variant={selected === 'clear' || selected === 'kill' || selected === 'ban'
          ? 'destructive'
          : 'primary'}
        disabled={busy || (!online && requiresOnline(selected))}
        onclick={() => void send()}
        >{busy
          ? 'Sending…'
          : safety
            ? 'Confirm and send'
            : confirming
              ? `Confirm ${selectedLabel}`
              : selected === 'message'
                ? 'Send message'
                : selectedLabel}</Button
      >
    </div>
  {/if}
</Sheet>

<style>
  .context {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    margin-bottom: 14px;
  }
  .online-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--msc2-status-ok);
  }
  .edition {
    margin-left: auto;
    color: var(--msc2-text-tertiary);
  }
  .action-list {
    display: flex;
    flex-direction: column;
  }
  .group-label {
    color: var(--msc2-text-tertiary);
    margin: 18px 0 4px;
  }
  .context + .group-label {
    margin-top: 0;
  }
  .action {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--msc2-hairline-faint);
    color: var(--msc2-text-primary);
    padding: 10px 2px;
    cursor: pointer;
  }
  .action:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.04);
  }
  .action:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .action-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .action strong {
    font-size: 12px;
    font-weight: 500;
  }
  .action small {
    font-size: 10px;
    font-weight: 400;
    color: var(--msc2-text-tertiary);
  }
  .chevron {
    color: var(--msc2-text-tertiary);
  }
  .back {
    border: 0;
    padding: 0;
    background: none;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    cursor: pointer;
  }
  h3 {
    font-size: 15px;
    font-weight: 500;
    margin: 14px 0 4px;
  }
  .recipient {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    margin: 0 0 18px;
  }
  label,
  .field-label {
    display: block;
    font-size: 11px;
    color: var(--msc2-text-secondary);
    margin: 12px 0 6px;
  }
  select,
  input {
    box-sizing: border-box;
    width: 100%;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    background: var(--msc2-tier-chrome);
    color: var(--msc2-text-primary);
    padding: 8px 10px;
    font: inherit;
    font-size: 12px;
  }
  select + select {
    margin-top: 8px;
  }
  .coordinates {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }
  .impact,
  .confirm,
  .error,
  .notice {
    font-size: 12px;
    line-height: 1.5;
    margin: 14px 0 0;
  }
  .impact {
    color: var(--msc2-text-secondary);
  }
  .confirm {
    color: var(--msc2-status-warn);
  }
  .error {
    color: var(--msc2-status-error);
  }
  .notice {
    color: var(--msc2-status-ok);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 20px;
  }
</style>
