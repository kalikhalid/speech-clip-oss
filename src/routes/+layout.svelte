<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";
  import { initLocaleFromSettings } from "$lib/i18n";

  onMount(() => {
    void initLocaleFromSettings();
  });
</script>

<slot />
