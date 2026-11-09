<script lang="ts" generics="T extends string">
  import Icon from './Icon.svelte';
  import type { IconName } from '../lib/icons';

  let { value, options, onchange, label }: {
    value: T;
    options: { value: T; label: string; icon?: IconName }[];
    onchange: (v: T) => void;
    label: string;
  } = $props();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o}
    <button type="button" role="radio" aria-checked={o.value === value} onclick={() => onchange(o.value)}>
      {#if o.icon}<Icon name={o.icon} size={14} />{/if}
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border-radius: var(--radius-m);
    background: var(--sunken);
    border: 1px solid var(--line);
  }
  button {
    height: calc(var(--control-h) - 6px);
    padding: 0 12px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  button:hover {
    color: var(--ink);
  }
  button[aria-checked='true'] {
    background: var(--raised);
    color: var(--ink);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }
</style>
