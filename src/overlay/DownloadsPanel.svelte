<script lang="ts">
  import { onMount } from 'svelte';
  import DownloadRow from '../components/DownloadRow.svelte';
  import { api } from '../lib/api';
  import { on } from '../lib/events';
  import type { DownloadRecord } from '../lib/types';

  let { auto, onshowall, onclose }: { auto: boolean; onshowall: () => void; onclose: () => void } = $props();
  let list = $state<DownloadRecord[]>([]);
  let error = $state('');
  let hovering = false;
  let timer: number | undefined;

  const recent = $derived(list.slice(0, 5));

  function schedule() {
    clearTimeout(timer);
    if (auto) timer = window.setTimeout(() => !hovering && onclose(), 5000);
  }

  onMount(() => {
    api.downloadsList().then((l) => (list = l));
    schedule();
    const offs = [
      on<DownloadRecord>('download-updated', (d) => {
        const i = list.findIndex((x) => x.id === d.id);
        if (i >= 0) list[i] = d;
        else list = [d, ...list];
      }),
      on('downloads-cleared', () => api.downloadsList().then((l) => (list = l))),
    ];
    return () => {
      clearTimeout(timer);
      offs.forEach((o) => o());
    };
  });
</script>

<div
  class="panel"
  role="region"
  aria-label="Recent downloads"
  onmouseenter={() => {
    hovering = true;
    clearTimeout(timer);
  }}
  onmouseleave={() => {
    hovering = false;
    schedule();
  }}
>
  <div class="head">
    <h2>Downloads</h2>
  </div>
  {#if recent.length}
    <div class="rows">
      {#each recent as d (d.id)}
        <DownloadRow {d} compact onerror={(m) => (error = m)} />
      {/each}
    </div>
  {:else}
    <p class="empty">No downloads yet.</p>
  {/if}
  {#if error}<p class="error">{error}</p>{/if}
  <button type="button" class="all" data-autofocus onclick={onshowall}>Show all downloads</button>
</div>

<style>
  .panel {
    padding: 8px 6px 6px;
  }
  .head {
    padding: 6px 10px 6px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .empty {
    color: var(--muted);
    padding: 18px 10px;
    margin: 0;
  }
  .error {
    color: var(--danger);
    font-size: 12px;
    margin: 4px 10px;
  }
  .all {
    width: 100%;
    height: 36px;
    margin-top: 4px;
    border: 0;
    border-top: 1px solid var(--line);
    background: transparent;
    color: var(--accent);
    border-radius: 0 0 var(--radius-m) var(--radius-m);
  }
  .all:hover {
    background: var(--hover);
  }
</style>
