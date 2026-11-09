<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, description, children, stacked = false }: { label: string; description?: string; children?: Snippet; stacked?: boolean } = $props();
</script>

<div class="row" class:stacked>
  <div class="text">
    <div class="label">{label}</div>
    {#if description}<div class="desc">{description}</div>{/if}
  </div>
  {#if children}<div class="control">{@render children()}</div>{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 14px 0;
    border-bottom: 1px solid var(--line);
  }
  .row:last-child {
    border-bottom: 0;
  }
  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 10px;
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .label {
    color: var(--ink);
  }
  .desc {
    color: var(--muted);
    font-size: 12.5px;
    margin-top: 2px;
    line-height: 1.5;
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stacked .control {
    flex-wrap: wrap;
  }
</style>
