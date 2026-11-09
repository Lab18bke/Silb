<script lang="ts">
  import { onMount } from 'svelte';
  import { api, errorText } from '../../lib/api';
  import type { ClearKinds } from '../../lib/types';

  let { onclose, ondone }: { onclose: () => void; ondone: (msg: string) => void } = $props();

  let range = $state('3600000');
  let kinds = $state<ClearKinds>({ history: true, cookies: false, cache: true, downloads: false });
  let busy = $state(false);
  let error = $state('');
  let dialog: HTMLDialogElement;

  const options: { key: keyof ClearKinds; label: string; detail: string }[] = [
    { key: 'history', label: 'Browsing history', detail: 'Pages you visited, their titles, and suggestions based on them. Bookmarks are kept.' },
    { key: 'cookies', label: 'Cookies and other site data', detail: 'Signs you out of most sites. Includes local storage and other website databases. Bookmarks and settings are kept.' },
    { key: 'cache', label: 'Cached images and files', detail: 'Copies of page resources saved to load sites faster. Some sites may load more slowly next time.' },
    { key: 'downloads', label: 'Download history', detail: 'The list of downloads. Downloaded files stay on your computer.' },
  ];

  async function clear() {
    busy = true;
    error = '';
    try {
      const since = range === 'all' ? null : Date.now() - Number(range);
      await api.clearBrowsingData(kinds, since);
      ondone('Browsing data cleared');
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    dialog.showModal();
  });
</script>

<dialog bind:this={dialog} onclose={onclose} aria-labelledby="clear-title">
  <h2 id="clear-title">Clear browsing data</h2>
  <label class="range">
    Time range
    <select class="field" bind:value={range}>
      <option value="3600000">Last hour</option>
      <option value="86400000">Last 24 hours</option>
      <option value="604800000">Last 7 days</option>
      <option value="2419200000">Last 4 weeks</option>
      <option value="all">All time</option>
    </select>
  </label>
  <div class="opts">
    {#each options as o}
      <label class="opt">
        <input type="checkbox" bind:checked={kinds[o.key]} />
        <span>
          <span class="label">{o.label}</span>
          <span class="detail">{o.detail}</span>
        </span>
      </label>
    {/each}
  </div>
  <p class="note">Silb has no password manager, so there are no saved passwords to clear. Private windows keep their own data, which is deleted when they close.</p>
  {#if error}<p class="error">{error}</p>{/if}
  <div class="actions">
    <button class="btn quiet" onclick={() => dialog.close()}>Cancel</button>
    <button class="btn primary" disabled={busy || !Object.values(kinds).some(Boolean)} onclick={clear}>
      {busy ? 'Clearing…' : 'Clear data'}
    </button>
  </div>
</dialog>

<style>
  dialog {
    width: min(520px, calc(100% - 48px));
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    background: var(--raised);
    color: var(--ink);
    padding: 26px 26px 20px;
    box-shadow: var(--shadow-pop);
  }
  dialog::backdrop {
    background: rgba(10, 10, 10, 0.32);
  }
  h2 {
    font-family: var(--font-display);
    font-weight: 450;
    font-size: 24px;
    margin: 0 0 18px;
  }
  .range {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    font-weight: 500;
  }
  .opts {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--line);
  }
  .opt {
    display: flex;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--line);
  }
  .opt input {
    margin-top: 3px;
    accent-color: var(--accent);
  }
  .label {
    display: block;
    font-weight: 500;
  }
  .detail {
    display: block;
    color: var(--muted);
    font-size: 12.5px;
    margin-top: 2px;
  }
  .note {
    color: var(--muted);
    font-size: 12px;
    margin: 14px 0 0;
  }
  .error {
    color: var(--danger);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }
</style>
