<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import DashboardSidebar from "$lib/components/dashboard/DashboardSidebar.svelte";
  import DashboardStatusBar from "$lib/components/dashboard/DashboardStatusBar.svelte";
  import DashboardPageHeader from "$lib/components/dashboard/DashboardPageHeader.svelte";
  import LocalePicker from "$lib/components/dashboard/LocalePicker.svelte";
  import type { DashboardSection } from "$lib/components/dashboard/types";
  import {
    locale,
    messagesFor,
    format,
    setLocale,
    normalizeLocale,
    type UiLocale,
  } from "$lib/i18n";

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

  type TranscriptionTiming = {
    total_ms: number;
    asr_ms?: number;
    postprocess_ms?: number;
    typing_ms?: number;
  };

  type HistoryEntry = {
    id: string;
    timestamp: number;
    normalized_text: string;
    app_name: string | null;
    duration_ms: number;
    engine?: string;
    local_model?: string | null;
    timing?: TranscriptionTiming | null;
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
    paste_delay_before_ms: number;
    paste_delay_after_ms: number;
    restore_clipboard_after_paste: boolean;
    recording_mode: string;
    strip_filler_words: boolean;
    warmup_on_start: boolean;
    ui_locale: string;
  };

  type SetupStage = {
    id: string;
    label: string;
  };

  const msg = $derived(messagesFor($locale));

  const SETUP_STAGES = $derived<SetupStage[]>([
    { id: "download", label: msg.setup.stages.download },
    { id: "verify", label: msg.setup.stages.verify },
    { id: "extract", label: msg.setup.stages.extract },
    { id: "install", label: msg.setup.stages.install },
    { id: "ready", label: msg.setup.stages.ready },
  ]);

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
    paste_delay_before_ms: 50,
    paste_delay_after_ms: 30,
    restore_clipboard_after_paste: true,
    recording_mode: "push_to_talk",
    strip_filler_words: false,
    warmup_on_start: true,
    ui_locale: "en",
  });
  let dictionaryFrom = $state("");
  let dictionaryTo = $state("");
  let dictionaryError = $state("");
  let hotkeyInput = $state("control+`");
  let hotkeyError = $state("");
  let showHotkeyErrorDetails = $state(false);
  let saveMessage = $state("");
  let isSaving = $state(false);
  let dictionaryImportMessage = $state("");
  let repasteMessage = $state("");
  let csvFileInput: HTMLInputElement | undefined = $state();

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
    const m = messagesFor($locale);
    const lower = raw.toLowerCase();
    if (lower.includes("404") && lower.includes("download")) {
      return {
        title: m.errors.downloadTitle,
        hint: m.errors.downloadHint,
      };
    }
    if (lower.includes("checksum")) {
      return {
        title: m.errors.checksumTitle,
        hint: m.errors.checksumHint,
      };
    }
    const firstLine = raw.split("\n")[0]?.replace(/https?:\/\/\S+/g, "").trim();
    return {
      title:
        firstLine && firstLine.length < 100 ? firstLine : m.errors.genericTitle,
      hint: m.errors.genericHint,
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
    if (stageId === "ready") {
      return parakeetStatus?.ready ?? false;
    }
    const current = installProgress?.stage ?? parakeetStatus?.install_stage;
    const cur = stageIndex(current);
    const target = stageIndex(stageId);
    if (target < 0 || cur < 0) {
      if (stageId === "download") return parakeetStatus?.model_downloaded ?? false;
      return false;
    }
    return cur > target;
  }

  function isStageActive(stageId: string): boolean {
    if (isStageComplete(stageId)) return false;
    const current = installProgress?.stage ?? parakeetStatus?.install_stage ?? "";
    if (current !== stageId) return false;
    return installing || (parakeetStatus?.install_in_progress ?? false);
  }

  function clearInstallProgressIfReady() {
    if (parakeetStatus?.ready && !installing && !parakeetStatus.install_in_progress) {
      installProgress = null;
    }
  }

  function relativeTime(ts: number): string {
    const m = messagesFor($locale);
    const diff = Date.now() - ts;
    const sec = Math.floor(diff / 1000);
    if (sec < 60) return m.history.justNow;
    const min = Math.floor(sec / 60);
    if (min < 60) return format(m.history.minutesAgo, { n: min });
    const hr = Math.floor(min / 60);
    if (hr < 24) return format(m.history.hoursAgo, { n: hr });
    const day = Math.floor(hr / 24);
    if (day < 7) return format(m.history.daysAgo, { n: day });
    const dateLocale = $locale === "ru" ? "ru-RU" : "en-US";
    return new Date(ts).toLocaleDateString(dateLocale, {
      month: "short",
      day: "numeric",
    });
  }

  function formatDuration(ms: number): string {
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    return `${Math.floor(s / 60)}m ${s % 60}s`;
  }

  function formatTimingDetail(entry: HistoryEntry): string | null {
    const m = messagesFor($locale);
    const t = entry.timing;
    if (!t?.asr_ms) return null;
    const parts = [
      format(m.history.asrTiming, { ms: Math.round(t.asr_ms) }),
    ];
    if (t.typing_ms != null) {
      parts.push(
        format(m.history.pasteTiming, { ms: Math.round(t.typing_ms) }),
      );
    }
    return parts.join(" · ");
  }

  async function repasteLast() {
    repasteMessage = "";
    try {
      await invoke("repaste_last");
      repasteMessage = messagesFor($locale).settingsPanel.repasted;
    } catch (e) {
      repasteMessage = String(e);
    }
    setTimeout(() => {
      repasteMessage = "";
    }, 2500);
  }

  async function pasteHistoryEntry(text: string) {
    try {
      await invoke("paste_text_command", { text });
    } catch (e) {
      console.error(e);
    }
  }

  async function exportDictionary() {
    try {
      const csv = await invoke<string>("export_dictionary_csv");
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "speech-clip-dictionary.csv";
      a.click();
      URL.revokeObjectURL(url);
      dictionaryImportMessage = messagesFor($locale).dictionary.exported;
    } catch (e) {
      dictionaryImportMessage = String(e);
    }
    setTimeout(() => {
      dictionaryImportMessage = "";
    }, 2500);
  }

  function triggerDictionaryImport() {
    csvFileInput?.click();
  }

  async function onDictionaryFileSelected(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    dictionaryImportMessage = "";
    try {
      const csv = await file.text();
      const result = await invoke<{
        entries_added: number;
        dictionary: DictionaryEntry[];
      }>("import_dictionary_csv", { csv, merge: true });
      settings.dictionary = result.dictionary;
      dictionaryImportMessage = format(
        messagesFor($locale).dictionary.imported,
        { count: result.entries_added },
      );
    } catch (e) {
      dictionaryImportMessage = String(e);
    }
    setTimeout(() => {
      dictionaryImportMessage = "";
    }, 3000);
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
      clearInstallProgressIfReady();
    }
  }

  async function runInstall() {
    if (installing) return;
    installing = true;
    installError = "";
    showErrorDetails = false;
    installProgress = {
      stage: "start",
      message: messagesFor($locale).setup.starting,
      percent: 0,
    };
    try {
      parakeetStatus = await invoke<ParakeetStatus>("ensure_parakeet_runtime");
      installProgress = null;
    } catch (e) {
      installError = String(e);
      await refreshStatus();
    } finally {
      installing = false;
      clearInstallProgressIfReady();
    }
  }

  async function loadSettings() {
    const s = await invoke<AppSettings>("get_settings");
    settings = {
      ...s,
      ui_locale: normalizeLocale(s.ui_locale),
    };
    hotkeyInput = s.hotkey;
    setLocale(normalizeLocale(s.ui_locale));
  }

  function onUiLocaleChange(next: UiLocale) {
    settings.ui_locale = next;
    setLocale(next);
    scheduleSave();
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
      saveMessage = messagesFor($locale).settingsPanel.saved;
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
    const m = messagesFor($locale);
    if (!from) {
      dictionaryError = m.dictionary.errFrom;
      return;
    }
    if (!to) {
      dictionaryError = m.dictionary.errTo;
      return;
    }
    if (
      settings.dictionary.some((e) => e.from.toLowerCase() === from.toLowerCase())
    ) {
      dictionaryError = m.dictionary.errDuplicate;
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
        } else if (event.payload.stage === "ready" && !installing) {
          void refreshStatus();
        }
      },
    );

    await loadSettings();
    await loadHistory();
    await refreshStatus();
    void invoke("warmup_parakeet").catch(() => {});

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
    installing ||
      parakeetStatus?.install_in_progress ||
      (installProgress !== null && !parakeetStatus?.ready),
  );
  const meta = $derived(msg.sections[section]);
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
              <span class="sr-only">{msg.history.filterAria}</span>
              <select
                bind:value={historyFilter}
                class="rounded-md border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-1.5 text-sm text-[var(--dash-text)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
              >
                <option value="all">{msg.history.allTime}</option>
                <option value="week">{msg.history.pastWeek}</option>
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
                <h2 class="text-lg font-semibold text-white">{msg.hero.checkingTitle}</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {msg.hero.checkingSubtitle}
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
                <h2 class="text-lg font-semibold text-white">{msg.hero.readyTitle}</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {#if settings.recording_mode === "toggle"}
                    {msg.hero.toggleHintBefore}
                    <kbd
                      class="mx-1 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[var(--dash-text)]"
                      >{settings.hotkey}</kbd
                    >
                    {msg.hero.toggleHintAfter}
                  {:else}
                    {msg.hero.holdHintBefore}
                    <kbd
                      class="mx-1 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[var(--dash-text)]"
                      >{settings.hotkey}</kbd
                    >
                    {msg.hero.holdHintAfter}
                  {/if}
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
                <h2 class="text-lg font-semibold text-white">{msg.hero.settingUpTitle}</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {installProgress?.message ??
                    parakeetStatus?.message ??
                    msg.hero.settingUpFallback}
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
                  {errorInfo?.title ?? msg.hero.setupFailed}
                </h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {errorInfo?.hint ?? msg.hero.tryAgain}
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
                <h2 class="text-lg font-semibold text-white">{msg.hero.finishTitle}</h2>
                <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
                  {parakeetStatus?.message ?? msg.hero.finishFallback}
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
                {showErrorDetails ? msg.errors.hideDetails : msg.errors.showDetails}
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
                {installing ? msg.hero.installing : msg.hero.runSetup}
              </button>
            {/if}
            <button
              type="button"
              class="rounded-md border border-[var(--dash-border)] bg-transparent px-4 py-2 text-sm font-medium text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30 disabled:opacity-50"
              onclick={refreshStatus}
              disabled={installing}
            >
              {msg.hero.refresh}
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
                {msg.cards.model}
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
                {msg.cards.status}
              </p>
              <p class="mt-1 text-sm font-medium {parakeetStatus.ready
                ? 'text-emerald-400'
                : 'text-[var(--dash-text-muted)]'}">
                {parakeetStatus.ready ? msg.cards.ready : msg.cards.pending}
              </p>
            </div>
            <div
              class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] px-3 py-3"
            >
              <p
                class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
              >
                {msg.cards.hotkey}
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
                {msg.cards.dictionary}
              </p>
              <p class="mt-1 text-sm font-medium text-[var(--dash-text)]">
                {format(msg.cards.rules, { count: settings.dictionary.length })}
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
                {msg.cards.speechModel}
              </h3>
              <button
                type="button"
                class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] underline-offset-2 transition hover:text-[var(--dash-text-muted)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                onclick={() => (showTechDetails = !showTechDetails)}
                aria-expanded={showTechDetails}
              >
                {showTechDetails ? msg.cards.less : msg.cards.details}
              </button>
            </div>

            <ul class="mt-3 space-y-2">
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">{msg.cards.parakeetOnnx}</span>
                <span
                  class="rounded-md px-2 py-0.5 text-xs font-medium {parakeetStatus.model_downloaded
                    ? 'bg-emerald-500/15 text-emerald-400'
                    : 'bg-white/5 text-[var(--dash-text-muted)]'}"
                >
                  {parakeetStatus.model_downloaded
                    ? msg.cards.installed
                    : msg.cards.pending}
                </span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">{msg.cards.runtime}</span>
                <span class="text-xs font-medium {parakeetStatus.ready
                  ? 'text-emerald-400'
                  : 'text-[var(--dash-text-muted)]'}">
                  {parakeetStatus.ready ? msg.cards.ready : msg.cards.notReady}
                </span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">{msg.cards.engine}</span>
                <span class="font-mono text-xs text-[var(--dash-text)]">transcribe-rs</span>
              </li>
              <li class="flex items-center justify-between gap-3 text-sm">
                <span class="text-[var(--dash-text-muted)]">{msg.cards.modelId}</span>
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
                  <dt>{msg.cards.modelDirectory}</dt>
                  <dd class="mt-0.5 break-all text-[var(--dash-text-muted)]">
                    {parakeetStatus.model_dir}
                  </dd>
                </div>
                <div>
                  <dt>{msg.cards.configuredModel}</dt>
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
            {msg.dictionary.intro}
          </p>

          {#if settings.dictionary.length > 0}
            <ul class="mt-5 space-y-2" aria-label={msg.dictionary.rulesAria}>
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
                    aria-label={msg.dictionary.removeAria}
                  >
                    {msg.dictionary.delete}
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="mt-4 text-sm text-[var(--dash-text-subtle)]">
              {msg.dictionary.empty}
            </p>
          {/if}

          <div class="mt-5 grid gap-3 sm:grid-cols-2">
            <label class="block">
              <span class="text-xs font-medium text-[var(--dash-text-muted)]"
                >{msg.dictionary.spokenPhrase}</span
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
                >{msg.dictionary.replaceWith}</span
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
          <div class="mt-3 flex flex-wrap items-center gap-2">
            <button
              type="button"
              class="rounded-md bg-[var(--dash-accent)] px-4 py-2 text-sm font-medium text-white transition hover:bg-[var(--dash-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50"
              onclick={() => addDictionaryEntry()}
            >
              {msg.dictionary.addRule}
            </button>
            <button
              type="button"
              class="rounded-md border border-[var(--dash-border)] px-3 py-2 text-sm text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
              onclick={exportDictionary}
            >
              {msg.dictionary.exportCsv}
            </button>
            <button
              type="button"
              class="rounded-md border border-[var(--dash-border)] px-3 py-2 text-sm text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
              onclick={triggerDictionaryImport}
            >
              {msg.dictionary.importCsv}
            </button>
            <input
              bind:this={csvFileInput}
              type="file"
              accept=".csv,text/csv"
              class="hidden"
              onchange={onDictionaryFileSelected}
            />
          </div>
          {#if dictionaryImportMessage}
            <p class="mt-2 text-xs text-[var(--dash-text-muted)]" role="status">
              {dictionaryImportMessage}
            </p>
          {/if}
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
                {msg.history.clearAll}
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
                  ? msg.history.empty
                  : msg.history.emptyPeriod}
              </p>
              <p class="mt-1 max-w-sm text-xs text-[var(--dash-text-subtle)]">
                {msg.history.emptyHint}
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
                      {#if formatTimingDetail(entry)}
                        <span class="text-[var(--dash-text-subtle)]"
                          >{formatTimingDetail(entry)}</span
                        >
                      {/if}
                    </div>
                    <div
                      class="flex shrink-0 gap-1 opacity-0 transition group-hover:opacity-100 focus-within:opacity-100"
                    >
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                        onclick={() => pasteHistoryEntry(entry.normalized_text)}
                      >
                        {msg.history.pasteAgain}
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                        onclick={() =>
                          startDictionaryFromHistory(entry.normalized_text)}
                      >
                        {msg.history.dictionary}
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                        onclick={() => deleteEntry(entry.id)}
                        aria-label={msg.history.deleteAria}
                      >
                        {msg.dictionary.delete}
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
          <LocalePicker
            value={normalizeLocale(settings.ui_locale)}
            onchange={onUiLocaleChange}
          />

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.hotkeyTitle}</h2>
            <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
              {#if settings.recording_mode === "toggle"}
                {msg.settingsPanel.toggleModeHint}
              {:else}
                {msg.settingsPanel.holdModeHint}
              {/if}
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
              {msg.settingsPanel.hotkeyExamples}
              <code class="text-[var(--dash-text-muted)]">control+`</code>,
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
                  {showHotkeyErrorDetails
                    ? msg.errors.hideDetails
                    : msg.cards.details}
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
            <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.recordingTitle}</h2>
            <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
              {msg.settingsPanel.recordingHint}
            </p>
            <div class="mt-3 flex gap-2">
              <button
                type="button"
                class="rounded-lg border px-3 py-2 text-sm transition focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40 {settings.recording_mode !==
                'toggle'
                  ? 'border-[var(--dash-accent)]/50 bg-[var(--dash-accent-muted)] text-[#ffb899]'
                  : 'border-[var(--dash-border)] text-[var(--dash-text-muted)] hover:border-white/20'}"
                onclick={() => {
                  settings.recording_mode = "push_to_talk";
                  scheduleSave();
                }}
              >
                {msg.settingsPanel.pushToTalk}
              </button>
              <button
                type="button"
                class="rounded-lg border px-3 py-2 text-sm transition focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40 {settings.recording_mode ===
                'toggle'
                  ? 'border-[var(--dash-accent)]/50 bg-[var(--dash-accent-muted)] text-[#ffb899]'
                  : 'border-[var(--dash-border)] text-[var(--dash-text-muted)] hover:border-white/20'}"
                onclick={() => {
                  settings.recording_mode = "toggle";
                  scheduleSave();
                }}
              >
                {msg.settingsPanel.toggle}
              </button>
            </div>
          </section>

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.pasteTitle}</h2>
            <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
              {msg.settingsPanel.pasteHint}
            </p>
            <div class="mt-3 grid gap-3 sm:grid-cols-2">
              <label class="block">
                <span class="text-xs text-[var(--dash-text-muted)]">{msg.settingsPanel.delayBefore}</span>
                <input
                  type="number"
                  min="0"
                  max="2000"
                  bind:value={settings.paste_delay_before_ms}
                  onchange={scheduleSave}
                  class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
                />
              </label>
              <label class="block">
                <span class="text-xs text-[var(--dash-text-muted)]">{msg.settingsPanel.delayAfter}</span>
                <input
                  type="number"
                  min="0"
                  max="2000"
                  bind:value={settings.paste_delay_after_ms}
                  onchange={scheduleSave}
                  class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
                />
              </label>
            </div>
            <label class="mt-4 flex cursor-pointer items-center gap-3">
              <input
                type="checkbox"
                bind:checked={settings.restore_clipboard_after_paste}
                onchange={scheduleSave}
                class="rounded border-[var(--dash-border)]"
              />
              <span class="text-sm text-[var(--dash-text-muted)]"
                >{msg.settingsPanel.restoreClipboard}</span
              >
            </label>
          </section>

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.fillerTitle}</h2>
                <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
                  {msg.settingsPanel.fillerHint}
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.strip_filler_words}
                aria-label={msg.settingsPanel.fillerAria}
                class="relative mt-0.5 h-6 w-11 shrink-0 rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50 {settings.strip_filler_words
                  ? 'bg-[var(--dash-accent)]'
                  : 'bg-[#3a3a44]'}"
                onclick={() => {
                  settings.strip_filler_words = !settings.strip_filler_words;
                  void saveSettings({ quiet: true });
                }}
              >
                <span
                  class="absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform {settings.strip_filler_words
                    ? 'translate-x-5'
                    : 'translate-x-0'}"
                ></span>
              </button>
            </div>
          </section>

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.repasteTitle}</h2>
            <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
              {msg.settingsPanel.repasteHint}
            </p>
            <button
              type="button"
              class="mt-3 rounded-md border border-[var(--dash-border)] px-4 py-2 text-sm text-[var(--dash-text-muted)] transition hover:border-[var(--dash-accent)]/40 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
              onclick={repasteLast}
            >
              {msg.settingsPanel.repasteButton}
            </button>
            {#if repasteMessage}
              <p class="mt-2 text-xs text-[var(--dash-text-muted)]">{repasteMessage}</p>
            {/if}
          </section>

          <section
            class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
          >
            <div class="flex items-start justify-between gap-4">
              <div>
                <h2 class="text-sm font-semibold text-white">{msg.settingsPanel.soundsTitle}</h2>
                <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
                  {msg.settingsPanel.soundsHint}
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={settings.sound_effects_enabled}
                aria-label={msg.settingsPanel.soundsAria}
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
                {msg.settingsPanel.saving}
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
