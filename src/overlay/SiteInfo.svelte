<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import { api, errorText } from '../lib/api';
  import { PERMISSION_LABELS } from '../lib/format';
  import type { SiteInfo } from '../lib/types';

  let { data, onclose }: { data: { tabId: number; info: SiteInfo }; onclose: () => void } = $props();
  let info = $state(untrack(() => data.info));
  let message = $state('');

  const shown = ['location', 'camera', 'microphone', 'notifications', 'clipboard', 'popups', 'javascript'];

  const connection = $derived.by(() => {
    switch (info.security) {
      case 'secure':
        return { icon: 'lock' as const, title: 'Connection is secure', text: 'Information you send to this site is encrypted in transit.' };
      case 'cert-error':
        return { icon: 'warning' as const, title: 'Certificate problem', text: 'This site’s certificate couldn’t be verified. Someone may be able to read or change information you send.', tone: 'danger' };
      case 'insecure':
        return { icon: 'warning' as const, title: 'Connection is not secure', text: 'Don’t enter passwords or payment details on this site. Information is sent unencrypted.', tone: 'warn' };
      default:
        return { icon: 'info' as const, title: 'Page information', text: '' };
    }
  });

  async function set(kind: string, value: string) {
    if (!info.origin) return;
    try {
      const v = value === 'default' ? null : (value as 'allow' | 'block');
      await api.siteSettingSet(info.origin, kind, v);
      info = { ...info, permissions: info.permissions.map(([k, old]) => [k, k === kind ? v : old]) };
      message = kind === 'javascript' || kind === 'popups' ? 'Reload the page to apply this change.' : '';
    } catch (e) {
      message = errorText(e);
    }
  }

  async function clearData() {
    try {
      await api.siteDataClear(info.host.replace(/^www\./, ''));
      message = 'Cookies and site data cleared. Reload the page to sign in again.';
    } catch (e) {
      message = errorText(e);
    }
  }
</script>

<div class="info">
  <div class="conn {connection.tone ?? ''}">
    <Icon name={connection.icon} size={16} />
    <div>
      <div class="title">{connection.title}</div>
      <div class="host">{info.host}</div>
      {#if connection.text}<p>{connection.text}</p>{/if}
    </div>
  </div>
  {#if info.origin}
    <div class="perms">
      {#each info.permissions.filter(([k]) => shown.includes(k)) as [kind, value] (kind)}
        <label>
          <span>{PERMISSION_LABELS[kind]}</span>
          <select class="field" value={value ?? 'default'} onchange={(e) => set(kind, (e.currentTarget as HTMLSelectElement).value)}>
            <option value="default">{kind === 'popups' || kind === 'javascript' ? 'Default' : 'Ask'}</option>
            <option value="allow">Allow</option>
            <option value="block">Block</option>
          </select>
        </label>
      {/each}
    </div>
    {#if message}<p class="msg">{message}</p>{/if}
    <div class="actions">
      <button class="btn quiet" onclick={clearData}><Icon name="cookie" size={14} />Clear site data</button>
      <button class="btn quiet" data-autofocus onclick={onclose}>Done</button>
    </div>
  {/if}
</div>

<style>
  .info {
    padding: 16px 16px 12px;
  }
  .conn {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    color: var(--ok);
  }
  .conn.warn {
    color: var(--warn);
  }
  .conn.danger {
    color: var(--danger);
  }
  .title {
    font-weight: 600;
    color: var(--ink);
  }
  .host {
    color: var(--muted);
    font-size: 12px;
    word-break: break-all;
  }
  .conn p {
    margin: 6px 0 0;
    color: var(--ink-2);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .perms {
    margin-top: 14px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  label span {
    flex: 1;
  }
  label .field {
    width: 112px;
    height: 28px;
  }
  .msg {
    margin: 10px 0 0;
    font-size: 12px;
    color: var(--accent);
  }
  .actions {
    display: flex;
    justify-content: space-between;
    margin-top: 12px;
  }
</style>
