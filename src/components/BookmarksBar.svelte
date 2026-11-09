<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import Favicon from './Favicon.svelte';
  import { api, errorText } from '../lib/api';
  import { on } from '../lib/events';
  import { childrenOf, folderOptions } from '../lib/bookmarks';
  import { openUrl, runAction } from '../chrome/actions';
  import { anchorBelow, openPopover, toast } from '../chrome/popover';
  import { browser } from '../chrome/state.svelte';
  import { BOOKMARKS_BAR, type Bookmark } from '../lib/types';

  type Item = { id?: string; label?: string; icon?: string; disabled?: boolean; separator?: boolean; children?: Item[] };

  let all = $state<Bookmark[]>([]);
  const items = $derived(childrenOf(all, BOOKMARKS_BAR));

  async function load() {
    all = await api.bookmarksAll();
  }

  function open(b: Bookmark, e: MouseEvent) {
    if (!b.url) return;
    const background = e.ctrlKey || e.button === 1;
    openUrl(b.url, background ? 'background' : e.shiftKey ? 'window' : 'current');
  }

  function build(parent: number, depth: number): Item[] {
    return childrenOf(all, parent)
      .slice(0, 60)
      .map((b) =>
        b.isFolder
          ? { id: `folder:${b.id}`, label: b.title, icon: 'folder', children: depth < 5 ? build(b.id, depth + 1) : [] }
          : { id: `bm:${b.id}`, label: b.title || b.url || '', icon: 'bookmark' },
      );
  }

  function folderMenu(folder: Bookmark, el: HTMLElement) {
    const list = build(folder.id, 0);
    const shown: Item[] = list.length ? list : [{ id: 'empty', label: 'This folder is empty', disabled: true }];
    openPopover('menu', anchorBelow(el, 260, Math.min(shown.length, 14) * 32 + 12, 'start', 4), { items: shown }, {
      onEvent: (name, data) => {
        if (name !== 'select' || !String(data.itemId).startsWith('bm:')) return;
        const b = all.find((x) => x.id === Number(String(data.itemId).slice(3)));
        if (b?.url) openUrl(b.url, data.newTab ? 'background' : 'current');
      },
    });
  }

  function edit(b: Bookmark) {
    const el = document.querySelector(`[data-bm="${b.id}"]`);
    if (!el) return;
    const data = {
      bookmark: b,
      folders: folderOptions(all, b.isFolder ? b.id : undefined),
      created: false,
      editUrl: true,
      isPrivate: browser.isPrivate,
    };
    openPopover('bookmark', anchorBelow(el, 340, b.url ? 262 : 200), data, {
      onEvent: async (n, d) => {
        try {
          if (n === 'save') {
            await api.bookmarkUpdate(b.id, d.title || b.title, b.url ? d.url || b.url : null);
            if (d.parentId !== b.parentId) await api.bookmarkMove(b.id, d.parentId, 1 << 30);
          } else if (n === 'remove') await api.bookmarkDelete(b.id);
        } catch (err) {
          toast(errorText(err));
        }
      },
    });
  }

  function contextMenu(e: MouseEvent, b: Bookmark) {
    e.preventDefault();
    const items: Item[] = [
      ...(b.url
        ? [
            { id: 'open-tab', label: 'Open in new tab', icon: 'plus' },
            { id: 'open-window', label: browser.isPrivate ? 'Open in new private window' : 'Open in new window', icon: 'window' },
            { separator: true },
          ]
        : []),
      { id: 'edit', label: b.url ? 'Edit bookmark…' : 'Rename folder…', icon: 'edit' },
      { id: 'delete', label: 'Delete', icon: 'trash' },
      { separator: true },
      { id: 'manage', label: 'Manage bookmarks', icon: 'bookmark' },
    ];
    const height = items.reduce((h, i) => h + (i.separator ? 9 : 32), 12);
    openPopover('menu', { x: Math.min(e.clientX, window.innerWidth - 240), y: e.clientY, width: 232, height }, { items }, {
      onEvent: async (name, data) => {
        if (name !== 'select') return;
        try {
          if (data.itemId === 'open-tab' && b.url) await openUrl(b.url, 'background');
          if (data.itemId === 'open-window' && b.url) await openUrl(b.url, 'window');
          if (data.itemId === 'delete') await api.bookmarkDelete(b.id);
          if (data.itemId === 'manage') runAction('bookmarks');
          if (data.itemId === 'edit') setTimeout(() => edit(b), 60);
        } catch (err) {
          toast(errorText(err));
        }
      },
    });
  }

  onMount(() => {
    load();
    return on('bookmarks-changed', load);
  });
</script>

<nav class="bmbar" aria-label="Bookmarks bar">
  {#each items as b (b.id)}
    <button
      type="button"
      class="item"
      data-bm={b.id}
      title={b.url ? `${b.title}\n${b.url}` : b.title}
      onclick={(e) => (b.isFolder ? folderMenu(b, e.currentTarget as HTMLElement) : open(b, e))}
      onauxclick={(e) => e.button === 1 && open(b, e)}
      oncontextmenu={(e) => contextMenu(e, b)}
    >
      {#if b.isFolder}
        <Icon name="folder" size={14} />
      {:else}
        <Favicon src={b.favicon} size={14} />
      {/if}
      <span>{b.title || b.url}</span>
    </button>
  {:else}
    <span class="empty">Press Ctrl+D on any page to add it here.</span>
  {/each}
</nav>

<style>
  .bmbar {
    height: 32px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--line);
    overflow: hidden;
  }
  .item {
    flex: none;
    max-width: 180px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink-2);
    font-size: 12px;
  }
  .item span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .item:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .empty {
    color: var(--faint);
    font-size: 12px;
    padding: 0 4px;
  }
</style>
