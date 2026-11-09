<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, children }: { title?: string; children: Snippet } = $props();
</script>

<section class="group">
  {#if title}<h3>{title}</h3>{/if}
  <div class="box">{@render children()}</div>
</section>

<style>
  :global(.group + .group) {
    margin-top: 28px;
  }
  h3 {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    margin: 0 0 6px;
  }
  .box {
    padding: 2px 18px;
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    background: var(--raised);
  }
</style>
