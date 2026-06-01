<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { fade } from "svelte/transition";
  import PermissionGuide from "$lib/components/PermissionGuide.svelte";
  import ShortcutHint from "$lib/components/ShortcutHint.svelte";
  import { config } from "$lib/config";

  // Survives HMR remounts — prevents duplicate hotkey listeners in dev.
  let overlayListenerCleanups: Array<() => void> = [];

  function teardownOverlayListeners() {
    for (const cleanup of overlayListenerCleanups) {
      cleanup();
    }
    overlayListenerCleanups = [];
  }

  type AppState = "idle" | "recording" | "processing";
  let state: AppState = $state("idle");
  let needsPermission = $state(false);
  let audioContext: AudioContext | null = $state(null);
  let analyser: AnalyserNode | null = null;
  let frequencyData: Uint8Array | null = null;
  let animationFrameId: number | null = null;
  let barHeights: number[] = $state(
    new Array(config.barCount).fill(config.barMinHeight),
  );
  let nextHeights: number[] = new Array(config.barCount).fill(
    config.barMinHeight,
  );
  let mediaStream: MediaStream | null = $state(null);
  let mediaRecorder: MediaRecorder | null = null;
  let audioChunks: Blob[] = [];
  let isHovered = $state(false);
  let showShortcutHint = $state(false);
  let currentHotkey = $state("control+`");

  let unlistenPressed: (() => void) | null = null;
  let unlistenReleased: (() => void) | null = null;
  let unlistenHover: (() => void) | null = null;
  let unlistenOverlayClick: (() => void) | null = null;
  let unlistenCaptureStart: (() => void) | null = null;
  let unlistenCaptureEnd: (() => void) | null = null;
  let unlistenParakeetInstall: (() => void) | null = null;
  let unlistenSettingsUpdated: (() => void) | null = null;
  let releaseTimestamp: number | null = null;
  let recordingStartTime: number | null = null;
  let isCapturingHotkey = $state(false);
  let installStatusMessage = $state("");
  let soundEffectsEnabled = $state(true);
  let isProcessingRecording = false;
  let isStoppingRecording = false;
  let lastHotkeyReleaseAt = 0;
  const MAX_DURATION_MS = 15 * 60 * 1000; // 15 minutes

  async function refreshSoundEffectsSetting() {
    try {
      const appSettings = await invoke<{ sound_effects_enabled: boolean }>(
        "get_settings",
      );
      soundEffectsEnabled = appSettings.sound_effects_enabled;
    } catch (e) {
      console.warn("Failed to load sound setting:", e);
    }
  }

  function checkDuration() {
    if (state !== "recording" || !recordingStartTime) return;

    const elapsed = Date.now() - recordingStartTime;
    if (elapsed >= MAX_DURATION_MS) {
      console.warn("⏱️ Max duration reached, stopping recording");
      stopRecording();
      return;
    }

    requestAnimationFrame(checkDuration);
  }

  // Sound effects - store as preloaded Audio elements
  let startSound: HTMLAudioElement;
  let endSound: HTMLAudioElement;
  let loadingSound: HTMLAudioElement;
  let soundsReady = false;

  // Preload a single sound and return a promise
  function preloadSound(src: string): Promise<HTMLAudioElement> {
    return new Promise((resolve, reject) => {
      const audio = new Audio(src);
      audio.preload = "auto";
      audio.addEventListener("canplaythrough", () => resolve(audio), {
        once: true,
      });
      audio.addEventListener("error", (e) => reject(e), { once: true });
      audio.load();
    });
  }

  // Helper function to play sound - clones audio for reliable playback
  function playSound(audio: HTMLAudioElement) {
    if (!soundEffectsEnabled || !audio || !soundsReady) {
      return;
    }
    try {
      // Clone the audio element to allow overlapping plays and avoid race conditions
      const clone = audio.cloneNode() as HTMLAudioElement;
      clone.volume = 1.0;
      clone
        .play()
        .catch((err) => {});
    } catch (err) {
      // Sound play error silently ignored
    }
  }

  async function resampleAudioBufferAsync(
    audioBuffer: AudioBuffer,
    targetSampleRate: number,
  ): Promise<AudioBuffer> {
    if (audioBuffer.sampleRate === targetSampleRate) {
      return audioBuffer;
    }

    const frameCount = Math.max(
      1,
      Math.round(audioBuffer.duration * targetSampleRate),
    );
    const offline = new OfflineAudioContext(
      1,
      frameCount,
      targetSampleRate,
    );
    const source = offline.createBufferSource();
    source.buffer = audioBuffer;
    source.connect(offline.destination);
    source.start(0);
    return offline.startRendering();
  }

  function encodeAudioBufferToWav(
    audioBuffer: AudioBuffer,
    targetSampleRate = 16000,
  ): Uint8Array {
    const numChannels = 1;
    const sampleRate = audioBuffer.sampleRate;
    const numSamples = audioBuffer.length;
    const bytesPerSample = 2;
    const blockAlign = bytesPerSample;
    const byteRate = sampleRate * blockAlign;
    const dataSize = numSamples * bytesPerSample;
    const headerSize = 44;
    const wavBuffer = new ArrayBuffer(headerSize + dataSize);
    const view = new DataView(wavBuffer);

    const writeString = (offset: number, value: string) => {
      for (let i = 0; i < value.length; i++) {
        view.setUint8(offset + i, value.charCodeAt(i));
      }
    };

    writeString(0, "RIFF");
    view.setUint32(4, 36 + dataSize, true);
    writeString(8, "WAVE");
    writeString(12, "fmt ");
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);
    view.setUint16(22, 1, true);
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, byteRate, true);
    view.setUint16(32, blockAlign, true);
    view.setUint16(34, 16, true);
    writeString(36, "data");
    view.setUint32(40, dataSize, true);

    const channels: Float32Array[] = [];
    for (let i = 0; i < audioBuffer.numberOfChannels; i++) {
      channels.push(audioBuffer.getChannelData(i));
    }

    let offset = headerSize;
    for (let i = 0; i < numSamples; i++) {
      let sample = 0;
      for (let ch = 0; ch < numChannels; ch++) {
        sample += channels[ch][i];
      }
      sample /= numChannels;

      const clamped = Math.max(-1, Math.min(1, sample));
      const pcm = clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff;
      view.setInt16(offset, pcm, true);
      offset += bytesPerSample;
    }

    return new Uint8Array(wavBuffer);
  }

  async function convertRecordingToWav(audioBlob: Blob): Promise<Uint8Array> {
    const encodedBuffer = await audioBlob.arrayBuffer();
    let decoderContext: AudioContext | null = null;
    let shouldCloseContext = false;
    try {
      if (audioContext && audioContext.state !== "closed") {
        decoderContext = audioContext;
      } else {
        decoderContext = new AudioContext();
        shouldCloseContext = true;
      }

      const decodedBuffer = await decoderContext.decodeAudioData(
        encodedBuffer.slice(0),
      );
      const resampled = await resampleAudioBufferAsync(decodedBuffer, 16000);
      return encodeAudioBufferToWav(resampled, 16000);
    } catch (error) {
      throw new Error(`Failed to convert recording to WAV: ${String(error)}`);
    } finally {
      if (decoderContext && shouldCloseContext) {
        try {
          await decoderContext.close();
        } catch (_) {}
      }
    }
  }

  function measureVoiceActivity(): number {
    if (!analyser) return 0;

    if (
      !frequencyData ||
      frequencyData.length !== analyser.frequencyBinCount
    ) {
      frequencyData = new Uint8Array(analyser.frequencyBinCount);
    }

    analyser.getByteFrequencyData(frequencyData);

    // Speech band (~150 Hz – 3 kHz) — ignores silence and non-voice noise better than raw RMS
    const startBin = 1;
    const endBin = Math.min(20, frequencyData.length - 1);
    if (endBin <= startBin) return 0;

    let sum = 0;
    for (let i = startBin; i <= endBin; i++) {
      sum += frequencyData[i];
    }

    return sum / (endBin - startBin + 1) / 255;
  }

  // Initialize audio pipeline
  async function initAudioPipeline() {
    try {
      // Request microphone access
      mediaStream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      // Reuse AudioContext if it exists (faster), or create new one
      if (!audioContext || audioContext.state === "closed") {
        audioContext = new AudioContext();
      }

      analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.65;

      const source = audioContext.createMediaStreamSource(mediaStream);
      source.connect(analyser);

      const supportedMimeTypes = [
        "audio/mp4;codecs=mp4a.40.2",
        "audio/mp4",
        "audio/webm;codecs=pcm",
        "audio/ogg;codecs=vorbis",
        "audio/webm;codecs=opus",
        "audio/ogg;codecs=opus",
      ];
      const mimeType =
        supportedMimeTypes.find((t) => MediaRecorder.isTypeSupported(t)) ||
        "audio/webm;codecs=opus";

      mediaRecorder = new MediaRecorder(mediaStream, { mimeType });
      audioChunks = [];

      mediaRecorder.ondataavailable = (e) => {
        if (e.data.size > 0) audioChunks.push(e.data);
      };

    } catch (error) {
      console.error("Failed to initialize audio:", error);
    }
  }

  onMount(async () => {
    teardownOverlayListeners();

    await refreshSoundEffectsSetting();

    // Preload all sounds in parallel and wait for them to be ready
    try {
      [startSound, endSound, loadingSound] = await Promise.all([
        preloadSound("/start.mp3"),
        preloadSound("/end.mp3"),
        preloadSound("/loading.mp3"),
      ]);
      soundsReady = true;
    } catch (e) {
      console.warn("Failed to preload sounds:", e);
    }

    // Check accessibility before overlay — guide mode needs a small centered window
    let hasPermission = true;
    try {
      hasPermission = await invoke<boolean>(
        "check_accessibility_permission",
        { prompt: false },
      );
    } catch (e) {
      console.error("Failed to check accessibility permission:", e);
    }

    if (!hasPermission) {
      needsPermission = true;
      await invoke("set_guide_mode", { enable: true });
    } else {
      await invoke("show_overlay");
    }

    // DON'T initialize audio here - only when first recording starts
    // This prevents the microphone indicator from showing immediately

    unlistenPressed = await listen("hotkey-pressed", () => {
      if (state === "idle" && !isCapturingHotkey) startRecording();
    });
    overlayListenerCleanups.push(unlistenPressed);

    unlistenReleased = await listen("hotkey-released", (event: any) => {
      const now = Date.now();
      if (now - lastHotkeyReleaseAt < 400) return;
      lastHotkeyReleaseAt = now;

      releaseTimestamp = event.payload as number;
      if (state === "recording") stopRecording();
    });
    overlayListenerCleanups.push(unlistenReleased);
    // Removed hover resize logic as window is now fullscreen
    unlistenHover = await listen("hover-changed", (event: any) => {
        isHovered = event.payload as boolean;
    });
    overlayListenerCleanups.push(unlistenHover);

    unlistenOverlayClick = await listen("overlay-clicked", () => {
      if (state === "idle" && !isCapturingHotkey) startRecording();
    });
    overlayListenerCleanups.push(unlistenOverlayClick);

    // Listen for hotkey capture state from dashboard
    unlistenCaptureStart = await listen("hotkey-capture-started", () => {
      isCapturingHotkey = true;
    });
    overlayListenerCleanups.push(unlistenCaptureStart);

    unlistenCaptureEnd = await listen("hotkey-capture-ended", () => {
      isCapturingHotkey = false;
    });
    overlayListenerCleanups.push(unlistenCaptureEnd);

    unlistenParakeetInstall = await listen<{
      message: string;
    }>("parakeet-install-progress", (event) => {
      installStatusMessage = event.payload.message;
    });
    overlayListenerCleanups.push(unlistenParakeetInstall);

    unlistenSettingsUpdated = await listen<{
      sound_effects_enabled: boolean;
    }>("settings-updated", (event) => {
      soundEffectsEnabled = event.payload.sound_effects_enabled;
    });
    overlayListenerCleanups.push(unlistenSettingsUpdated);

    checkAndShowShortcutHint();
  });

  onDestroy(() => {
    teardownOverlayListeners();
    unlistenPressed = null;
    unlistenReleased = null;
    unlistenHover = null;
    unlistenOverlayClick = null;
    unlistenCaptureStart = null;
    unlistenCaptureEnd = null;
    unlistenParakeetInstall = null;
    unlistenSettingsUpdated = null;

    // Properly close audio resources on unmount
    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
    if (mediaRecorder) {
      mediaRecorder.onstop = null;
      mediaRecorder = null;
    }
    mediaStream?.getTracks().forEach((t) => t.stop());
    mediaStream = null;
    audioContext?.close();
    audioContext = null;
    analyser = null;
  });

  async function startRecording() {
    if (state !== "idle" || isProcessingRecording) return;

    state = "recording";
    await refreshSoundEffectsSetting();

    // STEP 1: Initialize audio pipeline FIRST (if needed)
    if (!audioContext || !mediaStream || !mediaRecorder) {
      await initAudioPipeline();

      // If initialization failed, abort
      if (!audioContext || !mediaStream || !mediaRecorder) {
        console.error("Failed to initialize audio");
        state = "idle";
        return;
      }
    }

    // STEP 2: Ensure AudioContext is active
    if (audioContext?.state === "suspended") {
      await audioContext.resume();
    }

    // STEP 2.5: Play start sound
    playSound(startSound);

    // STEP 3: START RECORDING IMMEDIATELY (critical for zero latency)
    audioChunks = [];
    recordingStartTime = Date.now(); // Track duration
    if (mediaRecorder.state === "inactive") {
      mediaRecorder.start(100); // Collect chunks every 100ms
    }

    // Start duration check timer
    checkDuration();

    // STEP 3.5: Small delay to ensure audio buffer is capturing
    await new Promise((resolve) => setTimeout(resolve, 50));

    if (state !== "recording") return;

    // STEP 4: Update UI (sequential - after recording started)
    await invoke("resize_overlay", {
      recording: true,
      width: config.recording.width,
      height: config.recording.height,
    });

    // STEP 5: Start visualization
    visualize();
  }

  function visualize() {
    if (state !== "recording" || !analyser) {
      barHeights = new Array(config.barCount).fill(config.barMinHeight);
      return;
    }

    const voiceLevel = measureVoiceActivity();
    const isSpeaking = voiceLevel >= config.voiceActivityThreshold;

    const barCount = config.barCount;
    if (nextHeights.length !== barCount) {
      nextHeights = new Array(barCount).fill(config.barMinHeight);
    }

    const range = config.barMaxHeight - config.barMinHeight;

    for (let i = 0; i < barCount; i++) {
      const currentHeight = barHeights[i] || config.barMinHeight;

      if (isSpeaking) {
        const voiceBoost = Math.min(1, voiceLevel / config.voiceActivityThreshold);
        const maxForFrame =
          config.barMinHeight + range * (0.55 + Math.random() * 0.45 * voiceBoost);
        const targetHeight =
          config.barMinHeight + Math.random() * (maxForFrame - config.barMinHeight);
        const speed = 0.4 + Math.random() * 0.5;
        nextHeights[i] = currentHeight + (targetHeight - currentHeight) * speed;
      } else {
        nextHeights[i] =
          currentHeight + (config.barMinHeight - currentHeight) * 0.3;
      }
    }

    const prevHeights = barHeights;
    barHeights = nextHeights;
    nextHeights = prevHeights;
    animationFrameId = requestAnimationFrame(visualize);
  }

  async function stopRecording() {
    if (state !== "recording" || !mediaRecorder || isStoppingRecording) return;

    isStoppingRecording = true;
    state = "processing";
    installStatusMessage = "";
    await refreshSoundEffectsSetting();

    // Play loading sound during processing
    playSound(loadingSound);

    const recorder = mediaRecorder;
    let processingStarted = false;
    const startProcessingOnce = () => {
      if (processingStarted) return;
      processingStarted = true;
      void processRecording();
    };

    recorder.onstop = () => {
      recorder.onstop = null;
      startProcessingOnce();
    };

    try {
      if (recorder.state === "recording") {
        recorder.stop();
      } else {
        startProcessingOnce();
      }
    } catch (error) {
      console.error("Failed to stop recorder:", error);
      startProcessingOnce();
    }
  }

  async function processRecording() {
    if (isProcessingRecording) return;
    isProcessingRecording = true;

    const capturedChunks = audioChunks;
    audioChunks = [];

    try {
      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
      }
      barHeights = new Array(config.barCount).fill(config.barMinHeight);

      if (capturedChunks.length === 0) {
        return;
      }

      const audioBlob = new Blob(capturedChunks, {
        type: capturedChunks[0].type,
      });

      // Always send WAV to backend for stable local decoding.
      const audioData = await convertRecordingToWav(audioBlob);

      await invoke<string>("process_audio_with_history", {
        audioData,
        normalize: true,
        releaseTimestamp,
      });

      await refreshSoundEffectsSetting();
      playSound(endSound);
    } catch (error) {
      console.error("Processing error:", error);
    } finally {
      isProcessingRecording = false;
      isStoppingRecording = false;
      cleanup();
    }
  }

  function cleanup() {
    // Reset release timestamp
    releaseTimestamp = null;

    // FIRST: Reset state to idle immediately (fixes UI lag)
    state = "idle";
    barHeights = new Array(config.barCount).fill(config.barMinHeight);

    // Reset visualization
    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }

    // Detach onstop handler BEFORE stopping to prevent re-triggering processRecording
    if (mediaRecorder) {
      mediaRecorder.onstop = null;
      if (mediaRecorder.state !== "inactive") {
        mediaRecorder.stop();
      }
      mediaRecorder = null;
    }

    // Stop media stream tracks
    if (mediaStream) {
      mediaStream.getTracks().forEach((track) => track.stop());
      mediaStream = null;
    }

    if (analyser) {
      analyser.disconnect();
      analyser = null;
    }

    audioChunks = [];

    // Resize back to idle after animation
    setTimeout(() => {
      if (state === "idle" && !isHovered) {
        invoke("resize_overlay", {
          recording: false,
          width: config.idle.width,
          height: config.idle.height,
        }).catch(() => {});
      }
    }, 400);
  }

  function handleClick() {
    if (state === "idle") startRecording();
    else if (state === "recording") stopRecording();
  }

  async function handleRequestPermission() {
    await invoke("check_accessibility_permission", { prompt: true });
    startPolling();
  }

  function startPolling() {
    const interval = setInterval(async () => {
      const has = await invoke<boolean>("check_accessibility_permission", {
        prompt: false,
      });
      if (has) {
        clearInterval(interval);
        closeGuide();
      }
    }, 1000);
  }

  async function closeGuide() {
    needsPermission = false;
    // Reset window to normal overlay mode
    await invoke("set_guide_mode", { enable: false });
    // Force resize removed - window is fullscreen
  }

  function checkAndShowShortcutHint() {
    const storageKey = "speech_clip_oss_shortcut_hint_seen";
    if (!localStorage.getItem(storageKey)) {
      showShortcutHint = true;
    }
  }

  function dismissShortcutHint() {
    showShortcutHint = false;
    localStorage.setItem("speech_clip_oss_shortcut_hint_seen", "true");
  }
