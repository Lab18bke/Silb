<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import Favicon from '../components/Favicon.svelte';
  import { displayUrl } from '../lib/format';
  import type { Suggestion } from '../lib/types';

  let { data, onpick, onhover }: { data: { items: Suggestion[]; selected: number }; onpick: (i: number, newTab: boolean) => void; onhover: (i: number) => void } =
    $props();
</script>

<ul class="list" role="listbox" aria-label="Suggestions">
  {#each data.items as s, i}
    <li role="option" aria-selected={i === data.selected}>
      <button
        type="button"
        class:selected={i === data.selected}
        onpointerdown={(e) => {
          e.preventDefault();
          onpick(i, e.button === 1 || e.ctrlKey);
        }}
        onmouseenter={() => onhover(i)}
      >
        <span class="icon">
          {#if s.kind === 'search'}
            <Icon name="search" size={15} />
          {:else if s.kind === 'url'}
            <Icon name="globe" size={15} />
          {:else}
            <Favicon src={s.favicon} size={16} fallback={s.kind === 'bookmark' ? 'bookmark' : 'clock'} />
          {/if}
        </span>
        <span class="title">{s.title || displayUrl(s.url)}</span>
        {#if s.kind === 'history' || s.kind === 'bookmark'}
          <span class="url">{displayUrl(s.url)}</span>
        {/if}
        {#if s.kind === 'bookmark'}<span class="tag"><Icon name="star" size={12} filled /></span>{/if}
      </button>
    </li>
  {/each}
</ul>

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
  }
  button {
    width: 100%;
    height: 38px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink);
    text-align: left;
  }
  button.selected {
    background: var(--accent-soft);
  }
  .icon {
    width: 18px;
    display: flex;
    justify-content: center;
    color: var(--muted);
    flex: none;
  }
  .title {
    flex: 0 1 auto;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .url {
    flex: 1 1 0;
    min-width: 0;
    color: var(--accent);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    color: var(--accent);
    display: flex;
  }
</style>
