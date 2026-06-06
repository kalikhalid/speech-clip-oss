<script lang="ts">
  import { locale, messagesFor } from "$lib/i18n";
  import { useDashboard } from "../dashboard-context";

  const dash = useDashboard();
  const msg = $derived(messagesFor($locale));

  let csvFileInput = $state<HTMLInputElement | undefined>();

  const seedStatus = $derived(
    dash.settings.seed_dictionary_enabled
      ? msg.dictionary.seedActive.replace(
          "{count}",
          String(dash.settings.seed_dictionary_count),
        )
      : msg.dictionary.seedInactive,
  );
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

<section class="rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5">
  <p class="text-sm text-[var(--dash-text-muted)]">
    {msg.dictionary.intro}
  </p>

  <div
    class="mt-5 flex items-center justify-between gap-3 rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3.5 py-3"
  >
    <div class="min-w-0 flex-1">
      <p class="text-sm font-medium text-white">{msg.dictionary.seedTitle}</p>
      <p class="mt-0.5 text-xs leading-snug text-[var(--dash-text-subtle)]">
        {msg.dictionary.seedHint}
      </p>
      <p class="mt-1.5 text-xs text-[var(--dash-text-muted)]" role="status">
        {seedStatus}
      </p>
    </div>
    {@render settingsSwitch(
      dash.settings.seed_dictionary_enabled,
      msg.dictionary.seedAria,
      () => void dash.toggleSeedDictionary(),
    )}
  </div>

  {#if dash.settings.dictionary.length > 0}
    <ul
      class="dashboard-selectable mt-5 space-y-2"
      aria-label={msg.dictionary.rulesAria}
    >
      {#each dash.settings.dictionary as entry, index (entry.from + entry.to)}
        <li
          class="group flex items-center gap-2 rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2.5 text-sm transition hover:border-white/12"
        >
          <span
            class="min-w-0 flex-1 truncate font-mono text-[var(--dash-text-muted)]"
            title={entry.from}
          >
            {entry.from}
          </span>
          <span class="shrink-0 text-[var(--dash-text-subtle)]" aria-hidden="true">→</span>
          <span
            class="min-w-0 flex-1 truncate font-mono text-[var(--dash-text)]"
            title={entry.to}
          >
            {entry.to}
          </span>
          <button
            type="button"
            class="shrink-0 rounded-md px-2 py-1 text-xs text-[var(--dash-text-subtle)] opacity-0 transition group-hover:opacity-100 hover:bg-white/5 hover:text-red-400 focus:opacity-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-red-400/40"
            onclick={() => void dash.removeDictionaryEntry(index)}
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
        bind:value={dash.dictionaryFrom}
        placeholder={msg.dictionary.placeholderFrom}
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck={false}
        class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
      />
    </label>
    <label class="block">
      <span class="text-xs font-medium text-[var(--dash-text-muted)]"
        >{msg.dictionary.replaceWith}</span
      >
      <input
        type="text"
        bind:value={dash.dictionaryTo}
        placeholder={msg.dictionary.placeholderTo}
        autocomplete="off"
        autocorrect="off"
        autocapitalize="off"
        spellcheck={false}
        class="mt-1 w-full rounded-lg border border-[var(--dash-border)] bg-[var(--dash-bg)] px-3 py-2 font-mono text-sm text-white placeholder:text-[var(--dash-text-subtle)] focus:border-[var(--dash-accent)]/50 focus:outline-none focus:ring-2 focus:ring-[var(--dash-accent)]/20"
        onkeydown={(e) => {
          if (e.key === "Enter") void dash.addDictionaryEntry();
        }}
      />
    </label>
  </div>
  {#if dash.dictionaryError}
    <p class="mt-2 text-xs text-red-300">{dash.dictionaryError}</p>
  {/if}
  <div class="mt-3 flex flex-wrap items-center gap-2">
    <button
      type="button"
      class="rounded-md bg-[var(--dash-accent)] px-4 py-2 text-sm font-medium text-white transition hover:bg-[var(--dash-accent-hover)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50"
      onclick={() => void dash.addDictionaryEntry()}
    >
      {msg.dictionary.addRule}
    </button>
    <button
      type="button"
      class="rounded-md border border-[var(--dash-border)] px-3 py-2 text-sm text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
      onclick={() => void dash.exportDictionary()}
    >
      {msg.dictionary.exportCsv}
    </button>
    <button
      type="button"
      class="rounded-md border border-[var(--dash-border)] px-3 py-2 text-sm text-[var(--dash-text-muted)] transition hover:border-white/20 hover:text-white focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
      onclick={() => csvFileInput?.click()}
    >
      {msg.dictionary.importCsv}
    </button>
    <input
      bind:this={csvFileInput}
      type="file"
      accept=".csv,text/csv"
      class="hidden"
      onchange={(e) => void dash.onDictionaryFileSelected(e)}
    />
  </div>
  {#if dash.dictionaryImportMessage}
    <p class="mt-2 text-xs text-[var(--dash-text-muted)]" role="status">
      {dash.dictionaryImportMessage}
    </p>
  {/if}
</section>
