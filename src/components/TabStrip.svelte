<script lang="ts">
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Favicon from './Favicon.svelte';
  import WindowControls from './WindowControls.svelte';
  import { api } from '../lib/api';
  import { browser } from '../chrome/state.svelte';
  import { openPopover } from '../chrome/popover';
  import type { IconName } from '../lib/icons';
  import type { TabInfo } from '../lib/types';

  let tabsEl: HTMLDivElement;
  let drag = $state<{ id: number; from: number; to: number; dx: number; width: number; active: boolean } | null>(null);
  let start = { x: 0, centers: [] as number[] };

  const internalIcons: Record<string, IconName> = {
    history: 'clock',
    bookmarks: 'bookmark',
    downloads: 'download',
    settings: 'settings',
    private: 'mask',
  };

  function title(t: TabInfo) {
    if (t.internal) return t.title;
    return t.title || t.url || 'New tab';
  }

  function shift(i: number): number {
    if (!drag?.active) return 0;
    if (i === drag.from) return drag.dx;
    if (drag.from < drag.to && i > drag.from && i <= drag.to) return -drag.width;
    if (drag.to < drag.from && i >= drag.to && i < drag.from) return drag.width;
    return 0;
  }

  function onPointerDown(e: PointerEvent, tab: TabInfo, index: number) {
    if (e.button !== 0 || (e.target as HTMLElement).closest('button')) return;
    if (tab.id !== browser.active) api.tabActivate(tab.id);
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const nodes = Array.from(tabsEl.querySelectorAll<HTMLElement>('.tab'));
    start = { x: e.clientX, centers: nodes.map((n) => n.getBoundingClientRect().left + n.offsetWidth / 2) };
    drag = { id: tab.id, from: index, to: index, dx: 0, width: el.offsetWidth, active: false };
  }

  function onPointerMove(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - start.x;
    if (!drag.active && Math.abs(dx) < 5) return;
    const center = start.centers[drag.from] + dx;
    let to = 0;
    start.centers.forEach((c, i) => {
      if (i !== drag!.from && c < center) to++;
    });
    drag = { ...drag, active: true, dx, to };
  }

  function onPointerUp() {
    if (drag?.active && drag.to !== drag.from) api.tabMove(drag.id, drag.to);
    drag = null;
  }

  function onAux(e: MouseEvent, tab: TabInfo) {
    if (e.button === 1) {
      e.preventDefault();
      api.tabClose(tab.id);
    }
  }

  function onWheel(e: WheelEvent) {
    if (tabsEl.scrollWidth > tabsEl.clientWidth) {
      tabsEl.scrollLeft += e.deltaY || e.deltaX;
      e.preventDefault();
    }
  }

  function contextMenu(e: MouseEvent, tab: TabInfo) {
    e.preventDefault();
    const items = [
      { id: 'new-right', label: 'New tab to the right', icon: 'plus' },
      { separator: true },
      { id: 'reload', label: 'Reload', icon: 'reload', shortcut: 'Ctrl+R', disabled: !!tab.internal },
      { id: 'duplicate', label: 'Duplicate' , icon: 'copy' },
      { id: 'mute', label: tab.muted ? 'Unmute site' : 'Mute site', icon: tab.muted ? 'volume' : 'volume-off', disabled: !!tab.internal },
      { separator: true },
      { id: 'close', label: 'Close', shortcut: 'Ctrl+W' },
      { id: 'close-others', label: 'Close other tabs', disabled: browser.tabs.length < 2 },
      { id: 'reopen', label: 'Reopen closed tab', shortcut: 'Ctrl+Shift+T', disabled: browser.closedCount === 0 },
    ];
    const height = items.reduce((h, i) => h + (i.separator ? 9 : 32), 12);
    openPopover(
      'menu',
      { x: Math.min(e.clientX, window.innerWidth - 248), y: e.clientY, width: 240, height },
      { items },
      {
        onEvent: (name, data) => {
          if (name !== 'select') return;
          const actions: Record<string, () => unknown> = {
            'new-right': () => api.tabNew(undefined, true, tab.id),
            reload: () => api.tabReload(tab.id),
            duplicate: () => api.tabDuplicate(tab.id),
            mute: () => api.tabMute(tab.id),
            close: () => api.tabClose(tab.id),
            'close-others': () => api.tabCloseOthers(tab.id),
            reopen: () => api.tabReopen(),
          };
          actions[data.itemId]?.();
        },
      },
    );
  }

  $effect(() => {
    const id = browser.active;
    const el = tabsEl?.querySelector<HTMLElement>(`[data-tab="${id}"]`);
    el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  });
</script>

