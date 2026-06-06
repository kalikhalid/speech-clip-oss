<script lang="ts">
  import { locale, messagesFor } from "$lib/i18n";
  import { useDashboard } from "../dashboard-context";

  const dash = useDashboard();
  const msg = $derived(messagesFor($locale));
</script>

<section
  class="relative mb-6 overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-6"
  aria-live="polite"
>
  <div
    class="pointer-events-none absolute -right-8 -top-8 h-32 w-32 rounded-full bg-[var(--dash-brand-orange)]/[0.06] blur-2xl"
  ></div>

  <div class="flex items-start gap-4">
    {#if dash.hero === "loading"}
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
    {:else if dash.hero === "ready"}
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
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
        </svg>
      </div>
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold text-white">{msg.hero.readyTitle}</h2>
        <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
          {#if dash.settings.recording_mode === "toggle"}
            {msg.hero.toggleHintBefore}
            <kbd
              class="mx-1 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[var(--dash-text)]"
              >{dash.settings.hotkey}</kbd
            >
            {msg.hero.toggleHintAfter}
          {:else}
            {msg.hero.holdHintBefore}
            <kbd
              class="mx-1 rounded border border-white/15 bg-white/5 px-1.5 py-0.5 font-mono text-xs text-[var(--dash-text)]"
              >{dash.settings.hotkey}</kbd
            >
            {msg.hero.holdHintAfter}
          {/if}
        </p>
      </div>
    {:else if dash.hero === "setting-up"}
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
          {dash.installProgress?.message ??
            dash.parakeetStatus?.message ??
            msg.hero.settingUpFallback}
        </p>
      </div>
    {:else if dash.hero === "error"}
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
          {dash.errorInfo?.title ?? msg.hero.setupFailed}
        </h2>
        <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
          {dash.errorInfo?.hint ?? msg.hero.tryAgain}
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
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
      </div>
      <div class="min-w-0 flex-1">
        <h2 class="text-lg font-semibold text-white">{msg.hero.finishTitle}</h2>
        <p class="mt-1 text-sm text-[var(--dash-text-muted)]">
          {dash.parakeetStatus?.message ?? msg.hero.finishFallback}
        </p>
      </div>
    {/if}
  </div>

  {#if dash.showSetupProgress}
    <div class="mt-5">
      <div
        class="h-1.5 overflow-hidden rounded-full bg-white/10"
        role="progressbar"
        aria-valuenow={dash.progressPercent}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div
          class="h-full rounded-full bg-gradient-to-r from-[var(--dash-accent)] to-[#ff7a33] transition-[width] duration-300 ease-out"
          style="width: {Math.max(dash.progressPercent, dash.hero === 'setting-up' ? 4 : 0)}%"
        ></div>
      </div>

      <ol class="mt-4 grid gap-2 sm:grid-cols-2">
        {#each dash.setupStages as stage}
          {@const done = dash.isStageComplete(stage.id)}
          {@const active = dash.isStageActive(stage.id)}
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

  {#if dash.installError}
    <div class="mt-4 rounded-lg border border-red-500/20 bg-red-500/5 p-3">
      <button
        type="button"
        class="text-xs font-medium text-red-300/90 underline-offset-2 hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/50"
        onclick={() => (dash.showErrorDetails = !dash.showErrorDetails)}
        aria-expanded={dash.showErrorDetails}
      >
        {dash.showErrorDetails ? msg.errors.hideDetails : msg.errors.showDetails}
      </button>
      {#if dash.showErrorDetails}
        <pre
          class="dashboard-selectable mt-2 max-h-32 overflow-auto whitespace-pre-wrap break-words font-mono text-[11px] leading-relaxed text-red-200/70"
          >{dash.installError}</pre
        >
      {/if}
    </div>
  {/if}

  <div class="mt-5 flex flex-wrap gap-2">
    {#if dash.hero !== "ready" && dash.hero !== "loading"}
      <button
        type="button"
        class="rounded-md bg-[var(--dash-accent)] px-4 py-2 text-sm font-medium text-white transition hover:bg-[var(--dash-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/60 disabled:cursor-not-allowed disabled:opacity-50"
        onclick={() => void dash.runInstall()}
        disabled={dash.installing}
      >
        {dash.installing ? msg.hero.installing : msg.hero.runSetup}
      </button>
    {/if}
    <button
      type="button"
      class="rounded-md border border-[var(--dash-border)] bg-transparent px-4 py-2 text-sm font-medium text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-white/30 disabled:opacity-50"
      onclick={() => void dash.refreshStatus()}
      disabled={dash.installing}
    >
      {msg.hero.refresh}
    </button>
  </div>
</section>

<section
  class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
  aria-labelledby="dictation-stats-title"
>
  <div class="mb-4">
    <h3
      id="dictation-stats-title"
      class="text-sm font-semibold text-white"
    >
      {msg.stats.title}
    </h3>
    <p class="mt-1 text-xs text-[var(--dash-text-subtle)]">
      {msg.stats.subtitle}
    </p>
  </div>

  {#if dash.statsLoading}
    <p class="text-sm text-[var(--dash-text-muted)]" role="status">
      {msg.stats.loading}
    </p>
  {:else if dash.dictationStats}
    <div class="grid gap-3 sm:grid-cols-3">
      <div
        class="rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-4 py-3.5"
      >
        <p
          class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
        >
          {msg.stats.last24h}
        </p>
        <p class="mt-2 text-2xl font-semibold tabular-nums tracking-tight text-white">
          {dash.formatWordCount(dash.dictationStats.words_24h)}
        </p>
        <p class="mt-1 text-xs text-[var(--dash-text-muted)]">{msg.stats.wordsLabel}</p>
      </div>
      <div
        class="rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-4 py-3.5"
      >
        <p
          class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
        >
          {msg.stats.last7d}
        </p>
        <p class="mt-2 text-2xl font-semibold tabular-nums tracking-tight text-white">
          {dash.formatWordCount(dash.dictationStats.words_7d)}
        </p>
        <p class="mt-1 text-xs text-[var(--dash-text-muted)]">{msg.stats.wordsLabel}</p>
      </div>
      <div
        class="rounded-lg border border-[var(--dash-accent)]/25 bg-[var(--dash-accent-muted)] px-4 py-3.5"
      >
        <p
          class="text-[10px] font-medium uppercase tracking-wider text-[#ffb899]/80"
        >
          {msg.stats.allTime}
        </p>
        <p class="mt-2 text-2xl font-semibold tabular-nums tracking-tight text-white">
          {dash.formatWordCount(dash.dictationStats.words_all_time)}
        </p>
        <p class="mt-1 text-xs text-[var(--dash-text-muted)]">{msg.stats.wordsLabel}</p>
      </div>
    </div>
  {/if}
</section>
