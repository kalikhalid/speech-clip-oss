<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import DashboardSidebar from "$lib/components/dashboard/DashboardSidebar.svelte";
  import DashboardStatusBar from "$lib/components/dashboard/DashboardStatusBar.svelte";
  import DashboardPageHeader from "$lib/components/dashboard/DashboardPageHeader.svelte";
  import type { DashboardSection } from "$lib/components/dashboard/types";

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

  type DictionaryEntry = {
    from: string;
    to: string;
  };

  type AppSettings = {
    language: string;
    hotkey: string;
    parakeet_model: string;
    sound_effects_enabled: boolean;
    dictionary: DictionaryEntry[];
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

  const SECTION_META: Record<
    DashboardSection,
    { title: string; subtitle: string }
  > = {
    general: {
      title: "General",
      subtitle: "Setup status, model, and quick start for local dictation",
    },
    dictionary: {
      title: "Dictionary",
      subtitle: "Map spoken phrases to exact text after transcription",
    },
    history: {
      title: "History",
      subtitle: "Recent dictations saved on this Mac",
    },
    settings: {
      title: "Settings",
      subtitle: "Hotkey, sounds, and preferences",
    },
  };

  let section = $state<DashboardSection>("general");
  let historyFilter = $state<"all" | "week">("all");
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
    sound_effects_enabled: true,
    dictionary: [],
  });
  let dictionaryFrom = $state("");
  let dictionaryTo = $state("");
  let dictionaryError = $state("");
  let hotkeyInput = $state("control+`");
  let hotkeyError = $state("");
  let showHotkeyErrorDetails = $state(false);
  let saveMessage = $state("");
  let isSaving = $state(false);

  let unlistenProgress: (() => void) | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

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

  async function saveSettings(options?: { quiet?: boolean }): Promise<boolean> {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }

    const hotkeyToSave = hotkeyInput.trim();
    if (!hotkeyToSave) return false;

    isSaving = true;
    if (!options?.quiet) {
      saveMessage = "";
    }

    try {
      if (hotkeyToSave !== settings.hotkey) {
        await invoke("update_hotkey", { hotkey: hotkeyToSave });
        settings.hotkey = hotkeyToSave;
      }
      await invoke("save_settings", { newSettings: settings });
      hotkeyError = "";
      showHotkeyErrorDetails = false;
      saveMessage = "Saved";
      return true;
    } catch (e) {
      hotkeyError = String(e);
      saveMessage = "";
      hotkeyInput = settings.hotkey;
      return false;
    } finally {
      isSaving = false;
    }
  }

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      void saveSettings({ quiet: true });
    }, 600);
  }

  async function addDictionaryEntry() {
    dictionaryError = "";
    const from = dictionaryFrom.trim();
    const to = dictionaryTo.trim();
    if (!from) {
      dictionaryError = "Enter the spoken phrase or mis-transcription.";
      return;
    }
    if (!to) {
      dictionaryError = "Enter the replacement text.";
      return;
    }
    if (
      settings.dictionary.some((e) => e.from.toLowerCase() === from.toLowerCase())
    ) {
      dictionaryError = "A rule for this phrase already exists.";
      return;
    }
    settings.dictionary = [...settings.dictionary, { from, to }];
    dictionaryFrom = "";
    dictionaryTo = "";
    await saveSettings({ quiet: true });
  }

  async function removeDictionaryEntry(index: number) {
    settings.dictionary = settings.dictionary.filter((_, i) => i !== index);
    await saveSettings({ quiet: true });
  }

  function startDictionaryFromHistory(text: string) {
    dictionaryFrom = text.trim();
    dictionaryTo = "";
    dictionaryError = "";
    section = "dictionary";
  }

  async function toggleSoundEffects() {
    const previous = settings.sound_effects_enabled;
    settings.sound_effects_enabled = !previous;
    const saved = await saveSettings();
    if (!saved) {
      settings.sound_effects_enabled = previous;
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

  function navigate(next: DashboardSection) {
    section = next;
    if (next === "history") {
      void loadHistory();
    }
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
    if (saveTimer) clearTimeout(saveTimer);
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
  const meta = $derived(SECTION_META[section]);
  const filteredHistory = $derived(
    historyFilter === "week"
      ? historyEntries.filter(
          (e) => Date.now() - e.timestamp < 7 * 24 * 60 * 60 * 1000,
        )
      : historyEntries,
  );
  const modelShortName = $derived(
    parakeetStatus
      ? shortModel(parakeetStatus.model_id)
      : shortModel(settings.parakeet_model),
  );
</script>

<div
  class="dashboard-shell flex h-dvh min-w-[720px] flex-col bg-[var(--dash-bg)] font-sans text-[var(--dash-text)] antialiased selection:bg-[var(--dash-accent)]/30"
>
  <div class="flex min-h-0 flex-1">
    <DashboardSidebar
      active={section}
      historyCount={historyEntries.length}
      onNavigate={navigate}
    />

    <main class="min-w-0 flex-1 overflow-y-auto overscroll-y-contain px-8 py-7">
      <DashboardPageHeader title={meta.title} subtitle={meta.subtitle}>
        {#snippet actions()}
          {#if section === "history"}
            <label class="flex items-center gap-2">
              <span class="sr-only">Filter history</span>
              <select
                bind:value={historyFilter}
                class="rounded-md border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-1.5 text-sm text-[var(--dash-text)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
              >
                <option value="all">All time</option>
                <option value="week">Past 7 days</option>
              </select>
            </label>
          {/if}
        {/snippet}
      </DashboardPageHeader>

      {#if section === "general"}
        <section
          class="relative mb-6 overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-6"
          aria-live="polite"
        >
          <div
            class="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-[var(--dash-brand-orange)]/[0.06] blur-2xl"
          ></div>

          <div class="flex items-start gap-4">
            {#if hero === "loading"}
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-[var(--dash-border)] bg-white/[0.04]"
                aria-hidden="true"
              >
                <div
                  class="h-5 w-5 animate-spin rounded-full border-2 border-white/20 border-t-[var(--dash-accent)]"
                ></div>
              </div>
              <div>
                <h2 class="text-lg font-semibold text-white">Checking setup…</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  Preparing local speech recognition
                </p>
              </div>
            {:else if hero === "ready"}
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-emerald-500/30 bg-emerald-500/10"
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
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  Hold <kbd
                    class="rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[var(--dash-text)]"
                    >{settings.hotkey}</kbd
                  > and speak in any app
                </p>
              </div>
            {:else if hero === "setting-up"}
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-[var(--dash-accent)]/30 bg-[var(--dash-accent-muted)]"
                aria-hidden="true"
              >
                <div
                  class="h-5 w-5 animate-spin rounded-full border-2 border-[var(--dash-accent)]/30 border-t-[var(--dash-accent)]"
                ></div>
              </div>
              <div class="min-w-0 flex-1">
                <h2 class="text-lg font-semibold text-white">Setting up…</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {installProgress?.message ??
                    parakeetStatus?.message ??
                    "One-time download — this may take a few minutes"}
                </p>
              </div>
            {:else if hero === "error"}
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-red-500/30 bg-red-500/10"
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
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {errorInfo?.hint ?? "Try running setup again."}
                </p>
              </div>
            {:else}
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg border border-[var(--dash-border)] bg-white/[0.04]"
                aria-hidden="true"
              >
                <svg
                  class="h-6 w-6 text-[var(--dash-text-muted)]"
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
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
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
                  class="h-full rounded-full bg-gradient-to-r from-[var(--dash-accent)] to-[#ff7a33] transition-[width] duration-300 ease-out"
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
                        ? 'border-[var(--dash-accent)]/30 bg-[var(--dash-accent-muted)] text-[#ffb899]'
                        : 'border-white/5 bg-white/[0.02] text-[var(--dash-text-subtle)]'}"
                  >
                    {#if done}
                      <span class="text-emerald-400" aria-hidden="true">✓</span>
                    {:else if active}
                      <span
                        class="inline-block h-3.5 w-3.5 animate-spin rounded-full border-2 border-[var(--dash-accent)]/30 border-t-[var(--dash-accent)]"
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
                class="rounded-md bg-[var(--dash-accent)] px-4 py-2 text-sm font-medium text-white transition hover:bg-[var(--dash-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/60 disabled:cursor-not-allowed disabled:opacity-50"
                onclick={runInstall}
                disabled={installing}
              >
                {installing ? "Installing…" : "Run setup"}
              </button>
            {/if}
            <button
              type="button"
              class="rounded-md border border-[var(--dash-border)] bg-transparent px-4 py-2 text-sm font-medium text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30 disabled:opacity-50"
              onclick={refreshStatus}
              disabled={installing}
            >
              Refresh
            </button>
          </div>
        </section>

        {#if parakeetStatus && !statusLoading}
          <div class="mb-6 grid grid-cols-2 gap-2 sm:grid-cols-4">
            <div
              class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-3"
            >
              <p
                class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                Model
              </p>
              <p class="mt-1 truncate text-sm font-medium text-white">
                {modelShortName}
              </p>
            </div>
            <div
              class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-3"
            >
              <p
                class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                Status
              </p>
              <p class="mt-1 text-sm font-medium {parakeetStatus.ready
                ? 'text-emerald-400'
                : 'text-[var(--dash-text-muted)]'}">
                {parakeetStatus.ready ? "Ready" : "Pending"}
              </p>
            </div>
            <div
              class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-3"
            >
              <p
                class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                Hotkey
              </p>
              <p class="mt-1 truncate font-mono text-sm text-[var(--dash-text)]">
                {settings.hotkey}
              </p>
            </div>
            <div
              class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-3"
            >
              <p
                class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                Dictionary
              </p>
              <p class="mt-1 text-sm font-medium text-[var(--dash-text)]">
                {settings.dictionary.length} rules
              </p>
            </div>
          </div>

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-4"
          >
            <div class="flex items-center justify-between gap-2">
              <h3
                class="text-xs font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                Speech model
              </h3>
              <button
                type="button"
                class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] underline-offset-2 transition hover:text-[var(--dash-text-muted)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                onclick={() => (showTechDetails = !showTechDetails)}
                aria-expanded={showTechDetails}
              >
                {showTechDetails ? "Less" : "Details"}
              </button>
            </div>

            <ul class="mt-3 space-y-2">
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">Parakeet v3 (ONNX)</span>
                <span
                  class="rounded-md px-2 py-0.5 text-xs font-medium {parakeetStatus.model_downloaded
                    ? 'bg-emerald-500/15 text-emerald-400'
                    : 'bg-white/5 text-[var(--dash-text-muted)]'}"
                >
                  {parakeetStatus.model_downloaded ? "Installed" : "Pending"}
                </span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">Runtime</span>
                <span class="text-xs font-medium {parakeetStatus.ready
                  ? 'text-emerald-400'
                  : 'text-[var(--dash-text-muted)]'}">
                  {parakeetStatus.ready ? "Ready" : "Not ready"}
                </span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">Engine</span>
                <span class="font-mono text-xs text-[var(--dash-text)]">transcribe-rs</span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">Model ID</span>
                <span
                  class="max-w-[12rem] truncate font-mono text-xs text-[var(--dash-text)]"
                  title={parakeetStatus.model_id}
                >
                  {modelShortName}
                </span>
              </li>
            </ul>

            {#if showTechDetails}
              <dl
                class="mt-3 space-y-2 border-t border-[var(--dash-border)] pt-3 font-mono text-[11px] text-[var(--dash-text-subtle)]"
              >
                <div>
                  <dt>Model directory</dt>
                  <dd class="mt-0.5 break-all text-[var(--dash-text-muted)]">
                    {parakeetStatus.model_dir}
                  </dd>
                </div>
                <div>
                  <dt>Configured model</dt>
                  <dd class="mt-0.5 break-all text-[var(--dash-text-muted)]">
                    {settings.parakeet_model}
                  </dd>
                </div>
              </dl>
            {/if}
          </section>
        {/if}
      {:else if section === "dictionary"}
        <section class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5">
          <p class="text-sm text-[var(--dash-text-muted)]">
            Map spoken phrases to exact text after transcription — product names,
            APIs, casing, or agent mentions (e.g. “bridge mind” → BridgeMind).
            Processed locally on your Mac.
          </p>

          {#if settings.dictionary.length > 0}
            <ul class="mt-5 space-y-2" aria-label="Dictionary rules">
              {#each settings.dictionary as entry, index (entry.from + entry.to)}
                <li
                  class="group flex items-center gap-2 rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2.5 text-sm transition hover:border-white/12"
                >
                  <span
                    class="min-w-0 flex-1 truncate font-mono text-[var(--dash-text-muted)]"
                    title={entry.from}
                  >
                    {entry.from}
                  </span>
                  <span class="shrink-0 text-[var(--dash-text-subtle)]" aria-hidden="true"
                    >→</span
                  >
                  <span
                    class="min-w-0 flex-1 truncate font-mono text-[var(--dash-text)]"
                    title={entry.to}
                  >
                    {entry.to}
                  </span>
                  <button
                    type="button"
                    class="shrink-0 rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] opacity-0 transition group-hover:opacity-100 hover:bg-white/5 hover:text-red-400 focus:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                    onclick={() => removeDictionaryEntry(index)}
                    aria-label="Remove dictionary rule"
                  >
                    Delete
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="mt-4 text-sm text-[var(--dash-text-subtle)]">
              No rules yet. Add one below or use Dictionary on a history entry.
            </p>
          {/if}

          <div class="mt-5 grid gap-3 sm:grid-cols-2">
            <label class="block">
              <span class="text-xs font-medium text-[var(--dash-text-muted)]"
                >Spoken phrase</span
              >
              <input
                type="text"
                bind:value={dictionaryFrom}
                placeholder="bridge mind"
                class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
              />
            </label>
            <label class="block">
              <span class="text-xs font-medium text-[var(--dash-text-muted)]"
                >Replace with</span
              >
              <input
                type="text"
                bind:value={dictionaryTo}
                placeholder="BridgeMind"
                class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
                onkeydown={(e) => {
                  if (e.key === "Enter") void addDictionaryEntry();
                }}
              />
            </label>
          </div>
          {#if dictionaryError}
            <p class="mt-2 text-xs text-red-300">{dictionaryError}</p>
          {/if}
          <button
            type="button"
            class="mt-3 rounded-md bg-[var(--dash-accent)] px-4 py-2 text-sm font-medium text-white transition hover:bg-[var(--dash-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50"
            onclick={() => addDictionaryEntry()}
          >
            Add rule
          </button>
        </section>
      {:else if section === "history"}
        <section class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5">
          {#if historyEntries.length > 0}
            <div class="mb-4 flex justify-end">
              <button
                type="button"
                class="text-xs text-[var(--dash-text-subtle)] transition hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                onclick={clearHistory}
              >
                Clear all
              </button>
            </div>
          {/if}

          {#if filteredHistory.length === 0}
            <div class="flex flex-col items-center py-12 text-center">
              <div
                class="mb-4 flex h-14 w-14 items-center justify-center rounded-[10px] border border-[var(--dash-border)] bg-white/[0.03]"
                aria-hidden="true"
              >
                <svg
                  class="h-7 w-7 text-[var(--dash-text-subtle)]"
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
              <p class="text-sm font-medium text-[var(--dash-text-muted)]">
                {historyEntries.length === 0
                  ? "No transcriptions yet"
                  : "No entries in this period"}
              </p>
              <p class="mt-1 max-w-sm text-xs text-[var(--dash-text-subtle)]">
                Hold your hotkey anywhere on your Mac and speak — text appears in
                the focused field and shows up here.
              </p>
            </div>
          {:else}
            <ul class="space-y-3">
              {#each filteredHistory as entry (entry.id)}
                <li
                  class="group rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] p-4 transition hover:border-[var(--dash-accent)]/25"
                >
                  <div class="flex items-start justify-between gap-3">
                    <div
                      class="flex flex-wrap items-center gap-2 text-xs text-[var(--dash-text-subtle)]"
                    >
                      <time datetime={new Date(entry.timestamp).toISOString()}>
                        {relativeTime(entry.timestamp)}
                      </time>
                      {#if entry.app_name}
                        <span
                          class="rounded-md border border-white/10 bg-white/[0.03] px-1.5 py-0.5 text-[var(--dash-text-muted)]"
                        >
                          {entry.app_name}
                        </span>
                      {/if}
                      <span>{formatDuration(entry.duration_ms)}</span>
                    </div>
                    <div
                      class="flex shrink-0 gap-1 opacity-0 transition group-hover:opacity-100 focus-within:opacity-100"
                    >
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                        onclick={() =>
                          startDictionaryFromHistory(entry.normalized_text)}
                      >
                        Dictionary
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                        onclick={() => deleteEntry(entry.id)}
                        aria-label="Delete transcription"
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                  <p class="mt-2 text-sm leading-relaxed text-[var(--dash-text)]">
                    {entry.normalized_text}
                  </p>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {:else}
        <div class="max-w-xl space-y-4">
          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <h2 class="text-sm font-semibold text-white">Dictation hotkey</h2>
            <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
              Hold this combination to record. Release to transcribe and paste.
            </p>
            <input
              type="text"
              bind:value={hotkeyInput}
              placeholder="control+`"
              oninput={scheduleSave}
              onblur={() => saveSettings()}
              class="mt-3 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2.5 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
              aria-describedby="hotkey-help"
            />
            <p id="hotkey-help" class="mt-2 text-xs text-[var(--dash-text-subtle)]">
              Examples: <code class="text-[var(--dash-text-muted)]">control+`</code>,
              <code class="text-[var(--dash-text-muted)]">command+shift+d</code>
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

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="text-sm font-semibold text-white">Sound effects</h2>
                <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
                  Play short sounds when recording starts, while processing, and when
                  transcription completes.
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.sound_effects_enabled}
                aria-label="Enable sound effects"
                class="relative mt-0.5 h-6 w-11 shrink-0 rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50 {settings.sound_effects_enabled
                  ? 'bg-[var(--dash-accent)]'
                  : 'bg-[#3a3a44]'}"
                onclick={toggleSoundEffects}
              >
                <span
                  class="absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform {settings.sound_effects_enabled
                    ? 'translate-x-5'
                    : 'translate-x-0'}"
                ></span>
              </button>
            </div>
          </section>

          {#if isSaving || saveMessage}
            <p class="text-sm text-[var(--dash-text-muted)]" role="status">
              {#if isSaving}
                Saving…
              {:else}
                <span class="text-emerald-400">{saveMessage}</span>
              {/if}
            </p>
          {/if}
        </div>
      {/if}
    </main>
  </div>

  <DashboardStatusBar
    modelLabel={modelShortName}
    ready={parakeetStatus?.ready ?? false}
    installing={installing || (parakeetStatus?.install_in_progress ?? false)}
  />
</div>
