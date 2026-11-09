<script lang="ts">
  import { api, errorText } from '../lib/api';
  import { hostOf } from '../lib/format';

  let { data, onclose }: { data: { tabId: number; urls: string[]; host: string }; onclose: () => void } = $props();
  let error = $state('');

  async function open(url: string | null, allow: boolean) {
    try {
      await api.tabOpenPopup(data.tabId, url, allow);
      onclose();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<div class="popups">
  <h2>Pop-ups blocked</h2>
  <p>This page tried to open {data.urls.length === 1 ? 'a window' : `${data.urls.length} windows`} without a click.</p>
  <ul>
    {#each data.urls.slice(-5) as url}
      <li><button type="button" title={url} onclick={() => open(url, false)}>{url}</button></li>
    {/each}
  </ul>
  {#if error}<p class="err">{error}</p>{/if}
  <div class="actions">
    <button class="btn" onclick={() => open(null, true)}>Always allow on {hostOf(data.host)}</button>
    <button class="btn primary" data-autofocus onclick={onclose}>Done</button>
  </div>
</div>

<style>
  .popups {
    padding: 16px 16px 12px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 600;
  }
  p {
    margin: 0 0 8px;
    color: var(--muted);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li button {
    width: 100%;
    height: 30px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--accent);
    text-align: left;
    padding: 0 6px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  li button:hover {
    background: var(--hover);
  }
  .err {
    color: var(--danger);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
  }
</style>
