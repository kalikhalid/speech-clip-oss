<script lang="ts">
    import { fade, fly } from "svelte/transition";

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
</script>

<div class="onboarding-tip" transition:fly={{ y: -20, duration: 300 }}>
    <div class="tip-container">
        <div class="tip-header">
            <span class="icon">💡</span>
            <span class="title">Совет</span>
        </div>
        <div class="tip-body">
            Удерживайте <span class="hotkey-badge">{displayHotkey}</span> для записи
        </div>
    </div>

    <!-- Arrow pointing down -->
    <div class="arrow-down">
        <svg width="14" height="10" viewBox="0 0 14 10" fill="none">
            <path d="M7 10L0 0L14 0L7 10Z" fill="#1a1a1a" />
        </svg>
    </div>
</div>

<style>
    .onboarding-tip {
        position: absolute;
        bottom: 100%;
        left: 50%;
        transform: translateX(-50%);
        margin-bottom: 24px; /* More space for neon glow */
        z-index: 100;
        pointer-events: auto;
        -webkit-app-region: no-drag;
        display: flex;
        flex-direction: column;
        align-items: center;
        animation: float 3s ease-in-out infinite;
    }

    .tip-container {
        background: #1a1a1a;
        border: 1px solid rgba(255, 255, 255, 0.15);
        border-radius: 12px;
        padding: 12px 16px;
        display: flex;
        flex-direction: column;
        gap: 6px;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
        min-width: 200px;
        text-align: center;
    }

    .tip-header {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        font-size: 13px;
        font-weight: 600;
        color: rgba(255, 255, 255, 0.9);
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }

    .icon {
        font-size: 14px;
    }

    .title {
        color: #ff9f43; /* Warm orange */
    }

    .tip-body {
        font-size: 13px;
        color: rgba(255, 255, 255, 0.7);
        font-family: -apple-system, BlinkMacSystemFont, sans-serif;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
    }

    .hotkey-badge {
        background: rgba(255, 79, 0, 0.15);
        color: #ff4f00;
        border: 1px solid rgba(255, 79, 0, 0.3);
        font-family: monospace;
        font-size: 12px;
        font-weight: 700;
        padding: 2px 6px;
        border-radius: 4px;
    }

    .arrow-down {
        margin-top: -1px; /* Overlap slightly */
        color: #1a1a1a; /* Match background */
        filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
    }

    @keyframes float {
        0%,
        100% {
            transform: translateX(-50%) translateY(0);
        }
        50% {
            transform: translateX(-50%) translateY(-5px);
        }
    }
</style>
