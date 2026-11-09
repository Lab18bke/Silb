<script lang="ts">
  import { onMount } from 'svelte';
  import IconButton from './IconButton.svelte';
  import AddressBar from './AddressBar.svelte';
  import { api, errorText } from '../lib/api';
  import { on } from '../lib/events';
  import { browser } from '../chrome/state.svelte';
  import { bus, openUrl, runAction } from '../chrome/actions';
  import { anchorBelow, justClosed, openPopover, popoverKind, toast } from '../chrome/popover';
  import { settings } from '../lib/settings.svelte';
  import { folderOptions } from '../lib/bookmarks';
  import { BOOKMARKS_BAR, OTHER_BOOKMARKS, type Bookmark, type DownloadRecord } from '../lib/types';

  let starEl = $state<HTMLButtonElement>();
  let downloadsEl = $state<HTMLButtonElement>();
  let menuEl = $state<HTMLButtonElement>();
  let popupsEl = $state<HTMLButtonElement>();
  let bookmarked = $state(false);
  let active = $state(new Map<number, DownloadRecord>());
  let unseen = $state(false);

  const tab = $derived(browser.activeTab);
  const isWeb = $derived(!!tab && !tab.internal);

  async function refreshBookmarked() {
    const url = tab && !tab.internal ? tab.url : null;
    bookmarked = url ? (await api.bookmarksForUrl(url)).length > 0 : false;
  }

  $effect(() => {
    tab?.url;
    tab?.internal;
    refreshBookmarked();
  });

  const progress = $derived.by(() => {
    let total = 0;
    let received = 0;
    for (const d of active.values()) {
      if (!d.totalBytes) return null;
      total += d.totalBytes;
      received += d.receivedBytes;
    }
    return total ? received / total : null;
  });

  async function bookmarkPage() {
    const t = tab;
    if (!t || t.internal || !starEl) return;
    try {
      const existing = await api.bookmarksForUrl(t.url);
      const bm = existing[0] ?? (await api.bookmarkCreate(BOOKMARKS_BAR, t.title || t.url, t.url));
      bookmarked = true;
      const folders = folderOptions(await api.bookmarksAll());
      openPopover('bookmark', anchorBelow(starEl, 340, 262, 'end'), { bookmark: bm, folders, created: !existing.length, isPrivate: browser.isPrivate }, {
        onEvent: async (name, data) => {
          try {
            if (name === 'save') {
              await api.bookmarkUpdate(bm.id, data.title || t.title || t.url, data.url ?? null);
              if (data.parentId !== bm.parentId) await api.bookmarkMove(bm.id, data.parentId, 1 << 30);
            } else if (name === 'remove') {
              await api.bookmarkDelete(bm.id);
              bookmarked = false;
            }
          } catch (e) {
            toast(errorText(e));
          }
        },
      });
    } catch (e) {
      toast(errorText(e));
    }
  }

  function openDownloads(auto = false) {
    if (!downloadsEl || (!auto && justClosed('downloads'))) return;
    unseen = false;
    openPopover('downloads', anchorBelow(downloadsEl, 360, 320, 'end'), { auto }, {
      focus: !auto,
      onEvent: (name) => {
        if (name === 'show-all') runAction('downloads');
      },
    });
  }

  type MenuItem = {
    id?: string;
    label?: string;
    icon?: string;
    shortcut?: string;
    checked?: boolean;
    disabled?: boolean;
    separator?: boolean;
    kind?: 'zoom';
    zoom?: number;
    children?: MenuItem[];
  };

  function bookmarkItems(all: Bookmark[], parent: number, depth = 0): MenuItem[] {
    const kids = all.filter((b) => b.parentId === parent).sort((a, b) => a.position - b.position).slice(0, 60);
    return kids.map((b) =>
      b.isFolder
        ? { id: `folder:${b.id}`, label: b.title, icon: 'folder', children: depth < 5 ? bookmarkItems(all, b.id, depth + 1) : [] }
        : { id: `bm:${b.id}`, label: b.title || b.url || '', icon: 'bookmark' },
    );
  }

  async function openMenu() {
    if (!menuEl || justClosed('menu')) return;
    const all = await api.bookmarksAll();
    const t = tab;
    const bmChildren: MenuItem[] = [
      { id: 'bookmarks', label: 'Manage bookmarks', icon: 'bookmark', shortcut: 'Ctrl+Shift+O' },
      { id: 'toggle-bookmarks-bar', label: 'Show bookmarks bar', checked: !!settings.value?.showBookmarksBar, shortcut: 'Ctrl+Shift+B' },
      { separator: true },
      ...bookmarkItems(all, BOOKMARKS_BAR),
      ...(all.some((b) => b.parentId === OTHER_BOOKMARKS)
        ? [{ id: 'folder:2', label: 'Other bookmarks', icon: 'folder', children: bookmarkItems(all, OTHER_BOOKMARKS) }]
        : []),
    ];
    const items: MenuItem[] = [
      { id: 'new-tab', label: 'New tab', icon: 'plus', shortcut: 'Ctrl+T' },
      { id: 'new-window', label: 'New window', icon: 'window', shortcut: 'Ctrl+N' },
      { id: 'new-private-window', label: 'New private window', icon: 'mask', shortcut: 'Ctrl+Shift+N' },
      { separator: true },
      { kind: 'zoom', zoom: isWeb ? (t?.zoom ?? 1) : 1, disabled: !isWeb },
      { separator: true },
      { id: 'bookmarks-menu', label: 'Bookmarks', icon: 'bookmark', children: bmChildren },
      { id: 'history', label: 'History', icon: 'clock', shortcut: 'Ctrl+H' },
      { id: 'downloads', label: 'Downloads', icon: 'download', shortcut: 'Ctrl+J' },
      { id: 'reopen-tab', label: 'Reopen closed tab', icon: 'retry', shortcut: 'Ctrl+Shift+T', disabled: browser.closedCount === 0 },
      { separator: true },
      { id: 'find', label: 'Find on page', icon: 'search', shortcut: 'Ctrl+F', disabled: !isWeb },
      { id: 'reader', label: t?.readerActive ? 'Exit Reading mode' : 'Reading mode', icon: 'reader', disabled: !isWeb || !(t?.readerAvailable || t?.readerActive) },
      { id: 'print', label: 'Print…', icon: 'print', shortcut: 'Ctrl+P', disabled: !isWeb },
      { id: 'devtools', label: 'Developer tools', icon: 'code', shortcut: 'F12', disabled: !isWeb },
      { separator: true },
      { id: 'clear-data', label: 'Clear browsing data', icon: 'eraser', shortcut: 'Ctrl+Shift+Del' },
      { id: 'settings', label: 'Settings', icon: 'settings', shortcut: 'Ctrl+,' },
    ];
    const height = items.reduce((h, i) => h + (i.separator ? 9 : i.kind === 'zoom' ? 40 : 30), 12);
    openPopover('menu', anchorBelow(menuEl, 272, height, 'end'), { items }, {
      onEvent: (name, data) => {
        if (name === 'zoom') return runAction(data.action === 'fullscreen' ? 'fullscreen' : `zoom-${data.action}`);
        if (name !== 'select') return;
        const id: string = data.itemId;
        if (id.startsWith('bm:')) {
          const b = all.find((x) => x.id === Number(id.slice(3)));
          if (b?.url) openUrl(b.url, data.newTab ? 'background' : 'current');
        } else runAction(id);
      },
    });
  }

  function showPopups() {
    const t = tab;
    if (!t || !popupsEl) return;
    openPopover('popups', anchorBelow(popupsEl, 340, 120 + Math.min(t.blockedPopups.length, 5) * 34, 'end'), { tabId: t.id, urls: t.blockedPopups, host: t.url });
  }

  onMount(() => {
    const offs = [
      on<DownloadRecord>('download-updated', (d) => {
        const next = new Map(active);
        if (d.state === 'in_progress') next.set(d.id, d);
        else {
          if (next.has(d.id) && d.state === 'completed') unseen = true;
          next.delete(d.id);
        }
        active = next;
      }),
      on<DownloadRecord>('download-started', () => {
        if (settings.value?.showDownloadsPopup && !popoverKind()) openDownloads(true);
        else unseen = true;
      }),
      on('bookmarks-changed', refreshBookmarked),
    ];
    const handler = () => bookmarkPage();
    bus.addEventListener('bookmark-page', handler);
    api.downloadsList().then((list) => {
      active = new Map(list.filter((d) => d.state === 'in_progress').map((d) => [d.id, d]));
    });
    return () => {
      offs.forEach((o) => o());
      bus.removeEventListener('bookmark-page', handler);
    };
  });
