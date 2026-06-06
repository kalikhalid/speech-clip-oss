<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import DashboardSidebar from "$lib/components/dashboard/DashboardSidebar.svelte";
  import DashboardStatusBar from "$lib/components/dashboard/DashboardStatusBar.svelte";
  import DashboardPageHeader from "$lib/components/dashboard/DashboardPageHeader.svelte";
  import type { DashboardSection } from "$lib/components/dashboard/types";
  import LazyDashboardSection from "$lib/loading/LazyDashboardSection.svelte";
  import { setDashboardContext } from "$lib/loading/dashboard-context";
  import {
    preloadDashboardSection,
  } from "$lib/loading/dashboard-sections";
  import type {
    AppSettings,
    DictionaryEntry,
    HistoryEntry,
    ParakeetStatus,
    InstallProgress,
    HeroState,
    SetupStage,
    DictationStats,
    NormalizerSetupStatus,
  } from "$lib/loading/dashboard-types";
  import {
    locale,
    messagesFor,
    format,
    fetchSettingsOnce,
    setLocale,
    normalizeLocale,
    type UiLocale,
  } from "$lib/i18n";

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
  let normalizerStatus = $state<NormalizerSetupStatus | null>(null);
  let normalizerInstalling = $state(false);
  let normalizerError = $state("");
  let statusLoading = $state(true);
  let installing = $state(false);
  let installError = $state("");
  let showErrorDetails = $state(false);
  let installProgress = $state<InstallProgress | null>(null);
  let dictationStats = $state<DictationStats | null>(null);
  let statsLoading = $state(true);
  let historyEntries = $state<HistoryEntry[]>([]);
  let settings = $state<AppSettings>({
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
    hide_idle_pill: false,
    seed_dictionary_enabled: true,
    seed_dictionary_count: 0,
    show_asr_raw_in_history: false,
    dictation_normalize: true,
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

  let unlistenProgress: (() => void) | null = null;
  let unlistenNormalizerProgress: (() => void) | null = null;
  let unlistenHistory: (() => void) | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

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
    if (t.normalizer_ms != null) {
      parts.push(
        format(m.history.normalizerTiming, { ms: Math.round(t.normalizer_ms) }),
      );
    }
    if (t.typing_ms != null) {
      parts.push(
        format(m.history.pasteTiming, { ms: Math.round(t.typing_ms) }),
      );
    }
    return parts.join(" · ");
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

  async function refreshNormalizerStatus() {
    normalizerError = "";
    try {
      normalizerStatus = await invoke<NormalizerSetupStatus>("get_normalizer_status");
    } catch (e) {
      normalizerStatus = null;
      normalizerError = String(e);
      console.error(e);
    }
  }

  async function runNormalizerInstall() {
    if (normalizerInstalling) return;
    normalizerInstalling = true;
    normalizerError = "";
    try {
      normalizerStatus = await invoke<NormalizerSetupStatus>("ensure_normalizer_model");
    } catch (e) {
      normalizerError = String(e);
      await refreshNormalizerStatus();
    } finally {
      normalizerInstalling = false;
    }
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
    void loadDictationStats();
    void refreshNormalizerStatus();
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
    const s = await fetchSettingsOnce<AppSettings>();
    settings = {
      ...s,
      ui_locale: normalizeLocale(s.ui_locale),
      dictation_normalize: s.dictation_normalize ?? true,
    };
    hotkeyInput = s.hotkey;
    await setLocale(normalizeLocale(s.ui_locale));
  }

  async function onUiLocaleChange(next: UiLocale) {
    settings.ui_locale = next;
    await setLocale(next);
    scheduleSave();
  }

  function mapHistoryEntry(raw: HistoryEntry): HistoryEntry {
    return {
      id: raw.id,
      timestamp: raw.timestamp,
      raw_text: raw.raw_text,
      normalized_text: raw.normalized_text,
      app_name: raw.app_name ?? null,
      duration_ms: raw.duration_ms,
      engine: raw.engine,
      local_model: raw.local_model ?? null,
      timing: raw.timing ?? null,
    };
  }

  function prependHistoryEntry(raw: HistoryEntry) {
    const entry = mapHistoryEntry(raw);
    if (historyEntries.some((e) => e.id === entry.id)) return;
    historyEntries = [entry, ...historyEntries];
    void loadDictationStats();
  }

  async function loadHistory() {
    try {
      const h: { entries: HistoryEntry[] } = await invoke("get_history", {
        limit: null,
      });
      historyEntries = (h.entries ?? []).map(mapHistoryEntry);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadDictationStats() {
    statsLoading = true;
    try {
      dictationStats = await invoke<DictationStats>("get_dictation_stats");
    } catch (e) {
      console.error(e);
      dictationStats = { words_24h: 0, words_7d: 0, words_all_time: 0 };
    } finally {
      statsLoading = false;
    }
  }

  function formatWordCount(count: number): string {
    const loc = $locale === "ru" ? "ru-RU" : "en-US";
    return new Intl.NumberFormat(loc).format(count);
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
      settings.hotkey = hotkeyToSave;
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

  async function toggleHideIdlePill() {
    const previous = settings.hide_idle_pill;
    settings.hide_idle_pill = !previous;
    const saved = await saveSettings();
    if (!saved) {
      settings.hide_idle_pill = previous;
    }
  }

  async function toggleSeedDictionary() {
    const previous = settings.seed_dictionary_enabled;
    settings.seed_dictionary_enabled = !previous;
    const saved = await saveSettings({ quiet: true });
    if (!saved) {
      settings.seed_dictionary_enabled = previous;
    }
  }

  async function toggleShowAsrRawInHistory() {
    const previous = settings.show_asr_raw_in_history;
    settings.show_asr_raw_in_history = !previous;
    const saved = await saveSettings({ quiet: true });
    if (!saved) {
      settings.show_asr_raw_in_history = previous;
    }
  }

  async function toggleDictationNormalize() {
    const previous = settings.dictation_normalize;
    settings.dictation_normalize = !previous;
    const saved = await saveSettings({ quiet: true });
    if (!saved) {
      settings.dictation_normalize = previous;
    } else if (settings.dictation_normalize) {
      void refreshNormalizerStatus();
    }
  }

  async function deleteEntry(id: string) {
    await invoke("delete_history_entry", { entryId: id });
    historyEntries = historyEntries.filter((e) => e.id !== id);
    void loadDictationStats();
  }

  async function clearHistory() {
    await invoke("clear_all_history");
    historyEntries = [];
    void loadDictationStats();
  }

  function navigate(next: DashboardSection) {
    void preloadDashboardSection(next);
    section = next;
    if (next === "history") {
      void loadHistory();
    } else if (next === "general") {
      void loadDictationStats();
    }
  }

  setDashboardContext({
    get section() {
      return section;
    },
    set section(value) {
      section = value;
    },
    get parakeetStatus() {
      return parakeetStatus;
    },
    get normalizerStatus() {
      return normalizerStatus;
    },
    get normalizerInstalling() {
      return normalizerInstalling;
    },
    get normalizerError() {
      return normalizerError;
    },
    get statusLoading() {
      return statusLoading;
    },
    get installing() {
      return installing;
    },
    get installError() {
      return installError;
    },
    get showErrorDetails() {
      return showErrorDetails;
    },
    set showErrorDetails(value) {
      showErrorDetails = value;
    },
    get installProgress() {
      return installProgress;
    },
    get dictationStats() {
      return dictationStats;
    },
    get statsLoading() {
      return statsLoading;
    },
    get historyEntries() {
      return historyEntries;
    },
    get settings() {
      return settings;
    },
    get dictionaryFrom() {
      return dictionaryFrom;
    },
    set dictionaryFrom(value) {
      dictionaryFrom = value;
    },
    get dictionaryTo() {
      return dictionaryTo;
    },
    set dictionaryTo(value) {
      dictionaryTo = value;
    },
    get dictionaryError() {
      return dictionaryError;
    },
    get dictionaryImportMessage() {
      return dictionaryImportMessage;
    },
    get hotkeyInput() {
      return hotkeyInput;
    },
    set hotkeyInput(value) {
      hotkeyInput = value;
    },
    get hotkeyError() {
      return hotkeyError;
    },
    get showHotkeyErrorDetails() {
      return showHotkeyErrorDetails;
    },
    set showHotkeyErrorDetails(value) {
      showHotkeyErrorDetails = value;
    },
    get saveMessage() {
      return saveMessage;
    },
    get isSaving() {
      return isSaving;
    },
    get hero() {
      return hero;
    },
    get errorInfo() {
      return errorInfo;
    },
    get progressPercent() {
      return progressPercent;
    },
    get showSetupProgress() {
      return showSetupProgress;
    },
    get setupStages() {
      return SETUP_STAGES;
    },
    get filteredHistory() {
      return filteredHistory;
    },
    loadDictationStats,
    formatWordCount,
    isStageComplete,
    isStageActive,
    runInstall,
    refreshStatus,
    refreshNormalizerStatus,
    runNormalizerInstall,
    addDictionaryEntry,
    removeDictionaryEntry,
    exportDictionary,
    onDictionaryFileSelected,
    scheduleSave,
    saveSettings,
    onUiLocaleChange,
    toggleSoundEffects,
    toggleHideIdlePill,
    toggleSeedDictionary,
    toggleShowAsrRawInHistory,
    toggleDictationNormalize,
    pasteHistoryEntry,
    startDictionaryFromHistory,
    deleteEntry,
    clearHistory,
    formatTimingDetail,
    relativeTime,
    formatDuration,
  });

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

    unlistenNormalizerProgress = await listen<InstallProgress>(
      "normalizer-install-progress",
      (event) => {
        if (event.payload.stage === "failed") {
          normalizerError = event.payload.message;
        } else if (event.payload.stage === "ready" && !normalizerInstalling) {
          void refreshNormalizerStatus();
        }
      },
    );

    unlistenHistory = await listen<HistoryEntry>("history-updated", (event) => {
      prependHistoryEntry(event.payload);
    });

    void preloadDashboardSection("general");

    await Promise.all([loadSettings(), refreshStatus(), loadDictationStats()]);
    void invoke("warmup_parakeet").catch(() => {});

    if (parakeetStatus && !parakeetStatus.ready) {
      void runInstall();
    }
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    unlistenProgress?.();
    unlistenNormalizerProgress?.();
    unlistenHistory?.();
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
  class="dashboard-shell flex h-dvh min-w-[720px] flex-col bg-[var(--dash-bg)] font-sans text-[var(--dash-text)] antialiased"
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

      <LazyDashboardSection {section} />

    </main>
  </div>

  <DashboardStatusBar
    modelLabel={modelShortName}
    ready={parakeetStatus?.ready ?? false}
    installing={installing || (parakeetStatus?.install_in_progress ?? false)}
  />
</div>
