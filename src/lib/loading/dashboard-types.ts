import type { DashboardSection } from "$lib/components/dashboard/types";

export type NormalizerSetupStatus = {
  model_downloaded: boolean;
  model_id: string;
  model_path?: string | null;
  legacy: boolean;
};

export type ParakeetStatus = {
  model_id: string;
  model_dir: string;
  model_downloaded: boolean;
  ready: boolean;
  message: string;
  install_stage: string;
  install_in_progress: boolean;
};

export type InstallProgress = {
  stage: string;
  message: string;
  percent: number;
};

export type TranscriptionTiming = {
  total_ms: number;
  asr_ms?: number;
  normalizer_ms?: number;
  postprocess_ms?: number;
  typing_ms?: number;
};

export type HistoryEntry = {
  id: string;
  timestamp: number;
  raw_text?: string;
  normalized_text: string;
  app_name: string | null;
  duration_ms: number;
  engine?: string;
  local_model?: string | null;
  timing?: TranscriptionTiming | null;
};

export type DictionaryEntry = {
  from: string;
  to: string;
};

export type AppSettings = {
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
  hide_idle_pill: boolean;
  seed_dictionary_enabled: boolean;
  seed_dictionary_count: number;
  show_asr_raw_in_history: boolean;
  dictation_normalize: boolean;
};

export type SetupStage = {
  id: string;
  label: string;
};

export type HeroState = "loading" | "ready" | "setting-up" | "needs-setup" | "error";

export type DictationStats = {
  words_24h: number;
  words_7d: number;
  words_all_time: number;
};

export type DashboardContext = {
  get section(): DashboardSection;
  set section(value: DashboardSection);
  get parakeetStatus(): ParakeetStatus | null;
  get normalizerStatus(): NormalizerSetupStatus | null;
  get normalizerInstalling(): boolean;
  get normalizerError(): string;
  get statusLoading(): boolean;
  get installing(): boolean;
  get installError(): string;
  get showErrorDetails(): boolean;
  set showErrorDetails(value: boolean);
  get installProgress(): InstallProgress | null;
  get dictationStats(): DictationStats | null;
  get statsLoading(): boolean;
  get historyEntries(): HistoryEntry[];
  get settings(): AppSettings;
  get dictionaryFrom(): string;
  set dictionaryFrom(value: string);
  get dictionaryTo(): string;
  set dictionaryTo(value: string);
  get dictionaryError(): string;
  get dictionaryImportMessage(): string;
  get hotkeyInput(): string;
  set hotkeyInput(value: string);
  get hotkeyError(): string;
  get showHotkeyErrorDetails(): boolean;
  set showHotkeyErrorDetails(value: boolean);
  get saveMessage(): string;
  get isSaving(): boolean;
  get hero(): HeroState;
  get errorInfo(): { title: string; hint: string } | null;
  get progressPercent(): number;
  get showSetupProgress(): boolean;
  get setupStages(): SetupStage[];
  get filteredHistory(): HistoryEntry[];
  isStageComplete(stageId: string): boolean;
  loadDictationStats(): Promise<void>;
  formatWordCount(count: number): string;
  isStageActive(stageId: string): boolean;
  runInstall(): Promise<void>;
  refreshStatus(): Promise<void>;
  refreshNormalizerStatus(): Promise<void>;
  runNormalizerInstall(): Promise<void>;
  addDictionaryEntry(): Promise<void>;
  removeDictionaryEntry(index: number): Promise<void>;
  exportDictionary(): Promise<void>;
  onDictionaryFileSelected(event: Event): Promise<void>;
  scheduleSave(): void;
  saveSettings(options?: { quiet?: boolean }): Promise<boolean>;
  onUiLocaleChange(next: import("$lib/i18n").UiLocale): Promise<void>;
  toggleSoundEffects(): Promise<void>;
  toggleHideIdlePill(): Promise<void>;
  toggleSeedDictionary(): Promise<void>;
  toggleShowAsrRawInHistory(): Promise<void>;
  toggleDictationNormalize(): Promise<void>;
  pasteHistoryEntry(text: string): Promise<void>;
  startDictionaryFromHistory(text: string): void;
  deleteEntry(id: string): Promise<void>;
  clearHistory(): Promise<void>;
  formatTimingDetail(entry: HistoryEntry): string | null;
  relativeTime(ts: number): string;
  formatDuration(ms: number): string;
};
