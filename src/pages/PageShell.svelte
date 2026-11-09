<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../components/Icon.svelte';

  let {
    title,
    search = $bindable(''),
    searchLabel,
    actions,
    children,
    wide = false,
    onSearch,
  }: {
    title: string;
    search?: string;
    searchLabel?: string;
    actions?: Snippet;
    children: Snippet;
    wide?: boolean;
    onSearch?: () => void;
  } = $props();
</script>

<div class="page scroll">
  <div class="inner" class:wide>
    <div class="head">
      <h1>{title}</h1>
      <div class="tools">
        {#if searchLabel}
          <label class="search">
            <Icon name="search" size={14} />
            <input bind:value={search} oninput={onSearch} placeholder={searchLabel} aria-label={searchLabel} spellcheck="false" />
          </label>
        {/if}
        {@render actions?.()}
      </div>
    </div>
    {@render children()}
  </div>
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
    background: var(--surface);
  }
  .inner {
    width: min(820px, calc(100% - 56px));
    margin: 0 auto;
    padding: 40px 0 64px;
  }
  .inner.wide {
    width: min(1040px, calc(100% - 56px));
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
    margin-bottom: 26px;
  }
  h1 {
    flex: 1;
    font-family: var(--font-display);
    font-weight: 420;
    font-size: 34px;
    letter-spacing: -0.018em;
    margin: 0;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .search {
    height: var(--control-h);
    width: 260px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--line-strong);
    background: var(--raised);
    color: var(--muted);
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--ink);
  }
</style>