</script>

<main class="w-full h-full flex {needsPermission ? '' : 'items-end justify-center'}">
  {#if needsPermission}
    <PermissionGuide
      onRequestPermission={handleRequestPermission}
      onClose={closeGuide}
    />
  {:else}
    <!-- Shortcut Hint - positioned independently above mini-bar -->
    {#if showShortcutHint && state === "idle"}
      <ShortcutHint hotkey={currentHotkey} onDismiss={dismissShortcutHint} />
    {/if}

    <!-- Centered Bottom Anchor -->
    <div class="fixed bottom-8 left-1/2 -translate-x-1/2 z-50 flex items-center justify-center">
      <div
        role="button"
        tabindex="0"
        onclick={handleClick}
        onkeydown={(e) => (e.key === "Enter" || e.key === " ") && handleClick()}
        class="liquid-bar"
        class:dictating={state !== "idle"}
        class:hovered={isHovered}
      >
        {#if state === "idle"}
          <div class="idle-content">
            <!-- Empty pill in idle -->
          </div>
        {:else}
          <div class="dictation-content" transition:fade={{ duration: 200 }}>
            <!-- Cancel Button (Left) -->
            <button
              class="action-btn cancel"
              onclick={(e) => {
                e.stopPropagation();
                cleanup();
              }}
              aria-label="Cancel"
              disabled={state === "processing"}
            >
              <svg
                width="6"
                height="6"
                viewBox="0 0 12 12"
                fill="none"
                xmlns="http://www.w3.org/2000/svg"
              >
                <path
                  d="M1 1L11 11M1 11L11 1"
                  stroke="currentColor"
                  stroke-width="2.5"
                  stroke-linecap="round"
                />
              </svg>
            </button>

            <!-- Waveform (Center) -->
            <div class="waveform">
              {#if state === "processing"}
                <div class="processing-indicator">
                  <div
                    class="w-3 h-3 border-2 border-white/40 border-t-white rounded-full animate-spin"
                  ></div>
                  {#if installStatusMessage}
                    <span class="install-hint">{installStatusMessage}</span>
                  {/if}
                </div>
              {:else}
                {#each barHeights as height}
                  <div class="bar" style="height: {height}px"></div>
                {/each}
              {/if}
            </div>

            <!-- Stop Button (Right) -->
            <button
              class="action-btn stop"
              onclick={(e) => {
                e.stopPropagation();
                stopRecording();
              }}
              aria-label="Stop Recording"
              disabled={state === "processing"}
            >
              <div
                style="width: 6px; height: 6px; background: currentColor; border-radius: 1px;"
              ></div>
            </button>
          </div>
        {/if}

        <!-- Shine effect -->
        <div class="shine"></div>
      </div>
    </div>
  {/if}
</main>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    overflow: hidden;
    overscroll-behavior: none;
  }
  main {
    width: 100%;
    height: 100%;
    min-height: 100%;
    user-select: none;
    -webkit-app-region: drag;
    background: transparent;
  }
  button,
  div[role="button"] {
    -webkit-app-region: no-drag;
    cursor: pointer;
  }

  /* --- CERAMIC MATTE COMPONENT (Braun/Teenage Engineering Style) --- */

  .liquid-bar {
    position: relative;
    z-index: 10;

    /* Config: Idle Dimensions (SAME) */
    width: 40px;
    height: 10px;

    /* Aesthetic: Solid, Matte, Industrial */
    background: #080808;

    /* No blur - solid material */
    backdrop-filter: none;
    -webkit-backdrop-filter: none;

    border-radius: 999px;

    /* Crisp, physical border */
    border: 1px solid rgba(255, 255, 255, 0.12);

    /* No shadow */
    box-shadow: none;

    transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1);
    transform-origin: bottom;

    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  /* Hover State (Idle) */
  .liquid-bar.hovered:not(.dictating) {
    width: 50px;
    height: 14px;
    background: #141414;
    border-color: rgba(255, 255, 255, 0.2);
    box-shadow: none;
  }

  /* Dictating State (Active - 110x24) */
  .liquid-bar.dictating {
    width: 110px;
    height: 24px;

    /* Active: Deep solid surface */
    background: #111111;

    /* Precision Border */
    border: 1px solid rgba(255, 255, 255, 0.15);

    /* No shadow */
    box-shadow: none;
  }

  /* Content */
  .idle-content {
    width: 24px;
    height: 4px; /* A thin slot/groove */
    background: rgba(255, 255, 255, 0.2);
    border-radius: 2px;
  }

  .dictation-content {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    height: 100%;
    padding: 0 8px; /* Slightly more breathing room */
    box-sizing: border-box;
    gap: 6px;
  }

  .waveform {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 3px; /* Distinct separation */
    height: 100%;
    opacity: 0.9;
  }

  /* Buttons */
  .action-btn {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: #666;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
    padding: 0;
  }

  .action-btn:hover {
    color: #fff;
    background: #333;
    transform: scale(1.1);
  }

  .action-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
    transform: none !important;
  }

  /* Colors inspired by Braun buttons */
  .action-btn.cancel:hover {
    color: #ff3b30; /* Safety Red */
    background: rgba(255, 59, 48, 0.1);
  }

  .action-btn.stop {
    color: #999;
  }
  .action-btn.stop:hover {
    color: #ff9500; /* Signal Orange */
    background: rgba(255, 149, 0, 0.1);
  }

  .bar {
    width: 2px;
    /* Solid Orange Accent - The signature look */
    background: #ff4f00; /* International Orange */
    border-radius: 1px;
    transition: height 0.08s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: none; /* No glow for clean look */
  }

  /* Remove Shine */
  .shine {
    display: none;
  }

  /* Remove Hover Shine Trigger */
  .liquid-bar:hover .shine {
    /* animation: none; */
  }

  /* Loading spinner for processing state */
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .animate-spin {
    animation: spin 1s linear infinite;
  }

  .processing-indicator {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    max-width: 140px;
  }

  .install-hint {
    font-size: 9px;
    line-height: 1.2;
    color: rgba(255, 255, 255, 0.55);
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
</style>