</script>

<div class="toolbar">
  <IconButton icon="back" label="Back (Alt+Left)" disabled={!tab?.canGoBack} onclick={() => tab && api.tabBack(tab.id)} />
  <IconButton icon="forward" label="Forward (Alt+Right)" disabled={!tab?.canGoForward} onclick={() => tab && api.tabForward(tab.id)} />
  {#if tab?.loading}
    <IconButton icon="close" label="Stop loading (Esc)" onclick={() => tab && api.tabStop(tab.id)} />
  {:else}
    <IconButton icon="reload" label="Reload (Ctrl+R)" onclick={(e) => tab && api.tabReload(tab.id, e.shiftKey || e.ctrlKey)} />
  {/if}

  <AddressBar>
    {#snippet trailing()}
      {#if tab && tab.blockedPopups.length}
        <IconButton bind:el={popupsEl} icon="popup-blocked" label="Pop-ups blocked on this page" onclick={showPopups} />
      {/if}
      {#if isWeb && tab && Math.abs(tab.zoom - 1) > 0.001}
        <button class="zoom" type="button" title="Reset zoom (Ctrl+0)" onclick={() => tab && api.tabZoom(tab.id, 'reset')}>
          {Math.round(tab.zoom * 100)}%
        </button>
      {/if}
      {#if isWeb && tab && (tab.readerAvailable || tab.readerActive)}
        <IconButton
          icon="reader"
          label={tab.readerActive ? 'Exit Reading mode' : 'Reading mode'}
          active={tab.readerActive}
          onclick={() => tab && api.tabReader(tab.id)}
        />
      {/if}
      {#if isWeb}
        <IconButton
          bind:el={starEl}
          icon="star"
          filled={bookmarked}
          active={bookmarked}
          label={bookmarked ? 'Edit bookmark (Ctrl+D)' : 'Bookmark this page (Ctrl+D)'}
          onclick={bookmarkPage}
        />
      {/if}
    {/snippet}
  </AddressBar>

  <IconButton bind:el={downloadsEl} icon="download" label="Downloads" onclick={() => openDownloads()}>
    {#if progress !== null || active.size > 0}
      <svg class="ring" viewBox="0 0 32 32" aria-hidden="true">
        <circle cx="16" cy="16" r="14" pathLength="100" class="track" />
        <circle cx="16" cy="16" r="14" pathLength="100" class="value" class:indeterminate={progress === null}
          style:stroke-dasharray={progress === null ? '22 78' : `${Math.max(2, progress * 100)} 100`} />
      </svg>
    {:else if unseen}
      <span class="dot"></span>
    {/if}
  </IconButton>
  <IconButton bind:el={menuEl} icon="more" label="Menu" onclick={openMenu} />
</div>

<style>
  .toolbar {
    height: var(--toolbar-h);
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--line);
  }
  .toolbar > :global(.bar) {
    margin: 0 6px;
  }
  .zoom {
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    border: 1px solid var(--line-strong);
    background: var(--raised);
    color: var(--ink-2);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .zoom:hover {
    border-color: var(--accent);
  }
  .ring {
    position: absolute;
    inset: 2px;
    width: calc(100% - 4px);
    height: calc(100% - 4px);
    transform: rotate(-90deg);
    pointer-events: none;
  }
  .ring circle {
    fill: none;
    stroke-width: 2;
  }
  .ring .track {
    stroke: var(--line);
  }
  .ring .value {
    stroke: var(--accent);
    stroke-linecap: round;
    transition: stroke-dasharray 0.3s;
  }
  .ring .indeterminate {
    animation: spin 1.1s linear infinite;
    transform-origin: 16px 16px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .dot {
    position: absolute;
    top: 6px;
    right: 6px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
  }
</style>
