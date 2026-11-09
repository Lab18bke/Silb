<script lang="ts">
  import { onMount } from 'svelte';
  import PageShell from './PageShell.svelte';
  import DownloadRow from '../components/DownloadRow.svelte';
  import { api, errorText } from '../lib/api';
  import { on } from '../lib/events';
  import { dayLabel } from '../lib/format';
  import { settings } from '../lib/settings.svelte';
  import { browser } from '../chrome/state.svelte';
  import { openInternal } from '../chrome/actions';
  import { toast } from '../chrome/popover';
  import type { AppInfo, DownloadRecord } from '../lib/types';

  let list = $state<DownloadRecord[]>([]);
  let search = $state('');
  let info = $state<AppInfo | null>(null);

  const filtered = $derived(
    search.trim()
      ? list.filter((d) => (d.fileName + ' ' + d.url).toLowerCase().includes(search.trim().toLowerCase()))
      : list,
  );

  const groups = $derived.by(() => {
    const out: { label: string; items: DownloadRecord[] }[] = [];
    for (const d of filtered) {
      const label = dayLabel(d.startedAt);
      const last = out[out.length - 1];
      if (last?.label === label) last.items.push(d);
      else out.push({ label, items: [d] });
    }
    return out;
  });

  async function load() {
    list = await api.downloadsList();
  }

  async function clearAll() {
    try {
      await api.downloadsClear();
      await load();
    } catch (e) {
      toast(errorText(e));
    }
  }

  onMount(() => {
    load();
    api.appInfo().then((i) => (info = i));
    const offs = [
      on<DownloadRecord>('download-updated', (d) => {
        const i = list.findIndex((x) => x.id === d.id);
        if (i >= 0) list[i] = d;
        else list = [d, ...list];
      }),
      on('downloads-cleared', load),
    ];
    return () => offs.forEach((o) => o());
  });
</script>

<PageShell title="Downloads" bind:search searchLabel="Search downloads">
  {#snippet actions()}
    <button class="btn" disabled={!list.some((d) => d.state !== 'in_progress')} onclick={clearAll}>Clear list</button>
  {/snippet}

  <p class="where">
    {#if browser.isPrivate}
      Files downloaded in private windows are saved to your downloads folder. This list is cleared when you close all private windows.
    {:else}
      Saving to <button class="linkish" onclick={() => openInternal('settings/downloads')}>{settings.value?.downloadDir || info?.downloadDir || 'your Downloads folder'}</button>.
      Clearing the list keeps the files.
    {/if}
  </p>

  {#each groups as group (group.label)}
    <section>
      <h2>{group.label}</h2>
      {#each group.items as d (d.id)}
        <DownloadRow {d} onerror={toast} />
      {/each}
    </section>
  {:else}
    <div class="empty">{search ? `No downloads match “${search}”.` : 'Files you download will appear here.'}</div>
  {/each}
</PageShell>

<style>
  .where {
    margin: -12px 0 24px;
    color: var(--muted);
  }
  .linkish {
    border: 0;
    padding: 0;
    background: none;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .linkish:hover {
    text-decoration: underline;
  }
  section + section {
    margin-top: 22px;
  }
  h2 {
    font-size: 13px;
    font-weight: 600;
    margin: 0 0 4px;
    padding: 0 12px;
  }
  .empty {
    padding: 60px 0;
    text-align: center;
    color: var(--muted);
  }
</style>
