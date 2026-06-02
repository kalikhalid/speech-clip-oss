<script lang="ts">
  import { locale, messagesFor, type UiLocale } from "$lib/i18n";

  interface Props {
    value: UiLocale;
    onchange: (locale: UiLocale) => void;
  }

  let { value, onchange }: Props = $props();

  const msg = $derived(messagesFor($locale));

  const options: { id: UiLocale; code: string; label: string }[] = $derived([
    { id: "en", code: "EN", label: msg.locale.en },
    { id: "ru", code: "RU", label: msg.locale.ru },
  ]);
</script>

<section
  class="locale-picker overflow-hidden rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-5"
  aria-labelledby="locale-picker-title"
>
  <div class="flex items-start gap-3">
    <div
      class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-[var(--dash-accent)]/25 bg-[var(--dash-accent-muted)] text-[var(--dash-accent)]"
      aria-hidden="true"
    >
      <svg class="h-[18px] w-[18px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M12 21a9 9 0 100-18 9 9 0 000 18z"
        />
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="M3.6 9h16.8M3.6 15h16.8M12 3c2.2 2.4 3.4 5.5 3.4 9s-1.2 6.6-3.4 9c-2.2-2.4-3.4-5.5-3.4-9S9.8 5.4 12 3z"
        />
      </svg>
    </div>
    <div class="min-w-0 flex-1">
      <h2 id="locale-picker-title" class="text-sm font-semibold text-white">
        {msg.locale.label}
      </h2>
      <p class="mt-0.5 text-xs leading-relaxed text-[var(--dash-text-subtle)]">
        {msg.locale.hint}
      </p>
    </div>
  </div>

  <div
    class="mt-4 grid grid-cols-2 gap-2"
    role="radiogroup"
    aria-labelledby="locale-picker-title"
  >
    {#each options as opt (opt.id)}
      {@const selected = value === opt.id}
      <button
        type="button"
        role="radio"
        aria-checked={selected}
        class="locale-option group relative flex flex-col items-start gap-2 rounded-lg border px-3.5 py-3 text-left transition focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50 {selected
          ? 'border-[var(--dash-accent)]/45 bg-[var(--dash-accent-muted)] shadow-[inset_0_1px_0_rgba(255,255,255,0.06)]'
          : 'border-[var(--dash-border)] bg-[var(--dash-bg)] hover:border-white/18 hover:bg-white/[0.03]'}"
        onclick={() => onchange(opt.id)}
      >
        <div class="flex w-full items-center justify-between gap-2">
          <span
            class="rounded-md border px-1.5 py-0.5 font-mono text-[10px] font-semibold tracking-wide transition {selected
              ? 'border-[var(--dash-accent)]/35 bg-[var(--dash-accent)]/15 text-[#ffb899]'
              : 'border-white/10 bg-white/[0.04] text-[var(--dash-text-subtle)] group-hover:text-[var(--dash-text-muted)]'}"
          >
            {opt.code}
          </span>
          {#if selected}
            <span
              class="flex h-5 w-5 items-center justify-center rounded-full bg-[var(--dash-accent)] text-white"
              aria-hidden="true"
            >
              <svg class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
                <path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
              </svg>
            </span>
          {:else}
            <span
              class="h-5 w-5 rounded-full border border-white/12 bg-transparent transition group-hover:border-white/20"
              aria-hidden="true"
            ></span>
          {/if}
        </div>
        <span
          class="text-sm font-medium leading-tight transition {selected
            ? 'text-[#ffb899]'
            : 'text-[var(--dash-text-muted)] group-hover:text-[var(--dash-text)]'}"
        >
          {opt.label}
        </span>
      </button>
    {/each}
  </div>
</section>
