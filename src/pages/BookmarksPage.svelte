<script lang="ts">
  import { onMount } from 'svelte';
  import PageShell from './PageShell.svelte';
  import Icon from '../components/Icon.svelte';
  import Favicon from '../components/Favicon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import { api, errorText } from '../lib/api';
  import { on } from '../lib/events';
  import { childrenOf } from '../lib/bookmarks';
  import { hostOf } from '../lib/format';
  import { browser } from '../chrome/state.svelte';
  import { openUrl } from '../chrome/actions';
  import { toast } from '../chrome/popover';
  import { BOOKMARKS_BAR, OTHER_BOOKMARKS, type Bookmark } from '../lib/types';

  let all = $state<Bookmark[]>([]);
  let folder = $state(BOOKMARKS_BAR);
  let search = $state('');
  let results = $state<Bookmark[] | null>(null);
  let editing = $state<{ id: number | 'new-folder' | 'new-bookmark'; title: string; url: string } | null>(null);
  let dragId = $state<number | null>(null);
  let dropTarget = $state<string | null>(null);
  let timer: number | undefined;

  const current = $derived(all.find((b) => b.id === folder));
  const items = $derived(results ?? childrenOf(all, folder));
  const path = $derived.by(() => {
    const out: Bookmark[] = [];
    let c = current;
    while (c) {
      out.unshift(c);
      c = all.find((b) => b.id === c!.parentId);
    }
    return out;
  });

  const tree = $derived.by(() => {
    const out: { b: Bookmark; depth: number }[] = [];
    const walk = (parent: number, depth: number) => {
      for (const b of childrenOf(all, parent)) {
        if (!b.isFolder) continue;
        out.push({ b, depth });
        walk(b.id, depth + 1);
      }
    };
    for (const id of [BOOKMARKS_BAR, OTHER_BOOKMARKS]) {
      const root = all.find((b) => b.id === id);
      if (root) {
        out.push({ b: root, depth: 0 });
        walk(id, 1);
      }
    }
    return out;
  });

  async function load() {
    all = await api.bookmarksAll();
    if (!all.some((b) => b.id === folder)) folder = BOOKMARKS_BAR;
    if (search.trim()) results = await api.bookmarksSearch(search);
  }

  function onSearch() {
    clearTimeout(timer);
    timer = window.setTimeout(async () => {
      results = search.trim() ? await api.bookmarksSearch(search) : null;
    }, 140);
  }

  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      toast(errorText(e));
    }
  }

  async function save() {
    if (!editing) return;
    const e = editing;
    await run(async () => {
      if (e.id === 'new-folder') await api.bookmarkCreate(folder, e.title.trim() || 'New folder', null);
      else if (e.id === 'new-bookmark') {
        const url = await normalize(e.url);
        await api.bookmarkCreate(folder, e.title.trim() || url, url);
      } else {
        const b = all.find((x) => x.id === e.id);
        await api.bookmarkUpdate(e.id, e.title.trim() || b?.title || '', b?.isFolder ? null : await normalize(e.url));
      }
      editing = null;
    });
  }

  async function normalize(url: string) {
    const u = url.trim();
    if (/^(https?|file):/i.test(u)) return u;
    if (/^[\w-]+(\.[\w-]+)+(\/.*)?$/.test(u)) return `https://${u}`;
    throw 'Enter a full web address, such as https://example.com';
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') save();
    if (e.key === 'Escape') editing = null;
  }

  function open(e: MouseEvent, b: Bookmark) {
    e.preventDefault();
    if (b.isFolder) {
      folder = b.id;
      search = '';
      results = null;
      return;
    }
    if (b.url) openUrl(b.url, e.ctrlKey || e.button === 1 ? 'background' : 'current');
  }

  function drop(target: { folder: number; index?: number }) {
    const id = dragId;
    dragId = null;
    dropTarget = null;
    if (id == null || id === target.folder) return;
    run(() => api.bookmarkMove(id, target.folder, target.index ?? 1 << 30));
  }

  function addCurrentPage() {
    const t = browser.activeTab;
    const recent = browser.tabs.find((x) => !x.internal && x.url.startsWith('http'));
    const source = t && !t.internal ? t : recent;
    editing = { id: 'new-bookmark', title: source?.title ?? '', url: source?.url ?? '' };
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
  }

  onMount(() => {
    load();
    return on('bookmarks-changed', load);
  });
</script>

