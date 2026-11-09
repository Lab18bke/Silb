<script lang="ts">
  import { onMount } from 'svelte';
  import PageShell from './PageShell.svelte';
  import Favicon from '../components/Favicon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import { api, errorText } from '../lib/api';
  import { dayLabel, hostOf, timeOfDay } from '../lib/format';
  import { browser } from '../chrome/state.svelte';
  import { bus, openInternal, openUrl } from '../chrome/actions';
  import { toast } from '../chrome/popover';
  import type { HistoryEntry } from '../lib/types';

  const PAGE = 120;
  let search = $state('');
  let entries = $state<HistoryEntry[]>([]);
  let done = $state(false);
  let loading = $state(false);
  let selected = $state(new Set<number>());
  let seq = 0;
  let timer: number | undefined;
  let sentinel = $state<HTMLElement>();

  const groups = $derived.by(() => {
    const out: { label: string; items: HistoryEntry[] }[] = [];
    for (const e of entries) {
      const label = dayLabel(e.visitedAt);
      const last = out[out.length - 1];
      if (last?.label === label) last.items.push(e);
      else out.push({ label, items: [e] });
    }
    return out;
  });

  async function load(reset: boolean) {
    if (loading && !reset) return;
    const n = ++seq;
    loading = true;
    try {
      const before = reset ? undefined : entries[entries.length - 1]?.visitedAt;
      const batch = await api.historyQuery(search, before, PAGE);
      if (n !== seq) return;
      entries = reset ? batch : [...entries, ...batch];
      done = batch.length < PAGE;
      if (reset) selected = new Set();
    } finally {
      if (n === seq) loading = false;
    }
  }

  function onSearch() {
    clearTimeout(timer);
    timer = window.setTimeout(() => load(true), 160);
  }

  function toggle(id: number) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  async function remove(ids: number[]) {
    try {
      await api.historyDelete(ids);
      const gone = new Set(ids);
      entries = entries.filter((e) => !gone.has(e.id));
      selected = new Set([...selected].filter((id) => !gone.has(id)));
    } catch (e) {
      toast(errorText(e));
    }
  }

  function open(e: MouseEvent, entry: HistoryEntry) {
    e.preventDefault();
    openUrl(entry.url, e.ctrlKey || e.button === 1 ? 'background' : 'current');
  }

  onMount(() => {
    load(true);
    const reload = () => load(true);
    bus.addEventListener('reload-page', reload);
    return () => bus.removeEventListener('reload-page', reload);
  });

  $effect(() => {
    if (!sentinel) return;
    const io = new IntersectionObserver((list) => {
      if (list.some((x) => x.isIntersecting) && !done && entries.length) load(false);
    });
    io.observe(sentinel);
    return () => io.disconnect();
  });
</script>

<PageShell title="History" bind:search searchLabel="Search history" {onSearch}>
  {#snippet actions()}
    {#if selected.size}
      <button class="btn danger" onclick={() => remove([...selected])}>Delete {selected.size} selected</button>
      <button class="btn quiet" onclick={() => (selected = new Set())}>Cancel</button>
    {:else}
      <button class="btn" onclick={() => openInternal('settings/privacy/clear')}>Clear browsing data…</button>
    {/if}
  {/snippet}

  {#if browser.isPrivate}
    <p class="note">This is your regular history. Pages you visit in private windows are not added to it.</p>
  {/if}

  {#each groups as group (group.label)}
    <section>
      <h2>{group.label}</h2>
      <ul>
        {#each group.items as entry (entry.id)}
          <li class:checked={selected.has(entry.id)}>
            <input type="checkbox" checked={selected.has(entry.id)} onchange={() => toggle(entry.id)} aria-label="Select {entry.title || entry.url}" />
            <span class="time">{timeOfDay(entry.visitedAt)}</span>
            <Favicon src={entry.favicon} size={16} />
            <a href={entry.url} onclick={(e) => open(e, entry)} onauxclick={(e) => open(e, entry)} title={entry.url}>
              <span class="title">{entry.title || entry.url}</span>
              <span class="host">{hostOf(entry.url)}</span>
            </a>
            <IconButton icon="close-small" label="Remove from history" onclick={() => remove([entry.id])} />
          </li>
        {/each}
      </ul>
    </section>
  {:else}
    {#if !loading}
      <div class="empty">
        {search ? `No history matches “${search}”.` : 'Pages you visit will appear here.'}
      </div>
    {/if}
  {/each}
  <div bind:this={sentinel} class="sentinel"></div>
</PageShell>

<style>
  .note {
    margin: -10px 0 22px;
    color: var(--muted);
  }
  section + section {
    margin-top: 26px;
  }
  h2 {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    margin: 0 0 6px;
    padding: 0 10px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 4px 0 10px;
    border-radius: var(--radius-s);
  }
  li:hover,
  li.checked {
    background: var(--hover);
  }
  li input {
    margin: 0;
    accent-color: var(--accent);
    opacity: 0;
  }
  li:hover input,
  li.checked input,
  li input:focus-visible {
    opacity: 1;
  }
  li :global(.icon-btn) {
    opacity: 0;
  }
  li:hover :global(.icon-btn) {
    opacity: 1;
  }
  .time {
    width: 62px;
    flex: none;
    color: var(--faint);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  a {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 10px;
    text-decoration: none;
    color: var(--ink);
  }
  .title {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .host {
    flex: none;
    color: var(--muted);
    font-size: 12px;
  }
  .empty {
    padding: 60px 0;
    text-align: center;
    color: var(--muted);
  }
  .sentinel {
    height: 1px;
  }
</style>
