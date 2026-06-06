<script lang="ts">
  import type { DashboardSection } from "./types";
  import { locale, messagesFor } from "$lib/i18n";
  import { preloadDashboardSection } from "$lib/loading/dashboard-sections";

  type NavItem = {
    id: DashboardSection;
    label: string;
    icon: "general" | "dictionary" | "history" | "settings";
  };

  interface Props {
    active: DashboardSection;
    historyCount?: number;
    onNavigate: (section: DashboardSection) => void;
  }

  let { active, historyCount = 0, onNavigate }: Props = $props();

  const msg = $derived(messagesFor($locale));

  const NAV = $derived<NavItem[]>([
    { id: "general", label: msg.nav.general, icon: "general" },
    { id: "dictionary", label: msg.nav.dictionary, icon: "dictionary" },
    { id: "history", label: msg.nav.history, icon: "history" },
    { id: "settings", label: msg.nav.settings, icon: "settings" },
  ]);
</script>

<aside
  class="dashboard-sidebar flex w-[188px] shrink-0 flex-col border-r border-[var(--dash-border)] bg-[var(--dash-sidebar-bg)] pb-4"
  aria-label={msg.nav.aria}
>
  <div class="dashboard-sidebar__header mb-6 flex items-center gap-2.5 px-4">
    <img
      src="/logo.png"
      alt=""
      width="32"
      height="32"
      class="size-8 shrink-0 rounded-[22%] ring-1 ring-[var(--dash-brand-orange)]/30"
    />
    <div class="min-w-0">
      <span class="block truncate text-sm font-semibold tracking-tight text-white"
        >Speech Clip</span
      >
      <span
        class="text-[10px] font-medium uppercase tracking-wider text-[var(--dash-text-muted)]"
        >OSS</span
      >
    </div>
  </div>

  <nav class="flex flex-1 flex-col gap-0.5 px-2">
    {#each NAV as item}
      {@const isActive = active === item.id}
      <button
        type="button"
        class="group relative flex w-full items-center gap-2.5 rounded-lg px-2 py-2 text-left text-[13px] font-medium transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--dash-accent)]/50 {isActive
          ? 'bg-white/[0.04] text-white'
          : 'text-[var(--dash-text-muted)] hover:bg-white/[0.03] hover:text-[var(--dash-text)]'}"
        aria-current={isActive ? "page" : undefined}
        onmouseenter={() => preloadDashboardSection(item.id)}
        onclick={() => onNavigate(item.id)}
      >
        {#if isActive}
          <span
            class="absolute top-1.5 bottom-1.5 left-0 w-0.5 rounded-full bg-[var(--dash-accent)]"
            aria-hidden="true"
          ></span>
        {/if}
        <span
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border transition-colors {isActive
            ? 'border-[var(--dash-accent)]/25 bg-[var(--dash-accent-muted)] text-[var(--dash-accent)]'
            : 'border-white/8 bg-white/[0.03] text-[var(--dash-text-muted)] group-hover:border-white/12 group-hover:text-[var(--dash-text)]'}"
          aria-hidden="true"
        >
          {#if item.icon === "general"}
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
              <path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h10" />
            </svg>
          {:else if item.icon === "dictionary"}
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
            </svg>
          {:else if item.icon === "history"}
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          {:else}
            <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75">
              <path stroke-linecap="round" stroke-linejoin="round" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          {/if}
        </span>
        <span class="min-w-0 flex-1 truncate">{item.label}</span>
        {#if item.id === "history" && historyCount > 0}
          <span
            class="rounded-md bg-white/10 px-1.5 py-0.5 text-[10px] font-normal tabular-nums text-[var(--dash-text-muted)]"
          >
            {historyCount}
          </span>
        {/if}
      </button>
    {/each}
  </nav>
</aside>
