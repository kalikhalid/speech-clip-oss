<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { fade, fly, scale } from "svelte/transition";
    import { quintOut } from "svelte/easing";

    let currentStep = $state(0);
    let isBrowserOpening = $state(false);
    let showEmailLogin = $state(false);
    let email = $state("");
    let password = $state("");
    let isLoggingIn = $state(false);
    let loginError = $state("");
    let waveBars = $state(new Array(7).fill(4));
    let demoText = $state("");
    let touchStartX = $state(0);
    let touchEndX = $state(0);
    const fullDemoText = "Привет! Это Speech Clip — голосовой набор текста с AI.";

    onMount(() => {
        // Wave animation
        const waveInterval = setInterval(() => {
            if (currentStep === 0) {
                waveBars = waveBars.map(() => 3 + Math.random() * 18);
            }
        }, 80);

        // Typing animation
        const typeInterval = setInterval(() => {
            if (currentStep === 1 && demoText.length < fullDemoText.length) {
                demoText = fullDemoText.slice(0, demoText.length + 1);
            } else if (currentStep !== 1) {
                demoText = "";
            }
        }, 60);

        // Keyboard navigation
        const handleKeydown = (e: KeyboardEvent) => {
            if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                if (!showEmailLogin) nextStep();
            } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                if (!showEmailLogin) prevStep();
            }
        };
        window.addEventListener('keydown', handleKeydown);

        return () => {
            clearInterval(waveInterval);
            clearInterval(typeInterval);
            window.removeEventListener('keydown', handleKeydown);
        };
    });

    function nextStep() {
        if (currentStep < 2) {
            currentStep++;
        }
    }

    function prevStep() {
        if (currentStep > 0) {
            currentStep--;
        }
    }

    function goToStep(step: number) {
        currentStep = step;
        showEmailLogin = false;
    }

    // Touch handlers for swipe
    function handleTouchStart(e: TouchEvent) {
        touchStartX = e.changedTouches[0].screenX;
    }

    function handleTouchEnd(e: TouchEvent) {
        touchEndX = e.changedTouches[0].screenX;
        handleSwipe();
    }

    function handleSwipe() {
        const swipeThreshold = 50;
        const diff = touchStartX - touchEndX;
        
        if (diff > swipeThreshold) {
            // Swipe left - next
            if (!showEmailLogin) nextStep();
        } else if (diff < -swipeThreshold) {
            // Swipe right - previous
            if (!showEmailLogin) prevStep();
        }
    }

    async function handleLoginWithBrowser() {
        isBrowserOpening = true;
        try {
            await invoke("open_browser_login");
            setTimeout(() => { isBrowserOpening = false; }, 2000);
        } catch (error) {
            isBrowserOpening = false;
        }
    }

    async function handleEmailLogin(e: SubmitEvent) {
        e.preventDefault();
        if (!email || !password) return;
        
        isLoggingIn = true;
        loginError = "";
        
        try {
            await invoke("login", { email, password });
            // The user-logged-in event will be handled by the parent/main route
        } catch (error: any) {
            console.error("Login failed:", error);
            loginError = error.toString();
            isLoggingIn = false;
        }
    }
</script>