<div class="strip" data-tauri-drag-region>
  {#if browser.isPrivate}
    <div class="private" title="Private window: history, cookies and site data are discarded when you close all private windows">
      <Icon name="mask" size={15} />
      <span>Private</span>
    </div>
  {/if}
  <div class="tabs" role="tablist" bind:this={tabsEl} onwheel={onWheel}>
    {#each browser.tabs as tab, i (tab.id)}
      {@const active = tab.id === browser.active}
      <div
        class="tab"
        class:active
        class:dragging={drag?.active && drag.id === tab.id}
        data-tab={tab.id}
        role="tab"
        tabindex="-1"
        aria-selected={active}
        title={title(tab)}
        style:transform={shift(i) ? `translateX(${shift(i)}px)` : null}
        onpointerdown={(e) => onPointerDown(e, tab, i)}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        onauxclick={(e) => onAux(e, tab)}
        onmousedown={(e) => e.button === 1 && e.preventDefault()}
        oncontextmenu={(e) => contextMenu(e, tab)}
      >
        <span class="fav" class:none={tab.internal === 'newtab'}>
          {#if tab.loading}
            <span class="spinner" aria-label="Loading"></span>
          {:else if tab.crashed}
            <Icon name="warning" size={14} />
          {:else if tab.internal === 'newtab'}
            <Icon name="search" size={14} />
          {:else if tab.internal}
            {#if internalIcons[tab.internal.split('/')[0]]}
              <Icon name={internalIcons[tab.internal.split('/')[0]]} size={14} />
            {/if}
          {:else}
            <Favicon src={tab.favicon} size={16} />
          {/if}
        </span>
        <span class="title">{title(tab)}</span>
        {#if tab.audible || tab.muted}
          <button class="mini" type="button" aria-label={tab.muted ? 'Unmute tab' : 'Mute tab'} title={tab.muted ? 'Unmute tab' : 'Mute tab'} onclick={() => api.tabMute(tab.id)}>
            <Icon name={tab.muted ? 'volume-off' : 'volume'} size={13} />
          </button>
        {/if}
        <button class="mini close" type="button" aria-label="Close tab" title="Close tab (Ctrl+W)" onclick={() => api.tabClose(tab.id)}>
          <Icon name="close-small" size={14} />
        </button>
      </div>
    {/each}
  </div>
  <IconButton icon="plus" label="New tab (Ctrl+T)" onclick={() => api.tabNew()} />
  <div class="fill" data-tauri-drag-region></div>
  <WindowControls />
</div>

<style>
  .strip {
    height: var(--tabbar-h);
    display: flex;
    align-items: flex-end;
    gap: 4px;
    padding-left: 8px;
    background: var(--frame);
    user-select: none;
  }
  .strip > :global(.icon-btn) {
    margin-bottom: 4px;
    height: calc(var(--tabbar-h) - 12px);
    min-width: calc(var(--tabbar-h) - 12px);
  }
  .private {
    align-self: center;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px 0 6px;
    color: var(--accent);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .tabs {
    flex: 0 1 auto;
    display: flex;
    align-items: flex-end;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    height: 100%;
    padding-top: 6px;
  }
  .tab {
    position: relative;
    flex: 0 1 220px;
    width: 220px;
    min-width: 76px;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 12px;
    border-radius: var(--radius-m) var(--radius-m) 0 0;
    color: var(--muted);
    transition: background 0.12s, color 0.12s;
    outline: none;
    container-type: inline-size;
  }
  @container (max-width: 96px) {
    .title {
      display: none;
    }
    .tab .fav.none {
      display: flex;
    }
    .fav {
      margin: 0 auto;
    }
    .close {
      display: none;
    }
  }
  .tab:not(.active):not(.dragging) {
    transition: transform 0.18s ease, background 0.12s, color 0.12s;
  }
  .tab::after {
    content: '';
    position: absolute;
    right: 0;
    top: 30%;
    height: 40%;
    width: 1px;
    background: var(--line);
  }
  .tab.active::after,
  .tab:hover::after,
  .tab:has(+ .tab.active)::after,
  .tab:has(+ .tab:hover)::after {
    display: none;
  }
  .tab:hover:not(.active) {
    background: var(--hover);
    color: var(--ink-2);
  }
  .tab.active {
    background: var(--surface);
    color: var(--ink);
    z-index: 1;
  }
  .tab.active::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    box-shadow: 0 0 0 1px var(--line);
    clip-path: inset(-2px -2px 0 -2px);
    pointer-events: none;
  }
  .tab.dragging {
    z-index: 2;
    box-shadow: var(--shadow-pop);
  }
  .fav {
    width: 16px;
    height: 16px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .fav.none {
    display: none;
  }
  .spinner {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: 1.6px solid var(--line-strong);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .title {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    mask-image: linear-gradient(90deg, #000 calc(100% - 18px), transparent);
    font-size: 12.5px;
  }
  .mini {
    flex: none;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 5px;
    padding: 0;
    background: transparent;
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .mini:hover {
    background: var(--press);
    color: var(--ink);
  }
  .tab:not(.active):not(:hover) .close {
    opacity: 0;
  }
  .tab:not(.active) {
    min-width: 44px;
  }
  .tab:not(.active) {
    padding: 0 8px;
  }
  .fill {
    flex: 1;
    align-self: stretch;
    min-width: 32px;
  }
</style>
