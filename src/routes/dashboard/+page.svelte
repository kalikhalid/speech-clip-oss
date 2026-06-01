<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  type ParakeetStatus = {
    model_id: string;
    model_dir: string;
    model_downloaded: boolean;
    ready: boolean;
    message: string;
    install_stage: string;
    install_in_progress: boolean;
  };

  type InstallProgress = {
    stage: string;
    message: string;
    percent: number;
  };

  type HistoryEntry = {
    id: string;
    timestamp: number;
    normalized_text: string;
    app_name: string | null;
    duration_ms: number;
    engine?: string;
    local_model?: string | null;
  };

  type AppSettings = {
    language: string;
    hotkey: string;
    parakeet_model: string;
  };

  type SetupStage = {
    id: string;
    label: string;
  };

  const SETUP_STAGES: SetupStage[] = [
    { id: "download", label: "Download model" },
    { id: "verify", label: "Verify" },
    { id: "extract", label: "Extract" },
    { id: "install", label: "Install" },
    { id: "ready", label: "Ready" },
  ];

  let tab: "overview" | "settings" = $state("overview");
  let parakeetStatus = $state<ParakeetStatus | null>(null);
  let statusLoading = $state(true);
  let installing = $state(false);
  let installError = $state("");
  let showErrorDetails = $state(false);
  let showTechDetails = $state(false);
  let installProgress = $state<InstallProgress | null>(null);
  let historyEntries = $state<HistoryEntry[]>([]);
  let settings = $state<AppSettings>({
    language: "auto",
    hotkey: "control+`",
    parakeet_model: "parakeet-tdt-0.6b-v3",
  });
  let hotkeyInput = $state("control+`");
  let hotkeyError = $state("");
  let showHotkeyErrorDetails = $state(false);
  let saveMessage = $state("");

  let unlistenProgress: (() => void) | null = null;

  type HeroState = "loading" | "ready" | "setting-up" | "needs-setup" | "error";

  function heroState(): HeroState {
    if (installError) return "error";
    if (statusLoading && !installing && !parakeetStatus) return "loading";
    if (installing || parakeetStatus?.install_in_progress) return "setting-up";
    if (parakeetStatus?.ready) return "ready";
    if (parakeetStatus) return "needs-setup";
    return "loading";
  }

  function friendlyInstallError(raw: string): { title: string; hint: string } {
    const lower = raw.toLowerCase();
    if (lower.includes("404") && lower.includes("download")) {
      return {
        title: "Couldn't download Parakeet model",
        hint: "Check your internet connection and try again from the dashboard.",
      };
    }
    if (lower.includes("checksum")) {
      return {
        title: "Model download was corrupted",
        hint: "Retry setup to download Parakeet v3 again.",
      };
    }
    const firstLine = raw.split("\n")[0]?.replace(/https?:\/\/\S+/g, "").trim();
    return {
      title: firstLine && firstLine.length < 100 ? firstLine : "Setup didn't finish",
      hint: "Open Details below for technical information, or retry setup.",
    };
  }

  function stageIndex(stage: string | undefined): number {
    if (!stage) return -1;
    const order = ["start", "download", "verify", "extract", "install", "ready", "failed"];
    const idx = order.indexOf(stage);
    if (idx >= 0) return idx;
    if (stage === "installing") return 2;
    return 0;
  }

  function isStageComplete(stageId: string): boolean {
    const current = installProgress?.stage ?? parakeetStatus?.install_stage;
    const cur = stageIndex(current);
    const target = stageIndex(stageId);
    if (target < 0 || cur < 0) {
      if (stageId === "download") return parakeetStatus?.model_downloaded ?? false;
      return parakeetStatus?.ready ?? false;
    }
    return cur > target;
  }

  function isStageActive(stageId: string): boolean {
    const current = installProgress?.stage ?? "";
    return current === stageId || (installing && current === stageId);
  }

  function relativeTime(ts: number): string {
    const diff = Date.now() - ts;
    const sec = Math.floor(diff / 1000);
    if (sec < 60) return "Just now";
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min}m ago`;
    const hr = Math.floor(min / 60);
    if (hr < 24) return `${hr}h ago`;
    const day = Math.floor(hr / 24);
    if (day < 7) return `${day}d ago`;
    return new Date(ts).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  }

  function formatDuration(ms: number): string {
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    return `${Math.floor(s / 60)}m ${s % 60}s`;
  }

  function shortModel(id: string): string {
    const parts = id.split("/");
    return parts.length > 1 ? parts[parts.length - 1] : id;
  }

  async function refreshStatus() {
    statusLoading = true;
    installError = "";
    showErrorDetails = false;
    try {
      parakeetStatus = await invoke<ParakeetStatus>("get_parakeet_status");
    } catch (e) {
      parakeetStatus = null;
      installError = String(e);
      console.error(e);
    } finally {
      statusLoading = false;
    }
  }

  async function runInstall() {
    if (installing) return;
    installing = true;
    installError = "";
    showErrorDetails = false;
    installProgress = { stage: "start", message: "Starting setup…", percent: 0 };
    try {
      parakeetStatus = await invoke<ParakeetStatus>("ensure_parakeet_runtime");
      installProgress = { stage: "ready", message: "Ready", percent: 100 };
    } catch (e) {
      installError = String(e);
      await refreshStatus();
    } finally {
      installing = false;
    }
  }

  async function loadSettings() {
    const s = await invoke<AppSettings>("get_settings");
    settings = s;
    hotkeyInput = s.hotkey;
  }

  async function loadHistory() {
    const h: { entries: HistoryEntry[] } = await invoke("get_history");
    historyEntries = h.entries ?? [];
  }

  async function saveSettings() {
    hotkeyError = "";
    showHotkeyErrorDetails = false;
    saveMessage = "";
    try {
      if (hotkeyInput !== settings.hotkey) {
        await invoke("update_hotkey", { hotkey: hotkeyInput });
        settings.hotkey = hotkeyInput;
      }
      await invoke("save_settings", { newSettings: settings });
      saveMessage = "Settings saved";
      await refreshStatus();
    } catch (e) {
      hotkeyError = String(e);
    }
  }

  async function deleteEntry(id: string) {
    await invoke("delete_history_entry", { entryId: id });
    await loadHistory();
  }

  async function clearHistory() {
    await invoke("clear_all_history");
    await loadHistory();
  }

  onMount(async () => {
    unlistenProgress = await listen<InstallProgress>(
      "parakeet-install-progress",
      (event) => {
        installProgress = event.payload;
        if (event.payload.stage === "failed") {
          installError = event.payload.message;
        }
      },
    );

    await loadSettings();
    await loadHistory();
    await refreshStatus();

    if (parakeetStatus && !parakeetStatus.ready) {
      void runInstall();
    }
  });

  onDestroy(() => {
    unlistenProgress?.();
  });

  const hero = $derived(heroState());
  const errorInfo = $derived(
    installError ? friendlyInstallError(installError) : null,
  );
  const progressPercent = $derived(installProgress?.percent ?? 0);
  const showSetupProgress = $derived(
    installing || installProgress !== null || hero === "setting-up",
  );
</script>

<div
  class="dashboard-shell h-dvh overflow-y-auto overscroll-y-contain bg-[#0a0a0c] text-[#e8e8ed] font-sans antialiased selection:bg-[#ff4f00]/30"
>
  <div class="mx-auto max-w-2xl px-6 py-8 pb-16">
    <header class="mb-8">
      <div class="flex items-start justify-between gap-4">
        <div class="flex min-w-0 items-start gap-3">
          <img
            src="/logo.png"
            alt=""
            width="36"
            height="36"
            class="mt-0.5 size-9 shrink-0 rounded-[22%]"
          />
          <div class="min-w-0">
            <h1 class="text-xl font-semibold tracking-tight text-white">
              Speech Clip
            </h1>
            <p class="mt-1 text-sm text-[#8a8a96]">
              Local dictation on your Mac — no account required
            </p>
          </div>
        </div>
        <span
          class="rounded-full border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider text-[#8a8a96]"
        >
          OSS
        </span>
      </div>

      <nav
        class="mt-6 inline-flex rounded-lg border border-white/10 bg-[#111114] p-1"
        aria-label="Dashboard sections"
      >
        <button
          type="button"
          class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#ff4f00]/60 {tab ===
          'overview'
            ? 'bg-[#1a1a1f] text-white shadow-sm'
            : 'text-[#8a8a96] hover:text-[#c8c8d0]'}"
          onclick={() => (tab = "overview")}
        >
          Overview
        </button>
        <button
          type="button"
          class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[#ff4f00]/60 {tab ===
          'settings'
            ? 'bg-[#1a1a1f] text-white shadow-sm'
            : 'text-[#8a8a96] hover:text-[#c8c8d0]'}"
          onclick={() => (tab = "settings")}
        >
          Settings
        </button>
      </nav>
    </header>

    {#if tab === "overview"}
      <!-- Hero status -->
      <section
        class="relative mb-6 overflow-hidden rounded-2xl border border-white/10 bg-gradient-to-b from-[#141418] to-[#0e0e11] p-6"
        aria-live="polite"
      >
        <div
          class="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-[#ff4f00]/[0.07] blur-2xl"
        ></div>

        <div class="flex items-start gap-4">
          {#if hero === "loading"}
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.04]"
              aria-hidden="true"
            >
              <div
                class="h-5 w-5 animate-spin rounded-full border-2 border-white/20 border-t-[#ff4f00]"
              ></div>
            </div>
            <div>
              <h2 class="text-lg font-semibold text-white">Checking setup…</h2>
              <p class="mt-1 text-sm text-[#8a8a96]">
                Preparing local speech recognition
              </p>
            </div>
          {:else if hero === "ready"}
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-emerald-500/30 bg-emerald-500/10"
              aria-hidden="true"
            >
              <svg
                class="h-6 w-6 text-emerald-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="2"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <div class="min-w-0 flex-1">
              <h2 class="text-lg font-semibold text-white">Ready to dictate</h2>
              <p class="mt-1 text-sm text-[#8a8a96]">
                Hold <kbd
                  class="rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[#c8c8d0]"
                  >{settings.hotkey}</kbd
                > and speak in any app
              </p>
            </div>
          {:else if hero === "setting-up"}
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-[#ff4f00]/30 bg-[#ff4f00]/10"
              aria-hidden="true"
            >
              <div
                class="h-5 w-5 animate-spin rounded-full border-2 border-[#ff4f00]/30 border-t-[#ff4f00]"
              ></div>
            </div>
            <div class="min-w-0 flex-1">
              <h2 class="text-lg font-semibold text-white">Setting up…</h2>
              <p class="mt-1 text-sm text-[#8a8a96]">
                {installProgress?.message ??
                  parakeetStatus?.message ??
                  "One-time download — this may take a few minutes"}
              </p>
            </div>
          {:else if hero === "error"}
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-red-500/30 bg-red-500/10"
              aria-hidden="true"
            >
              <svg
                class="h-6 w-6 text-red-400"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="2"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M12 9v2m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div class="min-w-0 flex-1">
              <h2 class="text-lg font-semibold text-white">
                {errorInfo?.title ?? "Setup failed"}
              </h2>
              <p class="mt-1 text-sm text-[#8a8a96]">
                {errorInfo?.hint ?? "Try running setup again."}
              </p>
            </div>
          {:else}
            <div
              class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.04]"
              aria-hidden="true"
            >
              <svg
                class="h-6 w-6 text-[#8a8a96]"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="1.5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                />
              </svg>
            </div>
            <div class="min-w-0 flex-1">
              <h2 class="text-lg font-semibold text-white">Finish setup</h2>
              <p class="mt-1 text-sm text-[#8a8a96]">
                {parakeetStatus?.message ??
                  "Download speech components to start dictating"}
              </p>
            </div>
          {/if}
        </div>

        {#if showSetupProgress}
          <div class="mt-5">
            <div
              class="h-1.5 overflow-hidden rounded-full bg-white/10"
              role="progressbar"
              aria-valuenow={progressPercent}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <div
                class="h-full rounded-full bg-gradient-to-r from-[#ff4f00] to-[#ff7a33] transition-[width] duration-300 ease-out"
                style="width: {Math.max(progressPercent, hero === 'setting-up' ? 4 : 0)}%"
              ></div>
            </div>

            <ol class="mt-4 grid gap-2 sm:grid-cols-2">
              {#each SETUP_STAGES as stage}
                {@const done = isStageComplete(stage.id)}
                {@const active = isStageActive(stage.id)}
                <li
                  class="flex items-center gap-2.5 rounded-lg border px-3 py-2 text-sm {done
                    ? 'border-emerald-500/20 bg-emerald-500/5 text-emerald-300/90'
                    : active
                      ? 'border-[#ff4f00]/30 bg-[#ff4f00]/5 text-[#ffb899]'
                      : 'border-white/5 bg-white/[0.02] text-[#6a6a76]'}"
                >
                  {#if done}
                    <span class="text-emerald-400" aria-hidden="true">✓</span>
                  {:else if active}
                    <span
                      class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-[#ff4f00]/30 border-t-[#ff4f00]"
                      aria-hidden="true"
                    ></span>
                  {:else}
                    <span
                      class="inline-block h-2 w-2 rounded-full bg-white/15"
                      aria-hidden="true"
                    ></span>
                  {/if}
                  <span>{stage.label}</span>
                </li>
              {/each}
            </ol>
          </div>
        {/if}

        {#if installError}
          <div class="mt-4 rounded-lg border border-red-500/20 bg-red-500/5 p-3">
            <button
              type="button"
              class="text-xs font-medium text-red-300/90 underline-offset-2 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/50"
              onclick={() => (showErrorDetails = !showErrorDetails)}
              aria-expanded={showErrorDetails}
            >
              {showErrorDetails ? "Hide details" : "Show details"}
            </button>
            {#if showErrorDetails}
              <pre
                class="mt-2 max-h-32 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-red-200/70">{installError}</pre>
            {/if}
          </div>
        {/if}

        <div class="mt-5 flex flex-wrap gap-2">
          {#if hero !== "ready" && hero !== "loading"}
            <button
              type="button"
              class="rounded-lg bg-[#ff4f00] px-4 py-2 text-sm font-medium text-white transition hover:bg-[#e64800] focus:outline-none focus-visible:ring-2 focus-visible:ring-[#ff4f00]/60 disabled:cursor-not-allowed disabled:opacity-50"
              onclick={runInstall}
              disabled={installing}
            >
              {installing ? "Installing…" : "Run setup"}
            </button>
          {/if}
          <button
            type="button"
            class="rounded-lg border border-white/15 bg-transparent px-4 py-2 text-sm font-medium text-[#a8a8b4] transition hover:border-white/25 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30 disabled:opacity-50"
            onclick={refreshStatus}
            disabled={installing}
          >
            Refresh
          </button>
        </div>
      </section>

      <!-- Compact component status -->
      {#if parakeetStatus && !statusLoading}
        <section
          class="mb-6 rounded-xl border border-white/10 bg-[#111114] p-4"
        >
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-xs font-medium uppercase tracking-wider text-[#6a6a76]">
              Components
            </h3>
            <button
              type="button"
              class="text-xs text-[#6a6a76] underline-offset-2 hover:text-[#a8a8b4] focus:outline-none focus-visible:ring-2 focus-visible:ring-white/20"
              onclick={() => (showTechDetails = !showTechDetails)}
              aria-expanded={showTechDetails}
            >
              {showTechDetails ? "Less" : "Details"}
            </button>
          </div>

          <ul class="mt-3 space-y-2">
            <li class="flex items-center justify-between text-sm">
              <span class="text-[#a8a8b4]">Parakeet v3 (ONNX)</span>
              <span
                class="rounded-full px-2 py-0.5 text-xs font-medium {parakeetStatus.model_downloaded
                  ? 'bg-emerald-500/15 text-emerald-400'
                  : 'bg-white/5 text-[#8a8a96]'}"
              >
                {parakeetStatus.model_downloaded ? "Installed" : "Pending"}
              </span>
            </li>
            <li class="flex items-center justify-between text-sm">
              <span class="text-[#a8a8b4]">Engine</span>
              <span class="font-mono text-xs text-[#c8c8d0]">transcribe-rs</span>
            </li>
            <li class="flex items-center justify-between text-sm">
              <span class="text-[#a8a8b4]">Model</span>
              <span
                class="max-w-[12rem] truncate font-mono text-xs text-[#c8c8d0]"
                title={parakeetStatus.model_id}
              >
                {shortModel(parakeetStatus.model_id)}
              </span>
            </li>
          </ul>

          {#if showTechDetails}
            <dl
              class="mt-3 space-y-2 border-t border-white/5 pt-3 font-mono text-[11px] text-[#6a6a76]"
            >
              <div>
                <dt class="text-[#5a5a66]">Model directory</dt>
                <dd class="mt-0.5 break-all text-[#8a8a96]">
                  {parakeetStatus.model_dir}
                </dd>
              </div>
            </dl>
          {/if}
        </section>
      {/if}

      <!-- History -->
      <section class="rounded-xl border border-white/10 bg-[#111114] p-5">
        <div class="flex items-center justify-between gap-3">
          <h2 class="text-sm font-semibold text-white">History</h2>
          {#if historyEntries.length > 0}
            <button
              type="button"
              class="text-xs text-[#6a6a76] transition hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
              onclick={clearHistory}
            >
              Clear all
            </button>
          {/if}
        </div>

        {#if historyEntries.length === 0}
          <div
            class="mt-8 flex flex-col items-center py-6 text-center"
          >
            <div
              class="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl border border-white/10 bg-white/[0.03]"
              aria-hidden="true"
            >
              <svg
                class="h-7 w-7 text-[#4a4a56]"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                stroke-width="1.5"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M19 11a7 7 0 01-7 7m0 0a7 7 0 01-7-7m7 7v4m0 0H8m4 0h4m-4-8a3 3 0 01-3-3V5a3 3 0 116 0v6a3 3 0 01-3 3z"
                />
              </svg>
            </div>
            <p class="text-sm font-medium text-[#a8a8b4]">No transcriptions yet</p>
            <p class="mt-1 max-w-xs text-xs text-[#6a6a76]">
              Hold your hotkey anywhere on your Mac and speak — text appears in
              the focused field.
            </p>
          </div>
        {:else}
          <ul class="mt-4 space-y-3">
            {#each historyEntries as entry (entry.id)}
              <li
                class="group rounded-xl border border-white/8 bg-[#0c0c0f] p-4 transition hover:border-white/15"
              >
                <div class="flex items-start justify-between gap-3">
                  <div class="flex flex-wrap items-center gap-2 text-xs text-[#6a6a76]">
                    <time datetime={new Date(entry.timestamp).toISOString()}>
                      {relativeTime(entry.timestamp)}
                    </time>
                    {#if entry.app_name}
                      <span
                        class="rounded-md border border-white/10 bg-white/[0.03] px-1.5 py-0.5 text-[#8a8a96]"
                      >
                        {entry.app_name}
                      </span>
                    {/if}
                    <span>{formatDuration(entry.duration_ms)}</span>
                  </div>
                  <button
                    type="button"
                    class="shrink-0 rounded-md px-2 py-1 text-xs text-[#6a6a76] opacity-0 transition group-hover:opacity-100 hover:bg-white/5 hover:text-red-400 focus:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                    onclick={() => deleteEntry(entry.id)}
                    aria-label="Delete transcription"
                  >
                    Delete
                  </button>
                </div>
                <p class="mt-2 text-sm leading-relaxed text-[#d8d8e0]">
                  {entry.normalized_text}
                </p>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {:else}
      <!-- Settings -->
      <form
        class="space-y-6"
        onsubmit={(e) => {
          e.preventDefault();
          saveSettings();
        }}
      >
        <section class="rounded-xl border border-white/10 bg-[#111114] p-5">
          <h2 class="text-sm font-semibold text-white">Dictation hotkey</h2>
          <p class="mt-1 text-xs text-[#6a6a76]">
            Hold this combination to record. Release to transcribe and paste.
          </p>
          <input
            type="text"
            bind:value={hotkeyInput}
            placeholder="control+`"
            class="mt-3 w-full rounded-lg border border-white/10 bg-[#0a0a0c] px-3 py-2.5 font-mono text-sm text-white placeholder:text-[#4a4a56] focus:border-[#ff4f00]/50 focus:outline-none focus:ring-2 focus:ring-[#ff4f00]/20"
            aria-describedby="hotkey-help"
          />
          <p id="hotkey-help" class="mt-2 text-xs text-[#6a6a76]">
            Examples: <code class="text-[#8a8a96]">control+`</code>,
            <code class="text-[#8a8a96]">command+shift+d</code>
          </p>
          {#if hotkeyError}
            <div
              class="mt-3 rounded-lg border border-red-500/20 bg-red-500/5 p-3"
            >
              <p class="text-sm text-red-300">{hotkeyError.split("\n")[0]}</p>
              <button
                type="button"
                class="mt-1 text-xs text-red-300/80 underline-offset-2 hover:underline"
                onclick={() =>
                  (showHotkeyErrorDetails = !showHotkeyErrorDetails)}
              >
                {showHotkeyErrorDetails ? "Hide details" : "Details"}
              </button>
              {#if showHotkeyErrorDetails}
                <pre
                  class="mt-2 font-mono text-[11px] text-red-200/60">{hotkeyError}</pre>
              {/if}
            </div>
          {/if}
        </section>

        <section class="rounded-xl border border-white/10 bg-[#111114] p-5">
          <h2 class="text-sm font-semibold text-white">Speech model</h2>
          <p class="mt-1 text-xs text-[#6a6a76]">
            Parakeet v3 ONNX bundle (transcribe-rs). Installed automatically on
            first use (~456 MB).
          </p>
          <label class="mt-3 block">
            <span class="sr-only">Model ID</span>
            <input
              type="text"
              bind:value={settings.parakeet_model}
              readonly
              class="w-full rounded-lg border border-white/10 bg-[#0a0a0c] px-3 py-2.5 font-mono text-sm text-[#8a8a96] focus:outline-none"
            />
          </label>
        </section>

        <div class="flex items-center gap-3">
          <button
            type="submit"
            class="rounded-lg bg-[#ff4f00] px-5 py-2.5 text-sm font-medium text-white transition hover:bg-[#e64800] focus:outline-none focus-visible:ring-2 focus-visible:ring-[#ff4f00]/60"
          >
            Save settings
          </button>
          {#if saveMessage}
            <span class="text-sm text-emerald-400" role="status"
              >{saveMessage}</span
            >
          {/if}
        </div>
      </form>
    {/if}
  </div>
</div>