<div class="onboarding-container" transition:fade={{ duration: 400 }}>
    <div class="ambient-bg">
        <div class="gradient-orb orb-1"></div>
        <div class="gradient-orb orb-2"></div>
        <div class="grid-pattern"></div>
    </div>

    <div class="content-wrapper" class:login-mode={showEmailLogin}>
        <!-- Logo -->
        <div class="logo-section" class:compact={showEmailLogin}>
            <div class="logo-badge" in:scale={{ duration: 600, easing: quintOut }}>
                <img src="/logo.png" alt="" width="32" height="32" class="logo-img" />
            </div>
            <h1 class="app-title">Speech Clip</h1>
            {#if !showEmailLogin}
                <p class="app-subtitle">Голосовой набор текста с AI</p>
            {/if}
        </div>

        {#if !showEmailLogin}
            <!-- Feature Showcase with Swipe -->
            <div class="showcase-wrapper" transition:fade={{ duration: 200 }}>
                <!-- Prev Button -->
                <button 
                    class="nav-btn prev" 
                    onclick={prevStep}
                    disabled={currentStep === 0}
                    aria-label="Предыдущий шаг"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="15 18 9 12 15 6"/>
                    </svg>
                </button>

                <div 
                    class="showcase-area"
                    ontouchstart={handleTouchStart}
                    ontouchend={handleTouchEnd}
                >
                    {#if currentStep === 0}
                        <div class="feature-demo" in:fly={{ y: 30, duration: 500 }} out:fly={{ y: -30, duration: 300 }}>
                            <div class="demo-window">
                                <div class="window-header">
                                    <div class="window-dots"><span></span><span></span><span></span></div>
                                </div>
                                <div class="demo-content recording-demo">
                                    <div class="wave-container">
                                        {#each waveBars as height}
                                            <div class="wave-bar" style="height: {height}px;"></div>
                                        {/each}
                                    </div>
                                    <div class="recording-badge">
                                        <span class="recording-dot"></span>
                                        Запись
                                    </div>
                                </div>
                            </div>
                            <div class="feature-text">
                                <h3>Зажмите <kbd>Ctrl</kbd>+<kbd>`</kbd> и говорите</h3>
                                <p>Текст появляется мгновенно. Никаких кликов — просто голос.</p>
                            </div>
                        </div>
                    {/if}

                    {#if currentStep === 1}
                        <div class="feature-demo" in:fly={{ y: 30, duration: 500 }} out:fly={{ y: -30, duration: 300 }}>
                            <div class="demo-window wide">
                                <div class="window-header">
                                    <div class="window-dots"><span></span><span></span><span></span></div>
                                    <span class="window-title">VS Code — main.ts</span>
                                </div>
                                <div class="demo-content typing-demo">
                                    <div class="code-area">
                                        <span class="typed-text">{demoText}</span><span class="cursor">|</span>
                                    </div>
                                    <div class="ai-badge">
                                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <path d="M12 2L2 7l10 5 10-5-10-5z"/>
                                            <path d="M2 17l10 5 10-5"/>
                                            <path d="M2 12l10 5 10-5"/>
                                        </svg>
                                        AI форматирует
                                    </div>
                                </div>
                            </div>
                            <div class="feature-text">
                                <h3>ИИ форматирует за вас</h3>
                                <p>Пунктуация, структура и стиль — всё на месте автоматически.</p>
                            </div>
                        </div>
                    {/if}

                    {#if currentStep === 2}
                        <div class="feature-demo" in:fly={{ y: 30, duration: 500 }} out:fly={{ y: -30, duration: 300 }}>
                            <div class="apps-showcase">
                                <div class="app-icon vscode" title="VS Code">
                                    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M17.6.1a1.5 1.5 0 0 1 1.3 1.3v21a1.5 1.5 0 0 1-2 1.4l-16-8A1.5 1.5 0 0 1 0 15.5v-7a1.5 1.5 0 0 1 .8-1.3l16-8a1.5 1.5 0 0 1 1.7 0z"/></svg>
                                </div>
                                <div class="app-icon notion" title="Notion">
                                    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M4.5 4.2c.7.6 1 .6 2.4.5l13.2-.8c.3 0 0-.3-.1-.3L17.9 2c-.4-.3-1-.7-2.1-.6L3 2.8c-.5 0-.6.3-.4.5l2 2zm.8 3.1v13.9c0 .7.4 1 1.2 1l14.5-.8c.8 0 1-.6 1-1.2V6.4c0-.6-.2-.9-.7-.9l-15.2.9c-.6 0-.8.3-.8.9z"/></svg>
                                </div>
                                <div class="app-icon telegram" title="Telegram">
                                    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.4 0 0 5.4 0 12s5.4 12 12 12 12-5.4 12-12S18.6 0 12 0zm5 7.2c.1 0 .3 0 .5.1.2.2.2.3.2.5-.2 1.9-1 6.5-1.4 8.6-.2.9-.5 1.2-.8 1.2-.7.1-1.2-.5-1.9-.9-1.1-.7-1.7-1.1-2.7-1.8-1.2-.8-.4-1.2.3-1.9.2-.2 3.2-3 3.3-3.2 0 0 0-.2-.1-.2s-.2 0-.2 0c-.1 0-1.8 1.1-5.1 3.3-.5.3-.9.5-1.3.5-.4 0-1.3-.2-1.9-.4-.8-.2-1.3-.4-1.3-.8 0-.2.3-.4.9-.7 3.5-1.5 5.8-2.5 7-3 3.3-1.4 4-1.6 4.5-1.7z"/></svg>
                                </div>
                                <div class="app-icon safari" title="Safari">
                                    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.4 0 0 5.4 0 12s5.4 12 12 12 12-5.4 12-12S18.6 0 12 0zm0 22c-5.5 0-10-4.5-10-10S6.5 2 12 2s10 4.5 10 10-4.5 10-10 10zm6.5-10c0 3.6-2.9 6.5-6.5 6.5S5.5 15.6 5.5 12 8.4 5.5 12 5.5s6.5 2.9 6.5 6.5z"/></svg>
                                </div>
                            </div>
                            <div class="feature-text">
                                <h3>Работает везде</h3>
                                <p>VS Code, Notion, Telegram, Safari — в любое приложение мгновенно.</p>
                            </div>
                        </div>
                    {/if}
                </div>

                <!-- Next Button -->
                <button 
                    class="nav-btn next" 
                    onclick={nextStep}
                    disabled={currentStep === 2}
                    aria-label="Следующий шаг"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="9 18 15 12 9 6"/>
                    </svg>
                </button>
            </div>

            <!-- Progress -->
            <div class="progress-dots" transition:fade={{ duration: 200 }}>
                {#each [0, 1, 2] as step}
                    <button 
                        class="dot" 
                        class:active={step === currentStep}
                        class:completed={step < currentStep}
                        onclick={() => goToStep(step)}
                        aria-label="Шаг {step + 1}"
                    ></button>
                {/each}
            </div>

            <!-- CTA -->
            <div class="cta-section" class:visible={currentStep === 2}>
                <button class="primary-btn" onclick={handleLoginWithBrowser} disabled={isBrowserOpening}>
                    {#if isBrowserOpening}
                        <span class="spinner"></span>
                        <span>Открываем браузер...</span>
                    {:else}
                        <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
                            <polyline points="15 3 21 3 21 9"/>
                            <line x1="10" y1="14" x2="21" y2="3"/>
                        </svg>
                        <span>Войти через браузер</span>
                    {/if}
                </button>
                <button class="secondary-btn" onclick={() => showEmailLogin = true}>
                    <span>Войти через почту</span>
                </button>
                <p class="helper-text">
                    Нет аккаунта? <a href="https://speechclip.dev" target="_blank">Создать бесплатно</a>
                </p>
            </div>
        {:else}
            <!-- Email Login Form -->
            <div class="login-form-container" in:fly={{ y: 20, duration: 400 }} out:fade>
                <form onsubmit={handleEmailLogin} class="login-form">
                    <div class="form-group">
                        <label for="email">Email</label>
                        <input 
                            type="email" 
                            id="email" 
                            bind:value={email} 
                            placeholder="your@email.com" 
                            required
                            disabled={isLoggingIn}
                        />
                    </div>
                    <div class="form-group">
                        <label for="password">Пароль</label>
                        <input 
                            type="password" 
                            id="password" 
                            bind:value={password} 
                            placeholder="••••••••" 
                            required
                            disabled={isLoggingIn}
                        />
                    </div>

                    {#if loginError}
                        <div class="error-message" transition:fade>
                            {loginError}
                        </div>
                    {/if}

                    <button type="submit" class="primary-btn" disabled={isLoggingIn}>
                        {#if isLoggingIn}
                            <span class="spinner"></span>
                            <span>Входим...</span>
                        {:else}
                            <span>Войти</span>
                        {/if}
                    </button>
                </form>

                <button class="text-btn" onclick={() => showEmailLogin = false} disabled={isLoggingIn}>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="15 18 9 12 15 6"/>
                    </svg>
                    Назад к обзору
                </button>
            </div>
        {/if}
    </div>
</div>

<style>
    .onboarding-container {
        position: fixed;
        inset: 0;
        z-index: 9999;
        background: #0a0a0a;
        display: flex;
        align-items: center;
        justify-content: center;
        font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", sans-serif;
    }

    .ambient-bg {
        position: absolute;
        inset: 0;
        overflow: hidden;
    }

    .gradient-orb {
        position: absolute;
        border-radius: 50%;
        filter: blur(100px);
        opacity: 0.4;
    }

    .orb-1 {
        width: 500px;
        height: 500px;
        background: radial-gradient(circle, rgba(255, 79, 0, 0.35) 0%, transparent 70%);
        top: -150px;
        right: -150px;
        animation: float 15s ease-in-out infinite;
    }

    .orb-2 {
        width: 400px;
        height: 400px;
        background: radial-gradient(circle, rgba(0, 122, 255, 0.2) 0%, transparent 70%);
        bottom: -100px;
        left: -100px;
        animation: float 15s ease-in-out infinite reverse;
    }

    .grid-pattern {
        position: absolute;
        inset: 0;
        background-image: 
            linear-gradient(rgba(255,255,255,0.02) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.02) 1px, transparent 1px);
        background-size: 50px 50px;
    }

    @keyframes float {
        0%, 100% { transform: translate(0, 0) scale(1); }
        50% { transform: translate(20px, -20px) scale(1.05); }
    }

    .content-wrapper {
        position: relative;
        z-index: 1;
        display: flex;
        flex-direction: column;
        align-items: center;
        max-width: 520px;
        width: 100%;
        padding: 24px;
        transition: all 0.5s cubic-bezier(0.16, 1, 0.3, 1);
    }

    .content-wrapper.login-mode {
        max-width: 400px;
    }

    /* Logo */
    .logo-section {
        text-align: center;
        margin-bottom: 16px;
        transition: all 0.5s cubic-bezier(0.16, 1, 0.3, 1);
    }

    .logo-section.compact {
        margin-bottom: 12px;
        transform: scale(0.9);
    }

    .logo-badge {
        width: 64px;
        height: 64px;
        background: linear-gradient(145deg, #16161b 0%, #0a0a0c 100%);
        border: 1px solid rgba(255, 79, 0, 0.35);
        border-radius: 18px;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto 16px;
        box-shadow: 0 16px 40px rgba(255, 79, 0, 0.22);
        animation: logoPulse 3s ease-in-out infinite;
    }

    @keyframes logoPulse {
        0%, 100% { transform: scale(1); box-shadow: 0 16px 40px rgba(255, 79, 0, 0.22); }
        50% { transform: scale(1.03); box-shadow: 0 20px 50px rgba(255, 79, 0, 0.32); }
    }

    .logo-badge .logo-img {
        width: 32px;
        height: 32px;
        display: block;
        border-radius: 22%;
    }

    .app-title {
        font-size: 28px;
        font-weight: 700;
        color: white;
        margin: 0 0 6px 0;
        letter-spacing: -0.5px;
    }

    .app-subtitle {
        font-size: 15px;
        color: rgba(255, 255, 255, 0.5);
        margin: 0;
    }

    /* Showcase */
    .showcase-wrapper {
        width: 100%;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 16px;
        margin-bottom: 20px;
        position: relative;
    }

    .showcase-area {
        flex: 1;
        min-height: 200px;
        display: flex;
        align-items: center;
        justify-content: center;
        position: relative;
        touch-action: pan-y;
        user-select: none;
    }

    .nav-btn {
        width: 40px;
        height: 40px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        color: rgba(255, 255, 255, 0.6);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s ease;
        flex-shrink: 0;
        padding: 0;
    }

    .nav-btn:hover:not(:disabled) {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        transform: scale(1.1);
    }

    .nav-btn:disabled {
        opacity: 0.2;
        cursor: not-allowed;
    }

    .nav-btn svg {
        width: 20px;
        height: 20px;
    }

    .feature-demo {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: 100%;
        position: absolute;
    }

    /* Demo Window */
    .demo-window {
        width: 200px;
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 12px;
        overflow: hidden;
        margin-bottom: 16px;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
    }

    .demo-window.wide {
        width: 320px;
    }

    .window-header {
        height: 32px;
        background: rgba(255, 255, 255, 0.04);
        border-bottom: 1px solid rgba(255, 255, 255, 0.05);
        display: flex;
        align-items: center;
        padding: 0 12px;
    }

    .window-dots {
        display: flex;
        gap: 6px;
    }

    .window-dots span {
        width: 8px;
        height: 8px;
        border-radius: 50%;
    }

    .window-dots span:nth-child(1) { background: #ff5f57; }
    .window-dots span:nth-child(2) { background: #febc2e; }
    .window-dots span:nth-child(3) { background: #28c840; }

    .window-title {
        margin-left: auto;
        font-size: 11px;
        color: rgba(255, 255, 255, 0.3);
        font-weight: 500;
    }

    .demo-content {
        padding: 16px;
        display: flex;
        flex-direction: column;
        align-items: center;
        min-height: 80px;
        justify-content: center;
    }

    /* Recording Demo */
    .recording-demo {
        gap: 12px;
    }

    .wave-container {
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 4px;
        height: 36px;
    }

    .wave-bar {
        width: 4px;
        background: linear-gradient(180deg, #ff4f00 0%, #ff8c5a 100%);
        border-radius: 2px;
        transition: height 0.08s ease;
        box-shadow: 0 0 12px rgba(255, 79, 0, 0.4);
    }

    .recording-badge {
        display: flex;
        align-items: center;
        gap: 8px;
        font-size: 12px;
        color: rgba(255, 255, 255, 0.6);
        background: rgba(255, 79, 0, 0.15);
        padding: 4px 12px;
        border-radius: 20px;
        border: 1px solid rgba(255, 79, 0, 0.2);
    }

    .recording-dot {
        width: 6px;
        height: 6px;
        background: #ff4f00;
        border-radius: 50%;
        animation: pulse 1.5s ease-in-out infinite;
        box-shadow: 0 0 8px rgba(255, 79, 0, 0.6);
    }

    @keyframes pulse {
        0%, 100% { opacity: 1; transform: scale(1); }
        50% { opacity: 0.5; transform: scale(1.2); }
    }

    /* Typing Demo */
    .typing-demo {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        position: relative;
        padding-bottom: 4px;
    }

    .code-area {
        font-family: "SF Mono", Monaco, monospace;
        font-size: 13px;
        color: rgba(255, 255, 255, 0.8);
        line-height: 1.6;
        min-height: 40px;
    }

    .cursor {
        color: #ff4f00;
        animation: blink 1s step-end infinite;
    }

    @keyframes blink {
        0%, 100% { opacity: 1; }
        50% { opacity: 0; }
    }

    .ai-badge {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 11px;
        color: #007aff;
        background: rgba(0, 122, 255, 0.15);
        padding: 4px 10px;
        border-radius: 20px;
        border: 1px solid rgba(0, 122, 255, 0.25);
        animation: slideUp 0.5s ease;
        margin-top: 8px;
        align-self: flex-end;
    }

    @keyframes slideUp {
        from { opacity: 0; transform: translateY(10px); }
        to { opacity: 1; transform: translateY(0); }
    }

    .ai-badge svg {
        width: 14px;
        height: 14px;
    }

    /* Apps Showcase */
    .apps-showcase {
        display: flex;
        gap: 16px;
        margin-bottom: 16px;
    }

    .app-icon {
        width: 52px;
        height: 52px;
        border-radius: 12px;
        display: flex;
        align-items: center;
        justify-content: center;
        animation: popIn 0.5s ease backwards;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
    }

    .app-icon:nth-child(1) { animation-delay: 0s; }
    .app-icon:nth-child(2) { animation-delay: 0.1s; }
    .app-icon:nth-child(3) { animation-delay: 0.2s; }
    .app-icon:nth-child(4) { animation-delay: 0.3s; }

    @keyframes popIn {
        from { opacity: 0; transform: scale(0.5); }
        to { opacity: 1; transform: scale(1); }
    }

    .app-icon.vscode {
        background: linear-gradient(135deg, #007acc 0%, #1f9cf0 100%);
        color: white;
    }

    .app-icon.notion {
        background: white;
        color: #000;
    }

    .app-icon.telegram {
        background: linear-gradient(135deg, #0088cc 0%, #00a8e6 100%);
        color: white;
    }

    .app-icon.safari {
        background: linear-gradient(135deg, #00d4ff 0%, #007aff 100%);
        color: white;
    }

    .app-icon svg {
        width: 26px;
        height: 26px;
    }

    /* Feature Text */
    .feature-text {
        text-align: center;
        max-width: 400px;
    }

    .feature-text h3 {
        font-size: 20px;
        font-weight: 600;
        color: white;
        margin: 0 0 6px 0;
        line-height: 1.3;
    }

    .feature-text kbd {
        display: inline-flex;
        align-items: center;
        gap: 2px;
        background: rgba(255, 255, 255, 0.1);
        padding: 3px 6px;
        border-radius: 6px;
        font-size: 14px;
        font-weight: 600;
        font-family: inherit;
        border: 1px solid rgba(255, 255, 255, 0.15);
        margin: 0 2px;
    }

    .feature-text p {
        font-size: 14px;
        color: rgba(255, 255, 255, 0.5);
        margin: 0;
        line-height: 1.5;
    }

    /* Progress */
    .progress-dots {
        display: flex;
        gap: 10px;
        margin-bottom: 20px;
    }

    .dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: rgba(255, 255, 255, 0.15);
        border: none;
        cursor: pointer;
        transition: all 0.3s ease;
        padding: 0;
    }

    .dot:hover {
        background: rgba(255, 255, 255, 0.3);
    }

    .dot.active {
        background: #ff4f00;
        width: 24px;
        border-radius: 4px;
    }

    .dot.completed {
        background: rgba(255, 79, 0, 0.5);
    }

    /* CTA */
    .cta-section {
        width: 100%;
        max-width: 320px;
        opacity: 0;
        transform: translateY(20px);
        transition: all 0.5s cubic-bezier(0.16, 1, 0.3, 1);
        pointer-events: none;
    }

    .cta-section.visible {
        opacity: 1;
        transform: translateY(0);
        pointer-events: auto;
    }

    .primary-btn {
        width: 100%;
        padding: 14px 24px;
        background: linear-gradient(135deg, #ff4f00 0%, #ff6b2c 100%);
        border: none;
        border-radius: 14px;
        color: white;
        font-size: 16px;
        font-weight: 600;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 10px;
        transition: all 0.3s ease;
        box-shadow: 0 4px 20px rgba(255, 79, 0, 0.3);
        margin-bottom: 12px;
    }

    .primary-btn:hover:not(:disabled) {
        transform: translateY(-2px);
        box-shadow: 0 8px 30px rgba(255, 79, 0, 0.4);
    }

    .primary-btn:active:not(:disabled) {
        transform: translateY(0);
    }

    .primary-btn:disabled {
        opacity: 0.7;
        cursor: wait;
    }

    .secondary-btn {
        width: 100%;
        padding: 12px 24px;
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 14px;
        color: rgba(255, 255, 255, 0.8);
        font-size: 14px;
        font-weight: 500;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.3s ease;
        margin-bottom: 12px;
    }

    .secondary-btn:hover {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border-color: rgba(255, 255, 255, 0.2);
    }

    /* Login Form */
    .login-form-container {
        width: 100%;
        display: flex;
        flex-direction: column;
        gap: 16px;
    }

    .login-form {
        display: flex;
        flex-direction: column;
        gap: 12px;
    }

    .form-group {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .form-group label {
        font-size: 13px;
        font-weight: 500;
        color: rgba(255, 255, 255, 0.5);
        padding-left: 4px;
    }

    .form-group input {
        background: rgba(255, 255, 255, 0.03);
        border: 1px solid rgba(255, 255, 255, 0.08);
        border-radius: 12px;
        padding: 12px 16px;
        color: white;
        font-size: 15px;
        transition: all 0.2s ease;
    }

    .form-group input:focus {
        outline: none;
        background: rgba(255, 255, 255, 0.05);
        border-color: rgba(255, 79, 0, 0.4);
        box-shadow: 0 0 0 4px rgba(255, 79, 0, 0.1);
    }

    .error-message {
        font-size: 13px;
        color: #ff5f57;
        background: rgba(255, 95, 87, 0.1);
        padding: 8px 12px;
        border-radius: 10px;
        border: 1px solid rgba(255, 95, 87, 0.2);
    }

    .text-btn {
        background: transparent;
        border: none;
        color: rgba(255, 255, 255, 0.4);
        font-size: 14px;
        font-weight: 500;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        transition: all 0.2s ease;
        padding: 6px;
        margin-top: 4px;
    }

    .text-btn:hover:not(:disabled) {
        color: white;
    }

    .text-btn svg {
        width: 16px;
        height: 16px;
    }

    .btn-icon {
        width: 18px;
        height: 18px;
    }

    .spinner {
        width: 18px;
        height: 18px;
        border: 2px solid rgba(255, 255, 255, 0.3);
        border-top-color: white;
        border-radius: 50%;
        animation: spin 0.8s linear infinite;
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }

    .helper-text {
        font-size: 13px;
        color: rgba(255, 255, 255, 0.4);
        text-align: center;
        margin: 0;
    }

    .helper-text a {
        color: #ff4f00;
        text-decoration: none;
        font-weight: 500;
        transition: color 0.2s;
    }

    .helper-text a:hover {
        color: #ff6b2c;
        text-decoration: underline;
    }

    @media (max-width: 480px) {
        .content-wrapper {
            padding: 24px 20px;
        }

        .app-title {
            font-size: 28px;
        }

        .demo-window.wide {
            width: 280px;
        }

        .apps-showcase {
            gap: 12px;
        }

        .app-icon {
            width: 48px;
            height: 48px;
        }

        .app-icon svg {
            width: 24px;
            height: 24px;
        }

        .feature-text h3 {
            font-size: 20px;
        }
    }
</style>
