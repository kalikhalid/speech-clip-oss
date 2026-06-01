<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    subtitle?: string;
    actions?: Snippet;
  }

  let { title, subtitle = "", actions }: Props = $props();
</script>

<header class="mb-8 flex items-start justify-between gap-4">
  <div class="min-w-0">
    <h1 class="text-2xl font-semibold tracking-tight text-white">{title}</h1>
    {#if subtitle}
      <p class="mt-1.5 text-sm text-[var(--dash-text-muted)]">{subtitle}</p>
    {/if}
  </div>
  {#if actions}
    <div class="shrink-0 pt-1">
      {@render actions()}
    </div>
  {/if}
</header>
