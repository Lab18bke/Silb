<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import type { IconName } from '../lib/icons';

  interface Item {
    id?: string;
    label?: string;
    icon?: IconName;
    shortcut?: string;
    checked?: boolean;
    disabled?: boolean;
    separator?: boolean;
    kind?: 'zoom';
    zoom?: number;
    children?: Item[];
  }

  let { items, onselect, onzoom }: { items: Item[]; onselect: (id: string, newTab: boolean) => void; onzoom: (a: string) => void } = $props();

  let stack = $state<{ title: string; items: Item[] }[]>([]);
  let focusIndex = $state(-1);
  let list = $state<HTMLElement>();
  let zoom = $state(untrack(() => items.find((i) => i.kind === 'zoom')?.zoom ?? 1));

  const view = $derived(stack.length ? stack[stack.length - 1].items : items);
  const selectable = $derived(view.map((it, i) => (!it.separator && !it.disabled && it.kind !== 'zoom' ? i : -1)).filter((i) => i >= 0));

  function activate(it: Item, e?: MouseEvent | KeyboardEvent) {
    if (it.disabled || it.separator) return;
    if (it.children) {
      stack = [...stack, { title: it.label ?? '', items: it.children.length ? it.children : [{ label: 'Empty', disabled: true }] }];
      focusIndex = -1;
      return;
    }
    if (it.id) onselect(it.id, !!(e && (e.ctrlKey || ('button' in e && e.button === 1))));
  }

  function step(delta: number) {
    if (!selectable.length) return;
    const pos = selectable.indexOf(focusIndex);
    const next = selectable[(pos + delta + selectable.length) % selectable.length];
    focusIndex = pos < 0 && delta < 0 ? selectable[selectable.length - 1] : next;
    list?.querySelector<HTMLElement>(`[data-index="${focusIndex}"]`)?.focus();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      step(1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      step(-1);
    } else if ((e.key === 'ArrowLeft' || e.key === 'Backspace') && stack.length) {
      e.preventDefault();
      stack = stack.slice(0, -1);
    } else if (e.key === 'ArrowRight' && focusIndex >= 0 && view[focusIndex]?.children) {
      e.preventDefault();
      activate(view[focusIndex]);
    }
  }

  function zoomBy(action: 'in' | 'out' | 'reset' | 'fullscreen') {
    onzoom(action);
    const levels = [0.25, 0.33, 0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4, 5];
    if (action === 'in') zoom = levels.find((z) => z > zoom + 0.001) ?? 5;
    if (action === 'out') zoom = [...levels].reverse().find((z) => z < zoom - 0.001) ?? 0.25;
    if (action === 'reset') zoom = 1;
  }
</script>

<div class="menu" role="menu" tabindex="-1" bind:this={list} onkeydown={onKey} data-autofocus>
  {#if stack.length}
    <button type="button" class="item back" onclick={() => (stack = stack.slice(0, -1))}>
      <Icon name="chevron-left" size={14} />
      <span class="label">{stack[stack.length - 1].title}</span>
    </button>
    <div class="sep"></div>
  {/if}
  <div class="items">
    {#each view as it, i}
      {#if it.separator}
        <div class="sep" role="separator"></div>
      {:else if it.kind === 'zoom'}
        <div class="zoom" class:disabled={it.disabled}>
          <span class="label">Zoom</span>
          <button type="button" aria-label="Zoom out" disabled={it.disabled} onclick={() => zoomBy('out')}><Icon name="minus" size={14} /></button>
          <button type="button" class="pct" title="Reset zoom" disabled={it.disabled} onclick={() => zoomBy('reset')}>{Math.round(zoom * 100)}%</button>
          <button type="button" aria-label="Zoom in" disabled={it.disabled} onclick={() => zoomBy('in')}><Icon name="plus" size={14} /></button>
          <span class="divider"></span>
          <button type="button" aria-label="Full screen (F11)" title="Full screen (F11)" onclick={() => zoomBy('fullscreen')}><Icon name="expand" size={14} /></button>
        </div>
      {:else}
        <button
          type="button"
          class="item"
          role="menuitem"
          data-index={i}
          disabled={it.disabled}
          onclick={(e) => activate(it, e)}
          onauxclick={(e) => e.button === 1 && activate(it, e)}
          onmouseenter={() => (focusIndex = i)}
        >
          <span class="icon">
            {#if it.checked !== undefined}
              {#if it.checked}<Icon name="check" size={14} />{/if}
            {:else if it.icon}
              <Icon name={it.icon} size={14} />
            {/if}
          </span>
          <span class="label">{it.label}</span>
          {#if it.shortcut}<span class="kbd">{it.shortcut}</span>{/if}
          {#if it.children}<Icon name="chevron-right" size={13} />{/if}
        </button>
      {/if}
    {/each}
  </div>
</div>

<style>
  .menu {
    padding: 6px;
    outline: none;
    max-height: 640px;
    display: flex;
    flex-direction: column;
  }
  .items {
    overflow-y: auto;
  }
  .item {
    width: 100%;
    height: 30px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px 0 8px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink);
    text-align: left;
    outline: none;
  }
  .item:hover:not(:disabled),
  .item:focus-visible {
    background: var(--hover);
  }
  .item:disabled {
    color: var(--faint);
  }
  .icon {
    width: 16px;
    display: flex;
    justify-content: center;
    color: var(--muted);
  }
  .label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kbd {
    color: var(--faint);
    font-size: 11.5px;
  }
  .back {
    color: var(--muted);
    font-weight: 600;
  }
  .sep {
    height: 1px;
    background: var(--line);
    margin: 4px 6px;
  }
  .zoom {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 6px 0 34px;
  }
  .zoom.disabled .label {
    color: var(--faint);
  }
  .zoom button {
    height: 28px;
    min-width: 28px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .zoom button:hover:not(:disabled) {
    background: var(--hover);
  }
  .zoom button:disabled {
    color: var(--faint);
  }
  .pct {
    min-width: 50px !important;
    font-variant-numeric: tabular-nums;
    font-size: 12px;
  }
  .divider {
    width: 1px;
    height: 18px;
    background: var(--line);
    margin: 0 4px;
  }
</style>
