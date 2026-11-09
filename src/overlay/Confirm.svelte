<script lang="ts">
  let {
    data,
    onconfirm,
    oncancel,
  }: { data: { title: string; message: string; confirm: string; cancel: string }; onconfirm: () => void; oncancel: () => void } = $props();
</script>

<div class="backdrop" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="dialog" role="alertdialog" aria-labelledby="confirm-title" aria-describedby="confirm-msg">
    <h2 id="confirm-title">{data.title}</h2>
    <p id="confirm-msg">{data.message}</p>
    <div class="actions">
      <button class="btn quiet" onclick={oncancel}>{data.cancel}</button>
      <button class="btn primary" data-autofocus onclick={onconfirm}>{data.confirm}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(10, 10, 10, 0.34);
    display: flex;
    align-items: center;
    justify-content: center;
    animation: fade 0.12s ease-out;
  }
  .dialog {
    width: min(420px, calc(100% - 48px));
    padding: 24px 24px 18px;
    border-radius: var(--radius-l);
    background: var(--raised);
    border: 1px solid var(--line);
    box-shadow: var(--shadow-pop);
  }
  h2 {
    font-family: var(--font-display);
    font-weight: 450;
    font-size: 22px;
    margin: 0 0 8px;
  }
  p {
    margin: 0;
    color: var(--ink-2);
    line-height: 1.55;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>
