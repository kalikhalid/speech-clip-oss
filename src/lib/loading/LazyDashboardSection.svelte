<script lang="ts">
  import type { Component } from "svelte";
  import type { DashboardSection } from "$lib/components/dashboard/types";
  import { locale, messagesFor } from "$lib/i18n";
  import { loadDashboardSection } from "./dashboard-sections";

  let { section }: { section: DashboardSection } = $props();

  const msg = $derived(messagesFor($locale));

  let SectionComponent = $state<Component | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const active = section;
    SectionComponent = null;
    loadError = null;

    loadDashboardSection(active)
      .then((component) => {
        if (section === active) {
          SectionComponent = component;
        }
      })
      .catch((error) => {
        if (section === active) {
          loadError = String(error);
        }
      });
  });
</script>

{#if loadError}
  <p class="rounded-lg border border-red-500/20 bg-red-500/5 p-4 text-sm text-red-300">
    {loadError}
  </p>
{:else if SectionComponent}
  <SectionComponent />
{:else}
  <div
    class="flex items-center gap-3 rounded-[10px] border border-[var(--dash-border)] bg-[var(--dash-bg-card)] p-6 text-sm text-[var(--dash-text-muted)]"
    role="status"
    aria-busy="true"
  >
    <div
      class="h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-white/20 border-t-[var(--dash-accent)]"
      aria-hidden="true"
    ></div>
    <span>{msg.sections.panelLoading}</span>
  </div>
{/if}
