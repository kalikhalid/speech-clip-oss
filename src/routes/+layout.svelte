<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { initLocaleFromSettings } from "$lib/i18n";

  let i18nReady = $state(false);

  onMount(async () => {
    await initLocaleFromSettings();
    i18nReady = true;
  });
</script>

{#if i18nReady}
  <slot />
{/if}
