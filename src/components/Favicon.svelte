<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IconName } from '../lib/icons';

  let { src, size = 16, fallback = 'globe' }: { src: string | null | undefined; size?: number; fallback?: IconName } = $props();
  let failed = $state(false);
  $effect(() => {
    src;
    failed = false;
  });
</script>

{#if src && !failed}
  <img {src} width={size} height={size} alt="" onerror={() => (failed = true)} />
{:else}
  <span class="fallback" style:width="{size}px" style:height="{size}px"><Icon name={fallback} size={Math.round(size * 0.9)} /></span>
{/if}

<style>
  img {
    flex: none;
    border-radius: 3px;
    object-fit: contain;
  }
  .fallback {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
  }
</style>
