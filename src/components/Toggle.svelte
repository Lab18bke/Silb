<script lang="ts">
  let { checked, onchange, label, disabled = false }: { checked: boolean; onchange: (v: boolean) => void; label: string; disabled?: boolean } =
    $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  aria-checked={checked}
  aria-label={label}
  {disabled}
  onclick={() => onchange(!checked)}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    flex: none;
    width: 34px;
    height: 20px;
    border-radius: 10px;
    border: 1px solid var(--line-strong);
    background: var(--sunken);
    position: relative;
    padding: 0;
    transition: background 0.15s, border-color 0.15s;
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--muted);
    transition: transform 0.15s, background 0.15s;
  }
  .toggle[aria-checked='true'] {
    background: var(--accent);
    border-color: var(--accent);
  }
  .toggle[aria-checked='true'] .knob {
    transform: translateX(14px);
    background: var(--accent-ink);
  }
  .toggle:disabled {
    opacity: 0.5;
  }
</style>
