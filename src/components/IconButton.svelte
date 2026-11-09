<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from '../lib/icons';

  let {
    icon,
    label,
    onclick,
    disabled = false,
    active = false,
    filled = false,
    size = 16,
    el = $bindable(),
    children,
    ...rest
  }: {
    icon: IconName;
    label: string;
    onclick?: (e: MouseEvent) => void;
    disabled?: boolean;
    active?: boolean;
    filled?: boolean;
    size?: number;
    el?: HTMLButtonElement;
    children?: Snippet;
    [key: string]: unknown;
  } = $props();
</script>

<button
  bind:this={el}
  class="icon-btn"
  class:active
  type="button"
  aria-label={label}
  title={label}
  {disabled}
  {onclick}
  {...rest}
>
  <Icon name={icon} {size} {filled} />
  {@render children?.()}
</button>

<style>
  .icon-btn {
    position: relative;
    flex: none;
    height: var(--control-h);
    min-width: var(--control-h);
    padding: 0 7px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink-2);
    transition: background 0.12s, color 0.12s;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--hover);
    color: var(--ink);
  }
  .icon-btn:active:not(:disabled) {
    background: var(--press);
  }
  .icon-btn:disabled {
    color: var(--faint);
    opacity: 0.55;
  }
  .icon-btn.active {
    color: var(--accent);
  }
</style>
