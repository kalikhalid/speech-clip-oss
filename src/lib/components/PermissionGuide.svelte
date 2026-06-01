<script lang="ts">
    import { onMount } from "svelte";
    import { fade, fly } from "svelte/transition";

    export let onRequestPermission: () => void;
    export let onClose: () => void;

    let step: 1 | 2 = 1;

    function handlePrimaryAction() {
        if (step === 1) {
            onRequestPermission();
            step = 2;
            // If user clicked "Open Settings Again", it means they might have missed the window or it's behind.
            // We should re-trigger the permission request which brings the dialog related window to front
            // AND we can move our window out of the way to the side.
            onRequestPermission();
        }
    }
</script>

<div class="permission-guide" transition:fade>
    <!-- Close Button -->
    <button class="close-btn" on:click={onClose}>
        <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
        >
            <path
                d="M11 1L1 11M1 1L11 11"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
            />
        </svg>
    </button>

    <div class="content">
        {#if step === 1}
            <div
                class="step-container"
                in:fly={{ y: 20, duration: 300 }}
                out:fade
            >
                <h2>Требуется доступ</h2>
                <p class="description">
                    Speech Clip нужен доступ к универсальному доступу для ввода
                    текста.
                </p>

                <!-- Step 1: System Dialog Schematic -->
                <div class="schematic system-dialog">
                    <div class="dialog-header">
                        <div class="lock-icon">
                            <div class="lock-body"></div>
                            <div class="lock-shackle"></div>
                        </div>
                        <div class="dialog-title">Универсальный доступ</div>
                    </div>
                    <div class="dialog-body">
                        <div class="dialog-text-lines">
                            <div class="line w-full"></div>
                            <div class="line w-3_4"></div>
                        </div>
                    </div>
                    <div class="dialog-actions">
                        <div class="btn secondary">Запретить</div>
                        <div class="btn primary active-target">
                            Открыть настройки
                            <!-- Cursor Animation -->
                            <div class="cursor-hand step1">
                                <svg
                                    viewBox="0 0 24 24"
                                    fill="white"
                                    filter="drop-shadow(0 2px 4px rgba(0,0,0,0.5))"
                                >
                                    <path
                                        d="M7 2l12 11.2-5.8.5 3.4 8.3-2.6 1-3.4-8.3-4.6 4.3z"
                                    />
                                </svg>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="instruction-text">
                    Нажмите <b>«Открыть сис. настройки»</b> в окне.
                </div>
            </div>
        {:else}
            <div
                class="step-container"
                in:fly={{ y: 20, duration: 300 }}
                out:fade
            >
                <h2>Включите переключатель</h2>
                <p class="description">
                    Найдите <b>Speech Clip</b> в списке и включите его.
                </p>

                <!-- Step 2: Settings Window Schematic -->
                <div class="schematic mac-settings">
                    <div class="window-header">
                        <div class="window-controls">
                            <div class="dot red"></div>
                            <div class="dot yellow"></div>
                            <div class="dot green"></div>
                        </div>
                        <div class="window-title">
                            Конфиденциальность и безопасность
                        </div>
                    </div>

                    <div class="settings-list">
                        <div class="list-item other">
                            <div class="app-icon"></div>
                            <span class="app-name">Другое приложение</span>
                            <div class="toggle"></div>
                        </div>

                        <div class="list-item speech-clip">
                            <div class="app-icon speech-clip-icon">
                                <img src="/logo.png" alt="" width="20" height="20" />
                            </div>
                            <span class="app-name">Speech Clip</span>

                            <!-- Animated Toggle -->
                            <div class="toggle-container">
                                <div class="toggle active-anim"></div>
                                <div class="cursor-hand step2">
                                    <svg
                                        viewBox="0 0 24 24"
                                        fill="white"
                                        filter="drop-shadow(0 2px 4px rgba(0,0,0,0.5))"
                                    >
                                        <path
                                            d="M7 2l12 11.2-5.8.5 3.4 8.3-2.6 1-3.4-8.3-4.6 4.3z"
                                        />
                                    </svg>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        {/if}

        <button class="primary-btn" on:click={handlePrimaryAction}>
            {#if step === 1}
                Запросить доступ
            {:else}
                Открыть настройки снова
            {/if}
        </button>
    </div>
</div>

<style>
    .permission-guide {
        width: 100%;
        height: 100vh;
        background: #111111;
        border-radius: 12px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        display: flex;
        flex-direction: column;
        align-items: center;
        overflow: hidden;
        color: white;
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
            sans-serif;
        /* OS window already has shadow, inner component doesn't need heavy one */
        box-shadow: none;
        -webkit-app-region: drag;
        position: relative;
    }

    .close-btn {
        position: absolute;
        top: 16px;
        left: 16px; /* MacOS close buttons are typically left, but custom UI can vary. User asked for close button. */
        width: 24px;
        height: 24px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.1);
        border: none;
        color: #fff;
        display: flex;
        align-items: center;
        justify-content: center;
        cursor: pointer;
        z-index: 100;
        -webkit-app-region: no-drag;
        transition: all 0.2s;
    }
    .close-btn:hover {
        background: rgba(255, 255, 255, 0.2);
    }

    .content {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: 100%;
        height: 100%;
        padding: 24px 20px 20px 20px;
        box-sizing: border-box;
        -webkit-app-region: no-drag;
    }

    .step-container {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: 100%;
        flex: 1;
    }

    h2 {
        font-size: 20px;
        font-weight: 600;
        margin: 0 0 8px 0;
    }

    .description {
        font-size: 13px;
        color: #888;
        text-align: center;
        margin: 0 0 24px 0;
        line-height: 1.4;
    }

    .instruction-text {
        margin-top: 16px;
        font-size: 13px;
        color: #aaa;
        text-align: center;
    }
    .instruction-text b {
        color: white;
    }

    /* Schematic Base */
    .schematic {
        width: 100%;
        background: #1e1e1e;
        border-radius: 8px;
        border: 1px solid rgba(255, 255, 255, 0.08);
        overflow: hidden;
        box-shadow: 0 8px 20px rgba(0, 0, 0, 0.3);
        position: relative;
    }

    /* Step 1: System Dialog */
    .system-dialog {
        padding: 16px;
        background: #2a2a2a; /* Lighter for modal */
        width: 240px;
    }

    .dialog-header {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-bottom: 12px;
    }

    .lock-icon {
        width: 32px;
        height: 32px;
        background: linear-gradient(135deg, #ffd60a, #ff9f0a);
        border-radius: 6px;
        margin-bottom: 8px;
        position: relative;
    }
    .lock-body {
        position: absolute;
        bottom: 4px;
        left: 4px;
        right: 4px;
        height: 16px;
        background: rgba(255, 255, 255, 0.2);
        border-radius: 2px;
    }
    .lock-shackle {
        position: absolute;
        top: 4px;
        left: 8px;
        width: 16px;
        height: 12px;
        border: 3px solid rgba(255, 255, 255, 0.4);
        border-bottom: none;
        border-radius: 8px 8px 0 0;
    }

    .dialog-title {
        font-weight: 600;
        font-size: 13px;
    }

    .dialog-body {
        margin-bottom: 16px;
    }
    .dialog-text-lines {
        display: flex;
        flex-direction: column;
        gap: 4px;
        align-items: center;
    }
    .line {
        height: 4px;
        background: rgba(255, 255, 255, 0.1);
        border-radius: 2px;
    }
    .w-full {
        width: 100%;
    }
    .w-3_4 {
        width: 75%;
    }

    .dialog-actions {
        display: flex;
        gap: 8px;
        justify-content: center;
    }
    .btn {
        padding: 4px 12px;
        border-radius: 6px;
        font-size: 10px;
        font-weight: 500;
    }
    .btn.secondary {
        background: rgba(255, 255, 255, 0.1);
        color: #aaa;
    }
    .btn.primary {
        background: #007aff;
        color: white;
        position: relative;
    }

    /* Step 2: Mac Settings */
    .mac-settings {
        background: #1e1e1e;
    }
    .window-header {
        height: 28px;
        background: #2a2a2a;
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        display: flex;
        align-items: center;
        padding: 0 10px;
    }
    .window-controls {
        display: flex;
        gap: 5px;
    }
    .dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
    }
    .red {
        background: #ff5f57;
    }
    .yellow {
        background: #febc2e;
    }
    .green {
        background: #28c840;
    }

    .window-title {
        flex: 1;
        text-align: center;
        font-size: 10px;
        color: #666;
        font-weight: 500;
    }

    .settings-list {
        padding: 4px 0;
    }
    .list-item {
        display: flex;
        align-items: center;
        padding: 8px 12px;
        border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    }
    .list-item.speech-clip {
        background: rgba(255, 255, 255, 0.05);
    }

    .app-icon {
        width: 20px;
        height: 20px;
        border-radius: 5px;
        background: #333;
        margin-right: 10px;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    .app-icon.speech-clip-icon {
        background: linear-gradient(145deg, #16161b, #0a0a0c);
        border: 1px solid rgba(255, 79, 0, 0.4);
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
    }

    .toggle {
        width: 32px;
        height: 18px;
        background: #444;
        border-radius: 10px;
        position: relative;
    }
    .toggle::after {
        content: "";
        position: absolute;
        top: 2px;
        left: 2px;
        width: 14px;
        height: 14px;
        background: white;
        border-radius: 50%;
        transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    }

    /* Animations */
    .toggle.active-anim {
        animation: toggleOn 3s infinite;
    }
    .toggle.active-anim::after {
        animation: toggleSlide 3s infinite;
    }

    @keyframes toggleOn {
        0%,
        45% {
            background: #444;
        }
        55%,
        100% {
            background: #007aff;
        }
    }
    @keyframes toggleSlide {
        0%,
        45% {
            transform: translateX(0);
        }
        55%,
        100% {
            transform: translateX(14px);
        }
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
        0% {
            opacity: 0;
            transform: translate(30px, 30px);
        }
        10% {
            opacity: 1;
            transform: translate(15px, 15px);
        }
        30% {
            opacity: 1;
            transform: translate(0, 0) scale(1);
        }
        40% {
            transform: translate(0, 0) scale(0.9);
        } /* Click */
        50% {
            transform: translate(0, 0) scale(1);
        }
        70% {
            opacity: 1;
            transform: translate(5px, 20px);
        }
        80%,
        100% {
            opacity: 0;
            transform: translate(10px, 40px);
        }
    }

    /* Main Action Button */
    .primary-btn {
        margin-top: auto;
        background: white;
        color: black;
        border: none;
        padding: 12px 20px;
        border-radius: 8px;
        font-size: 14px;
        font-weight: 600;
        cursor: pointer;
        transition:
            transform 0.1s,
            background 0.2s;
        width: 100%;
    }
    .primary-btn:hover {
        background: #f0f0f0;
    }
    .primary-btn:active {
        transform: scale(0.98);
    }
</style>
