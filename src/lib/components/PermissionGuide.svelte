<script lang="ts">
    import { fade, fly } from "svelte/transition";
    import { locale, format, messagesFor } from "$lib/i18n";

    interface Props {
        onRequestPermission: () => void;
        onClose: () => void;
    }

    let { onRequestPermission, onClose }: Props = $props();

    let step: 1 | 2 = $state(1);

    const msg = $derived(messagesFor($locale));

    function handlePrimaryAction() {
        if (step === 1) {
            onRequestPermission();
            step = 2;
            onRequestPermission();
        }
    }
</script>

<div class="permission-guide" transition:fade>
    <div class="ambient-bg">
        <div class="gradient-orb orb-1"></div>
        <div class="gradient-orb orb-2"></div>
    </div>

    <button class="close-btn" on:click={onClose} aria-label={msg.permission.closeAria}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
            <path
                d="M11 1L1 11M1 1L11 11"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
            />
        </svg>
    </button>

    <div class="content">
        <div class="step-badge">{format(msg.permission.stepOf, { step })}</div>

        <div class="icon-badge">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <circle cx="12" cy="12" r="10" />
                <path d="M12 8v4M12 16h.01" stroke-linecap="round" />
            </svg>
        </div>

        {#if step === 1}
            <div class="step-container" in:fly={{ y: 16, duration: 280 }} out:fade>
                <h2>{msg.permission.step1Title}</h2>
                <p class="description">
                    {msg.permission.step1Desc}
                </p>

                <div class="schematic-wrap">
                    <div class="schematic system-dialog">
                        <div class="dialog-header">
                            <div class="lock-icon">
                                <svg viewBox="0 0 24 24" fill="none">
                                    <rect x="5" y="11" width="14" height="10" rx="2" fill="url(#lockGrad)" />
                                    <path d="M8 11V8a4 4 0 0 1 8 0v3" stroke="rgba(255,255,255,0.5)" stroke-width="2.5" fill="none" stroke-linecap="round" />
                                    <defs>
                                        <linearGradient id="lockGrad" x1="5" y1="11" x2="19" y2="21">
                                            <stop stop-color="#ffd60a" />
                                            <stop offset="1" stop-color="#ff9f0a" />
                                        </linearGradient>
                                    </defs>
                                </svg>
                            </div>
                            <div class="dialog-title">{msg.permission.dialogTitle}</div>
                        </div>
                        <div class="dialog-body">
                            <p class="dialog-subtitle">{msg.permission.dialogSubtitle}</p>
                        </div>
                        <div class="dialog-actions">
                            <div class="btn secondary">{msg.permission.dontAllow}</div>
                            <div class="btn primary active-target">
                                {msg.permission.openSettings}
                                <div class="cursor-hand step1">
                                    <svg viewBox="0 0 24 24" fill="white" filter="drop-shadow(0 2px 4px rgba(0,0,0,0.5))">
                                        <path d="M7 2l12 11.2-5.8.5 3.4 8.3-2.6 1-3.4-8.3-4.6 4.3z" />
                                    </svg>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <p class="instruction-text">
                    {msg.permission.step1Instruction}
                </p>
            </div>
        {:else}
            <div class="step-container" in:fly={{ y: 16, duration: 280 }} out:fade>
                <h2>{msg.permission.step2Title}</h2>
                <p class="description">
                    {msg.permission.step2Desc}
                </p>

                <div class="schematic-wrap">
                    <div class="schematic mac-settings">
                        <div class="window-header">
                            <div class="window-controls">
                                <div class="dot red"></div>
                                <div class="dot yellow"></div>
                                <div class="dot green"></div>
                            </div>
                            <div class="window-title">{msg.permission.privacyTitle}</div>
                        </div>

                        <div class="settings-list">
                            <div class="list-item other">
                                <div class="app-icon"></div>
                                <span class="app-name">{msg.permission.otherApp}</span>
                                <div class="toggle"></div>
                            </div>

                            <div class="list-item speech-clip">
                                <div class="app-icon speech-clip-icon">
                                    <img src="/logo.png" alt="" width="16" height="16" />
                                </div>
                                <span class="app-name">{msg.permission.appName}</span>
                                <div class="toggle-container">
                                    <div class="toggle active-anim"></div>
                                    <div class="cursor-hand step2">
                                        <svg viewBox="0 0 24 24" fill="white" filter="drop-shadow(0 2px 4px rgba(0,0,0,0.5))">
                                            <path d="M7 2l12 11.2-5.8.5 3.4 8.3-2.6 1-3.4-8.3-4.6 4.3z" />
                                        </svg>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        {/if}

        <div class="progress-dots">
            <div class="dot" class:active={step === 1}></div>
            <div class="dot" class:active={step === 2}></div>
        </div>

        <button class="primary-btn" on:click={handlePrimaryAction}>
            {#if step === 1}
                {msg.permission.requestAccess}
            {:else}
                {msg.permission.openAgain}
            {/if}
        </button>
    </div>
</div>

<style>
    .permission-guide {
        width: 100%;
        height: 100%;
        min-height: 0;
        background: #0a0a0a;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        display: flex;
        flex-direction: column;
        overflow: hidden;
        color: white;
        font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", sans-serif;
        position: relative;
        -webkit-app-region: drag;
        box-sizing: border-box;
    }

    .ambient-bg {
        position: absolute;
        inset: 0;
        overflow: hidden;
        pointer-events: none;
    }

    .gradient-orb {
        position: absolute;
        border-radius: 50%;
        filter: blur(80px);
        opacity: 0.35;
    }

    .orb-1 {
        width: 280px;
        height: 280px;
        background: radial-gradient(circle, rgba(255, 79, 0, 0.4) 0%, transparent 70%);
        top: -80px;
        right: -60px;
    }

    .orb-2 {
        width: 220px;
        height: 220px;
        background: radial-gradient(circle, rgba(0, 122, 255, 0.25) 0%, transparent 70%);
        bottom: 40px;
        left: -60px;
    }

    .close-btn {
        position: absolute;
        top: 14px;
        left: 14px;
        width: 26px;
        height: 26px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.08);
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.6);
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 100;
        -webkit-app-region: no-drag;
        transition: all 0.2s;
    }

    .close-btn:hover {
        background: rgba(255, 255, 255, 0.15);
        color: white;
    }

    .content {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: 100%;
        flex: 1;
        min-height: 0;
        padding: 18px 18px 16px;
        box-sizing: border-box;
        -webkit-app-region: no-drag;
        position: relative;
        z-index: 1;
    }

    .step-badge {
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        color: rgba(255, 255, 255, 0.35);
        margin-bottom: 10px;
        flex-shrink: 0;
    }

    .icon-badge {
        width: 36px;
        height: 36px;
        border-radius: 10px;
        background: rgba(255, 79, 0, 0.12);
        border: 1px solid rgba(255, 79, 0, 0.25);
        display: flex;
        align-items: center;
        justify-content: center;
        margin-bottom: 10px;
        color: #ff6b2c;
        flex-shrink: 0;
    }

    .icon-badge svg {
        width: 18px;
        height: 18px;
    }

    .step-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: 100%;
        flex: 1;
        min-height: 0;
        overflow-y: auto;
    }

    h2 {
        font-size: 17px;
        font-weight: 700;
        margin: 0 0 4px;
        letter-spacing: -0.3px;
        text-align: center;
        flex-shrink: 0;
    }

    .description {
        font-size: 12px;
        color: rgba(255, 255, 255, 0.45);
        text-align: center;
        margin: 0 0 14px;
        line-height: 1.45;
        max-width: 280px;
        flex-shrink: 0;
    }

    .description strong {
        color: rgba(255, 255, 255, 0.75);
        font-weight: 600;
    }

    .instruction-text {
        margin-top: 10px;
        font-size: 11px;
        color: rgba(255, 255, 255, 0.4);
        text-align: center;
        line-height: 1.4;
        flex-shrink: 0;
    }

    .instruction-text strong {
        color: rgba(255, 255, 255, 0.85);
        font-weight: 600;
    }

    .schematic-wrap {
        width: 100%;
        display: flex;
        justify-content: center;
    }

    .schematic {
        border-radius: 10px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        overflow: hidden;
        box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
        position: relative;
    }

    .system-dialog {
        width: 100%;
        max-width: 280px;
        padding: 14px 12px 12px;
        background: rgba(40, 40, 42, 0.95);
        backdrop-filter: blur(20px);
    }

    .dialog-header {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-bottom: 10px;
        text-align: center;
    }

    .lock-icon {
        width: 28px;
        height: 28px;
        margin-bottom: 8px;
    }

    .lock-icon svg {
        width: 100%;
        height: 100%;
    }

    .dialog-title {
        font-weight: 600;
        font-size: 11px;
        line-height: 1.35;
        color: rgba(255, 255, 255, 0.9);
    }

    .dialog-body {
        margin-bottom: 10px;
    }

    .dialog-subtitle {
        font-size: 9px;
        color: rgba(255, 255, 255, 0.4);
        text-align: center;
        margin: 0;
        line-height: 1.4;
    }

    .dialog-actions {
        display: flex;
        gap: 8px;
        justify-content: center;
    }

    .btn {
        padding: 5px 14px;
        border-radius: 6px;
        font-size: 11px;
        font-weight: 500;
    }

    .btn.secondary {
        background: rgba(255, 255, 255, 0.08);
        color: rgba(255, 255, 255, 0.55);
    }

    .btn.primary {
        background: #007aff;
        color: white;
        position: relative;
        box-shadow: 0 0 0 2px rgba(0, 122, 255, 0.3);
    }

    .mac-settings {
        width: 100%;
        max-width: 280px;
        background: rgba(30, 30, 32, 0.95);
        backdrop-filter: blur(20px);
    }

    .window-header {
        height: 30px;
        background: rgba(255, 255, 255, 0.04);
        border-bottom: 1px solid rgba(255, 255, 255, 0.06);
        display: flex;
        align-items: center;
        padding: 0 10px;
        gap: 8px;
    }

    .window-controls {
        display: flex;
        gap: 5px;
        flex-shrink: 0;
    }

    .dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
    }

    .red { background: #ff5f57; }
    .yellow { background: #febc2e; }
    .green { background: #28c840; }

    .window-title {
        flex: 1;
        text-align: center;
        font-size: 10px;
        color: rgba(255, 255, 255, 0.35);
        font-weight: 500;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .settings-list {
        padding: 4px 0;
    }

    .list-item {
        display: flex;
        align-items: center;
        padding: 9px 12px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    }

    .list-item.speech-clip {
        background: rgba(255, 79, 0, 0.08);
        border-left: 2px solid #ff4f00;
    }

    .app-icon {
        width: 22px;
        height: 22px;
        border-radius: 5px;
        background: #333;
        margin-right: 10px;
        flex-shrink: 0;
    }

    .app-icon.speech-clip-icon {
        background: linear-gradient(145deg, #16161b, #0a0a0c);
        border: 1px solid rgba(255, 79, 0, 0.4);
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .app-icon.speech-clip-icon img {
        width: 16px;
        height: 16px;
        border-radius: 22%;
    }

    .app-name {
        flex: 1;
        font-size: 12px;
        font-weight: 500;
        color: rgba(255, 255, 255, 0.85);
    }

    .toggle-container {
        position: relative;
    }

    .toggle {
        width: 34px;
        height: 20px;
        background: #444;
        border-radius: 10px;
        position: relative;
    }

    .toggle::after {
        content: "";
        position: absolute;
        top: 2px;
        left: 2px;
        width: 16px;
        height: 16px;
        background: white;
        border-radius: 50%;
        transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }

    .toggle.active-anim {
        animation: toggleOn 3s infinite;
    }

    .toggle.active-anim::after {
        animation: toggleSlide 3s infinite;
    }

    @keyframes toggleOn {
        0%, 45% { background: #444; }
        55%, 100% { background: #34c759; }
    }

    @keyframes toggleSlide {
        0%, 45% { transform: translateX(0); }
        55%, 100% { transform: translateX(14px); }
    }

    .cursor-hand {
        position: absolute;
        width: 20px;
        height: 20px;
        z-index: 20;
        pointer-events: none;
    }

    .cursor-hand.step1 {
        top: 100%;
        right: -10px;
        animation: cursorClick 3s infinite;
    }

    .cursor-hand.step2 {
        top: 100%;
        right: -15px;
        animation: cursorClick 3s infinite;
    }

    @keyframes cursorClick {
        0% { opacity: 0; transform: translate(30px, 30px); }
        10% { opacity: 1; transform: translate(15px, 15px); }
        30% { opacity: 1; transform: translate(0, 0) scale(1); }
        40% { transform: translate(0, 0) scale(0.9); }
        50% { transform: translate(0, 0) scale(1); }
        70% { opacity: 1; transform: translate(5px, 20px); }
        80%, 100% { opacity: 0; transform: translate(10px, 40px); }
    }

    .progress-dots {
        display: flex;
        gap: 8px;
        margin: 10px 0 10px;
        flex-shrink: 0;
    }

    .progress-dots .dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.15);
        transition: all 0.3s ease;
    }

    .progress-dots .dot.active {
        background: #ff4f00;
        width: 18px;
        border-radius: 3px;
    }

    .primary-btn {
        background: linear-gradient(135deg, #ff4f00 0%, #ff6b2c 100%);
        color: white;
        border: none;
        padding: 11px 18px;
        border-radius: 10px;
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        transition: transform 0.1s, box-shadow 0.2s;
        width: 100%;
        flex-shrink: 0;
        box-shadow: 0 4px 16px rgba(255, 79, 0, 0.3);
        -webkit-app-region: no-drag;
    }

    .primary-btn:hover {
        box-shadow: 0 6px 24px rgba(255, 79, 0, 0.4);
    }

    .primary-btn:active {
        transform: scale(0.98);
    }
</style>
