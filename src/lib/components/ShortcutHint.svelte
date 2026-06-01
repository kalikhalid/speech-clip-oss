<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";

  interface Props {
    hotkey: string;
    onDismiss?: () => void;
  }

  let { hotkey, onDismiss }: Props = $props();

  // Format hotkey for display (e.g., "control+`" -> "⌃`")
  function formatHotkey(hk: string): string {
    return hk
      .replace(/control/gi, "⌃")
      .replace(/shift/gi, "⇧")
      .replace(/alt|option/gi, "⌥")
      .replace(/command|meta|super/gi, "⌘")
      .replace(/\+/g, "");
  }

  const displayHotkey = $derived(formatHotkey(hotkey));

  // Автоматически скрываем через 5 секунд
  onMount(() => {
    const timer = setTimeout(() => {
      onDismiss?.();
    }, 5000);

    return () => clearTimeout(timer);
  });
</script>

<div
  class="shortcut-hint"
  transition:fly={{ y: 20, duration: 400 }}
>
  <div class="hint-container">
    <div class="hint-header">
      <span class="hint-title">Быстрый старт</span>
    </div>

    <div class="hint-divider"></div>

    <div class="hint-row">
      <span class="label">Диктовка</span>
      <span class="hotkey">{displayHotkey}</span>
    </div>

    <div class="hint-row subtle">
      <span class="label">Удерживайте для записи</span>
    </div>
  </div>
</div>

<style>
  .shortcut-hint {
    /* Ключевое: fixed позиционирование независимо от мини-бара */
    position: fixed;
    bottom: 80px; /* Позиция над мини-баром (bottom-8 = 32px + высота подсказки + отступ) */
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
    pointer-events: none;
    -webkit-app-region: no-drag;
  }

  .hint-container {
    background: rgba(20, 20, 20, 0.95);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 160px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
  }

  .hint-header {
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .hint-title {
    font-size: 10px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .hint-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
  }

  .hint-row.subtle {
    justify-content: flex-start;
  }

  .hint-row.subtle .label {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.35);
  }

  .label {
    color: rgba(255, 255, 255, 0.6);
    font-weight: 400;
  }

  .hotkey {
    background: rgba(255, 79, 0, 0.12);
    color: #ff4f00;
    border: 1px solid rgba(255, 79, 0, 0.25);
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 4px;
    letter-spacing: 0.3px;
  }

  .hint-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.08);
    margin: 0;
  }
</style>
