<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import { PERMISSION_ASKS } from '../lib/format';
  import type { IconName } from '../lib/icons';
  import type { PermissionPrompt } from '../lib/types';

  let { data, onresolve }: { data: PermissionPrompt; onresolve: (allow: boolean, remember: boolean) => void } = $props();
  let remember = $state(true);

  const icons: Record<string, IconName> = {
    location: 'globe',
    camera: 'image',
    microphone: 'volume',
    notifications: 'info',
    clipboard: 'copy',
    'automatic-downloads': 'download',
    'file-editing': 'edit',
  };

  const host = $derived.by(() => {
    try {
      return new URL(data.origin).host;
    } catch {
      return data.origin;
    }
  });
</script>

<div class="prompt">
  <div class="head">
    <span class="ico"><Icon name={icons[data.kind] ?? 'shield'} size={16} /></span>
    <p><strong>{host}</strong> wants to {PERMISSION_ASKS[data.kind] ?? 'use a device feature'}.</p>
  </div>
  <label class="remember">
    <input type="checkbox" bind:checked={remember} />
    Remember my decision for this site
  </label>
  <div class="actions">
    <button class="btn" onclick={() => onresolve(false, remember)}>Block</button>
    <button class="btn primary" data-autofocus onclick={() => onresolve(true, remember)}>Allow</button>
  </div>
</div>

<style>
  .prompt {
    padding: 16px 18px 14px;
  }
  .head {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .ico {
    width: 30px;
    height: 30px;
    flex: none;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  p {
    margin: 4px 0 0;
    line-height: 1.5;
  }
  strong {
    font-weight: 600;
    word-break: break-all;
  }
  .remember {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 14px 0 0 42px;
    color: var(--muted);
  }
  .remember input {
    accent-color: var(--accent);
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
</style>
