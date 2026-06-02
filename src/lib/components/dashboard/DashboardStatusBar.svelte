<script lang="ts">
  import { locale, format, messagesFor } from "$lib/i18n";

  interface Props {
    modelLabel: string;
    ready: boolean;
    installing?: boolean;
    version?: string;
  }

  let {
    modelLabel,
    ready,
    installing = false,
    version = "0.1.0",
  }: Props = $props();

  const msg = $derived(messagesFor($locale));
</script>

<footer
  class="dashboard-statusbar flex shrink-0 items-center justify-between gap-4 border-t border-[var(--dash-border)] bg-[var(--dash-bg-elevated)] px-5 py-2.5 text-xs"
>
  <div class="flex min-w-0 items-center gap-2.5">
    <span
      class="relative flex h-2 w-2 shrink-0"
      aria-hidden="true"
    >
      {#if installing}
        <span
          class="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--dash-accent)] opacity-40"
        ></span>
        <span
          class="relative inline-flex h-2 w-2 rounded-full bg-[var(--dash-accent)]"
        ></span>
      {:else if ready}
        <span
          class="relative inline-flex h-2 w-2 rounded-full bg-emerald-400"
        ></span>
      {:else}
        <span
          class="relative inline-flex h-2 w-2 rounded-full bg-[var(--dash-text-subtle)]"
        ></span>
      {/if}
    </span>
    <span class="truncate text-[var(--dash-text-muted)]">
      {#if installing}
        {msg.statusBar.settingUp}
      {:else if ready}
        {format(msg.statusBar.active, { model: modelLabel })}
      {:else}
        {msg.statusBar.noModel}
      {/if}
    </span>
  </div>
  <a
    href="https://github.com/cjpais/transcribe-rs"
    target="_blank"
    rel="noopener noreferrer"
    class="shrink-0 text-[var(--dash-text-subtle)] underline-offset-2 transition hover:text-[var(--dash-accent)] focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/40"
  >
    {format(msg.statusBar.version, { version })}
  </a>
</footer>
