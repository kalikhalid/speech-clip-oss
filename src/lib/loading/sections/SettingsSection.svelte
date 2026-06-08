<script lang="ts">
  import LocalePicker from "$lib/components/dashboard/LocalePicker.svelte";
  import { locale, messagesFor, normalizeLocale, format } from "$lib/i18n";
  import { useDashboard } from "../dashboard-context";

  const dash = useDashboard();
  const msg = $derived(messagesFor($locale));
</script>

{#snippet settingsSwitch(
  checked: boolean,
  ariaLabel: string,
  onToggle: () => void,
)}
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={ariaLabel}
    class="relative h-5 w-9 shrink-0 rounded-full transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50 {checked
      ? 'bg-[var(--dash-accent)]'
      : 'bg-[#3a3a44]'}"
    onclick={onToggle}
  >
    <span
      class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform {checked
        ? 'translate-x-4'
        : 'translate-x-0'}"
    ></span>
  </button>
{/snippet}

<div class="max-w-2xl space-y-3">
  <LocalePicker
    value={normalizeLocale(dash.settings.ui_locale)}
    onchange={(next) => void dash.onUiLocaleChange(next)}
  />

  <section
    class="overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)]"
  >
    <header class="border-b border-[var(--dash-border)] bg-white/[0.02] px-3.5 py-2">
      <h2
        class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
      >
        {msg.shortcutHint.dictation}
      </h2>
    </header>

    <div class="divide-y divide-[var(--dash-border)] px-3.5">
      <div class="py-2.5">
        <label for="dictation-hotkey" class="text-xs font-medium text-white">
          {msg.settingsPanel.hotkeyTitle}
        </label>
        <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
          {#if dash.settings.recording_mode === "toggle"}
            {msg.settingsPanel.toggleModeHint}
          {:else}
            {msg.settingsPanel.holdModeHint}
          {/if}
        </p>
        <input
          id="dictation-hotkey"
          type="text"
          bind:value={dash.hotkeyInput}
          placeholder="control+`"
          oninput={dash.scheduleSave}
          onblur={() => void dash.saveSettings()}
          class="mt-2 w-full rounded-md border border-[var(--dash-border)] bg-[var(--dash-bg)] px-2.5 py-1.5 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
          aria-describedby="hotkey-help"
        />
        <p id="hotkey-help" class="mt-1.5 text-[11px] text-[var(--dash-text-subtle)]">
          {msg.settingsPanel.hotkeyExamples}
          <code class="text-[var(--dash-text-muted)]">control+`</code>,
          <code class="text-[var(--dash-text-muted)]">command+shift+d</code>
        </p>
        {#if dash.hotkeyError}
          <div class="mt-2 rounded-md border border-red-500/20 bg-red-500/5 p-2.5">
            <p class="text-xs text-red-300">{dash.hotkeyError.split("\n")[0]}</p>
            <button
              type="button"
              class="mt-1 text-[11px] text-red-300/80 underline-offset-2 hover:underline"
              onclick={() =>
                (dash.showHotkeyErrorDetails = !dash.showHotkeyErrorDetails)}
            >
              {dash.showHotkeyErrorDetails
                ? msg.errors.hideDetails
                : msg.cards.details}
            </button>
            {#if dash.showHotkeyErrorDetails}
                      <pre
                        class="dashboard-selectable mt-1.5 font-mono text-[11px] text-red-200/60"
                        >{dash.hotkeyError}</pre
                      >
            {/if}
          </div>
        {/if}
      </div>

      <div class="py-2.5">
        <p class="text-xs font-medium text-white">{msg.settingsPanel.recordingTitle}</p>
        <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
          {msg.settingsPanel.recordingHint}
        </p>
        <div
          class="mt-2 grid grid-cols-2 gap-1.5"
          role="group"
          aria-label={msg.settingsPanel.recordingTitle}
        >
          <button
            type="button"
            class="rounded-md border px-2.5 py-1.5 text-xs transition focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40 {dash.settings
              .recording_mode !== 'toggle'
              ? 'border-[var(--dash-accent)]/50 bg-[var(--dash-accent-muted)] text-[#ffb899]'
              : 'border-[var(--dash-border)] text-[var(--dash-text-muted)] hover:border-white/20'}"
            onclick={() => {
              dash.settings.recording_mode = "push_to_talk";
              dash.scheduleSave();
            }}
          >
            {msg.settingsPanel.pushToTalk}
          </button>
          <button
            type="button"
            class="rounded-md border px-2.5 py-1.5 text-xs transition focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40 {dash.settings
              .recording_mode === 'toggle'
              ? 'border-[var(--dash-accent)]/50 bg-[var(--dash-accent-muted)] text-[#ffb899]'
              : 'border-[var(--dash-border)] text-[var(--dash-text-muted)] hover:border-white/20'}"
            onclick={() => {
              dash.settings.recording_mode = "toggle";
              dash.scheduleSave();
            }}
          >
            {msg.settingsPanel.toggle}
          </button>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">
            {msg.settingsPanel.hideIdlePillTitle}
          </p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.hideIdlePillHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.hide_idle_pill,
          msg.settingsPanel.hideIdlePillAria,
          () => void dash.toggleHideIdlePill(),
        )}
      </div>
    </div>
  </section>

  <section
    class="overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)]"
  >
    <header class="border-b border-[var(--dash-border)] bg-white/[0.02] px-3.5 py-2">
      <h2
        class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
      >
        {msg.settingsPanel.postProcessTitle}
      </h2>
    </header>

    <div class="divide-y divide-[var(--dash-border)] px-3.5">
      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">
            {msg.settingsPanel.spokenNormalizationTitle}
          </p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.spokenNormalizationHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.spoken_normalization_enabled ?? true,
          msg.settingsPanel.spokenNormalizationAria,
          () => void dash.toggleSpokenNormalization(),
        )}
      </div>

      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">
            {msg.settingsPanel.normalizeTitle}
          </p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.normalizeHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.dictation_normalize ?? true,
          msg.settingsPanel.normalizeAria,
          () => void dash.toggleDictationNormalize(),
        )}
        {#if dash.settings.dictation_normalize ?? true}
          <div
            class="mt-3 rounded-md border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2.5"
          >
            <p class="text-[11px] font-medium text-[var(--dash-text-muted)]">
              {msg.settingsPanel.normalizerModelTitle}
            </p>
            {#if dash.normalizerStatus?.model_downloaded}
              <p class="mt-1 text-xs text-emerald-300/90">
                {format(msg.settingsPanel.normalizerReady, {
                  model: dash.normalizerStatus.model_id,
                })}
              </p>
              {#if dash.normalizerStatus.legacy}
                <p class="mt-1 text-[11px] text-amber-300/80">
                  {msg.settingsPanel.normalizerLegacy}
                </p>
              {/if}
            {:else}
              <p class="mt-1 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
                {msg.settingsPanel.normalizerMissing}
              </p>
              <button
                type="button"
                class="mt-2 rounded-md border border-[var(--dash-border)] px-2.5 py-1 text-[11px] text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white disabled:opacity-50"
                onclick={() => void dash.runNormalizerInstall()}
                disabled={dash.normalizerInstalling}
              >
                {dash.normalizerInstalling
                  ? msg.settingsPanel.normalizerInstalling
                  : msg.settingsPanel.normalizerInstall}
              </button>
            {/if}
            {#if dash.normalizerError}
              <p class="mt-2 text-[11px] text-red-300/90">{dash.normalizerError}</p>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">
            {msg.settingsPanel.fillerTitle}
          </p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.fillerHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.strip_filler_words,
          msg.settingsPanel.fillerAria,
          () => {
            dash.settings.strip_filler_words = !dash.settings.strip_filler_words;
            void dash.saveSettings({ quiet: true });
          },
        )}
      </div>

      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">{msg.settingsPanel.soundsTitle}</p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.soundsHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.sound_effects_enabled,
          msg.settingsPanel.soundsAria,
          () => void dash.toggleSoundEffects(),
        )}
      </div>
    </div>
  </section>

  <section
    class="overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)]"
  >
    <div class="flex items-center justify-between gap-3 px-3.5 py-2.5">
      <span class="text-xs text-[var(--dash-text-muted)]"
        >{msg.settingsPanel.restoreClipboard}</span
      >
      {@render settingsSwitch(
        dash.settings.restore_clipboard_after_paste,
        msg.settingsPanel.restoreClipboard,
        () => {
          dash.settings.restore_clipboard_after_paste =
            !dash.settings.restore_clipboard_after_paste;
          dash.scheduleSave();
        },
      )}
    </div>
  </section>

  <section
    class="overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)]"
  >
    <header class="border-b border-[var(--dash-border)] bg-white/[0.02] px-3.5 py-2">
      <h2
        class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
      >
        {msg.nav.history}
      </h2>
    </header>
    <div class="px-3.5">
      <div class="flex items-center justify-between gap-3 py-2.5">
        <div class="min-w-0 flex-1">
          <p class="text-xs font-medium text-white">{msg.settingsPanel.showAsrRawTitle}</p>
          <p class="mt-0.5 text-[11px] leading-snug text-[var(--dash-text-subtle)]">
            {msg.settingsPanel.showAsrRawHint}
          </p>
        </div>
        {@render settingsSwitch(
          dash.settings.show_asr_raw_in_history,
          msg.settingsPanel.showAsrRawAria,
          () => void dash.toggleShowAsrRawInHistory(),
        )}
      </div>
    </div>
  </section>

  {#if dash.isSaving || dash.saveMessage}
    <p class="px-0.5 text-xs text-[var(--dash-text-muted)]" role="status">
      {#if dash.isSaving}
        {msg.settingsPanel.saving}
      {:else}
        <span class="text-emerald-400">{dash.saveMessage}</span>
      {/if}
    </p>
  {/if}
</div>
