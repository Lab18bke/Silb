<script lang="ts">
  const found = import.meta.glob('../assets/brand/logo*.{svg,png,webp}', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
  const dark = Object.entries(found).find(([k]) => /logo[-_]dark\./.test(k))?.[1];
  const light = Object.entries(found).find(([k]) => /logo\./.test(k))?.[1];

  let { height = 28, isDark = false }: { height?: number; isDark?: boolean } = $props();
  const src = $derived((isDark && dark) || light || dark);
</script>

{#if src}
  <img {src} alt="Silb" style:height="{height}px" />
{/if}

<style>
  img {
    display: block;
    width: auto;
  }
</style>
