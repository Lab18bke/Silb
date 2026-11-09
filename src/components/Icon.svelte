<script lang="ts">
  import { icons, type IconName } from '../lib/icons';

  let { name, size = 16, filled = false, stroke = 1.55 }: { name: IconName; size?: number; filled?: boolean; stroke?: number } = $props();

  const uid = $props.id();
  const body = `url(#${uid}-body)`;
  const sheen = `url(#${uid}-sheen)`;
  const parts = $derived(icons[name].map((d) => ({ d, closed: /z\s*$/i.test(d) })));
</script>

<svg class="icon" width={size} height={size} viewBox="0 0 16 16" aria-hidden="true">
  <defs>
    <linearGradient id="{uid}-body" gradientUnits="userSpaceOnUse" x1="0" y1="1.5" x2="0" y2="14.5">
      <stop offset="0" class="top" />
      <stop offset="0.5" class="mid" />
      <stop offset="1" class="bottom" />
    </linearGradient>
    <linearGradient id="{uid}-fade" gradientUnits="userSpaceOnUse" x1="0" y1="1" x2="0" y2="12">
      <stop offset="0" stop-color="#fff" />
      <stop offset="1" stop-color="#fff" stop-opacity="0" />
    </linearGradient>
    <mask id="{uid}-sheen" maskUnits="userSpaceOnUse" x="-1" y="-1" width="18" height="18">
      <rect x="-1" y="-1" width="18" height="18" fill="url(#{uid}-fade)" />
    </mask>
  </defs>
  <g fill={filled ? body : 'none'} stroke={body} stroke-width={stroke} stroke-linecap="round" stroke-linejoin="round">
    {#each parts as { d, closed }}
      <path {d} class:glass={!filled && closed} fill={!filled && closed ? body : undefined} />
    {/each}
  </g>
  <g
    class="gloss"
    mask={sheen}
    fill="none"
    stroke-width={Math.max(0.35, stroke * 0.32)}
    stroke-linecap="round"
    stroke-linejoin="round"
    transform="translate(0 -0.42)"
  >
    {#each parts as { d }}
      <path {d} />
    {/each}
  </g>
</svg>

<style>
  .icon {
    flex: none;
    display: block;
    overflow: visible;
    filter: drop-shadow(0 0.6px 0.5px var(--icon-shadow, rgba(0, 0, 0, 0.28)));
  }
  .top {
    stop-color: color-mix(in oklab, currentColor 58%, white);
  }
  .mid {
    stop-color: currentColor;
  }
  .bottom {
    stop-color: color-mix(in oklab, currentColor 72%, black);
  }
  .glass {
    fill-opacity: var(--icon-glass, 0.14);
  }
  .gloss {
    stroke: var(--icon-gloss, rgba(255, 255, 255, 0.7));
  }
</style>
