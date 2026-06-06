<script lang="ts">
  import { locale, messagesFor } from "$lib/i18n";
  import { useDashboard } from "../dashboard-context";

  const dash = useDashboard();
  const msg = $derived(messagesFor($locale));
</script>

<section class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5">
  {#if dash.historyEntries.length > 0}
    <div class="mb-4 flex justify-end">
      <button
        type="button"
        class="text-xs text-[var(--dash-text-subtle)] transition hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
        onclick={() => void dash.clearHistory()}
      >
        {msg.history.clearAll}
      </button>
    </div>
  {/if}

  {#if dash.filteredHistory.length === 0}
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
        {dash.historyEntries.length === 0
          ? msg.history.empty
          : msg.history.emptyPeriod}
      </p>
      <p class="mt-1 max-w-sm text-xs text-[var(--dash-text-subtle)]">
        {msg.history.emptyHint}
      </p>
    </div>
  {:else}
    <ul class="dashboard-selectable space-y-3">
      {#each dash.filteredHistory as entry (entry.id)}
        {@const timing = dash.formatTimingDetail(entry)}
        <li
          class="group rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] p-4 transition hover:border-[var(--dash-accent)]/25"
        >
          <div class="flex items-start justify-between gap-3">
            <div
              class="flex flex-wrap items-center gap-2 text-xs text-[var(--dash-text-subtle)]"
            >
              <time datetime={new Date(entry.timestamp).toISOString()}>
                {dash.relativeTime(entry.timestamp)}
              </time>
              {#if entry.app_name}
                <span
                  class="rounded-md border border-white/10 bg-white/[0.03] px-1.5 py-0.5 text-[var(--dash-text-muted)]"
                >
                  {entry.app_name}
                </span>
              {/if}
              <span>{dash.formatDuration(entry.duration_ms)}</span>
              {#if timing}
                <span class="text-[var(--dash-text-subtle)]">{timing}</span>
              {/if}
            </div>
            <div
              class="flex shrink-0 gap-1 opacity-0 transition group-hover:opacity-100 focus-within:opacity-100"
            >
              <button
                type="button"
                class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                onclick={() => void dash.pasteHistoryEntry(entry.normalized_text)}
              >
                {msg.history.pasteAgain}
              </button>
              <button
                type="button"
                class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
                onclick={() => dash.startDictionaryFromHistory(entry.normalized_text)}
              >
                {msg.history.dictionary}
              </button>
              <button
                type="button"
                class="rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] hover:bg-white/5 hover:text-red-400 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
                onclick={() => void dash.deleteEntry(entry.id)}
                aria-label={msg.history.deleteAria}
              >
                {msg.dictionary.delete}
              </button>
            </div>
          </div>
          {#if dash.settings.show_asr_raw_in_history}
            {@const rawText = entry.raw_text ?? ""}
            {@const hasRaw = rawText.length > 0}
            {@const changed = hasRaw && rawText !== entry.normalized_text}
            <div class="mt-2 space-y-2">
              <div>
                <p
                  class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
                >
                  {msg.history.outputLabel}
                </p>
                <p class="mt-0.5 text-sm leading-relaxed text-[var(--dash-text)]">
                  {entry.normalized_text}
                </p>
              </div>
              {#if hasRaw}
                <div>
                  <p
                    class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-subtle)]"
                  >
                    {msg.history.asrRawLabel}
                  </p>
                  <p
                    class="mt-0.5 font-mono text-sm leading-relaxed text-[var(--dash-text-muted)]"
                  >
                    {rawText}
                  </p>
                </div>
              {/if}
              {#if !hasRaw}
                <p class="text-[11px] text-[var(--dash-text-subtle)]">
                  {msg.history.asrRawMissing}
                </p>
              {:else if !changed}
                <p class="text-[11px] text-[var(--dash-text-subtle)]">
                  {msg.history.asrUnchanged}
                </p>
              {/if}
            </div>
          {:else}
            <p class="mt-2 text-sm leading-relaxed text-[var(--dash-text)]">
              {entry.normalized_text}
            </p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