<PageShell title="Bookmarks" bind:search searchLabel="Search bookmarks" {onSearch} wide>
  {#snippet actions()}
    <button class="btn" onclick={addCurrentPage}><Icon name="plus" size={14} />Add bookmark</button>
    <button class="btn" onclick={() => (editing = { id: 'new-folder', title: '', url: '' })}><Icon name="folder-plus" size={14} />New folder</button>
  {/snippet}

  <div class="layout">
    <nav class="tree" aria-label="Folders">
      {#each tree as { b, depth } (b.id)}
        <button
          type="button"
          class:active={!results && b.id === folder}
          class:drop={dropTarget === `f${b.id}`}
          style:padding-left="{10 + depth * 16}px"
          onclick={() => {
            folder = b.id;
            search = '';
            results = null;
          }}
          ondragover={(e) => {
            if (dragId != null && dragId !== b.id) {
              e.preventDefault();
              dropTarget = `f${b.id}`;
            }
          }}
          ondragleave={() => dropTarget === `f${b.id}` && (dropTarget = null)}
          ondrop={(e) => {
            e.preventDefault();
            drop({ folder: b.id });
          }}
        >
          <Icon name="folder" size={14} />
          <span>{b.title}</span>
        </button>
      {/each}
    </nav>

    <div class="list">
      {#if results}
        <p class="crumbs">{results.length} {results.length === 1 ? 'result' : 'results'}</p>
      {:else}
        <p class="crumbs">
          {#each path as p, i (p.id)}
            {#if i > 0}<Icon name="chevron-right" size={12} />{/if}
            <button type="button" onclick={() => (folder = p.id)}>{p.title}</button>
          {/each}
        </p>
      {/if}

      {#if editing && (editing.id === 'new-folder' || editing.id === 'new-bookmark')}
        <div class="row editing">
          <Icon name={editing.id === 'new-folder' ? 'folder' : 'bookmark'} size={16} />
          <input class="field" bind:value={editing.title} placeholder={editing.id === 'new-folder' ? 'Folder name' : 'Name'} onkeydown={onKey} use:focusOnMount />
          {#if editing.id === 'new-bookmark'}
            <input class="field url" bind:value={editing.url} placeholder="https://" onkeydown={onKey} />
          {/if}
          <button class="btn primary" onclick={save}>Save</button>
          <button class="btn quiet" onclick={() => (editing = null)}>Cancel</button>
        </div>
      {/if}

      <ul>
        {#each items as b, i (b.id)}
          <li
            class:drop={dropTarget === `i${b.id}`}
            draggable={!results && editing?.id !== b.id}
            ondragstart={(e) => {
              dragId = b.id;
              e.dataTransfer?.setData('text/plain', b.url ?? b.title);
            }}
            ondragend={() => {
              dragId = null;
              dropTarget = null;
            }}
            ondragover={(e) => {
              if (dragId != null && dragId !== b.id && !results) {
                e.preventDefault();
                dropTarget = `i${b.id}`;
              }
            }}
            ondrop={(e) => {
              e.preventDefault();
              drop({ folder, index: i });
            }}
          >
            {#if editing?.id === b.id}
              <div class="row editing">
                <Icon name={b.isFolder ? 'folder' : 'bookmark'} size={16} />
                <input class="field" bind:value={editing.title} onkeydown={onKey} aria-label="Name" use:focusOnMount />
                {#if !b.isFolder}
                  <input class="field url" bind:value={editing.url} onkeydown={onKey} aria-label="Address" />
                {/if}
                <button class="btn primary" onclick={save}>Save</button>
                <button class="btn quiet" onclick={() => (editing = null)}>Cancel</button>
              </div>
            {:else}
              <div class="row">
                {#if b.isFolder}
                  <span class="ico"><Icon name="folder" size={16} /></span>
                {:else}
                  <Favicon src={b.favicon} size={16} />
                {/if}
                <a href={b.url ?? '#'} onclick={(e) => open(e, b)} onauxclick={(e) => open(e, b)} title={b.url ?? b.title}>
                  <span class="title">{b.title || b.url}</span>
                  {#if b.url}<span class="host">{hostOf(b.url)}</span>{/if}
                </a>
                <span class="acts">
                  <IconButton icon="edit" label="Edit" onclick={() => (editing = { id: b.id, title: b.title, url: b.url ?? '' })} />
                  <IconButton icon="trash" label={b.isFolder ? 'Delete folder and its contents' : 'Delete bookmark'} onclick={() => run(() => api.bookmarkDelete(b.id))} />
                </span>
              </div>
            {/if}
          </li>
        {:else}
          {#if !editing}
            <li class="empty">
              {results ? `No bookmarks match “${search}”.` : 'This folder is empty. Bookmark a page with Ctrl+D, or drag bookmarks here.'}
            </li>
          {/if}
        {/each}
      </ul>
    </div>
  </div>
</PageShell>

<style>
  .layout {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 28px;
    align-items: start;
  }
  .tree {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .tree button {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 10px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink-2);
    text-align: left;
  }
  .tree span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .tree button:hover {
    background: var(--hover);
  }
  .tree button.active {
    background: var(--accent-soft);
    color: var(--ink);
    font-weight: 600;
  }
  .tree button.drop,
  li.drop {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 0 0 10px;
    color: var(--muted);
    min-height: 24px;
  }
  .crumbs button {
    border: 0;
    background: none;
    padding: 2px 4px;
    border-radius: 4px;
    color: var(--muted);
  }
  .crumbs button:last-child {
    color: var(--ink);
    font-weight: 600;
  }
  .crumbs button:hover {
    background: var(--hover);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    border-radius: var(--radius-s);
  }
  .row {
    min-height: 42px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 6px 0 10px;
    border-radius: var(--radius-s);
  }
  .row:not(.editing):hover {
    background: var(--hover);
  }
  .row.editing {
    padding: 6px 6px 6px 10px;
    background: var(--sunken);
    margin-bottom: 4px;
  }
  .row.editing .field {
    flex: 1;
  }
  .row.editing .url {
    flex: 1.4;
  }
  .ico {
    color: var(--muted);
    display: flex;
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
    font-size: 12px;
    color: var(--muted);
  }
  .acts {
    display: flex;
    opacity: 0;
  }
  .row:hover .acts,
  .acts:focus-within {
    opacity: 1;
  }
  .empty {
    padding: 48px 10px;
    color: var(--muted);
    text-align: center;
  }
  @media (max-width: 760px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .tree {
      position: static;
    }
  }
</style>
