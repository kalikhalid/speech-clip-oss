import type { Component } from "svelte";
import type { DashboardSection } from "$lib/components/dashboard/types";

type SectionModule = { default: Component };

const loaders: Record<DashboardSection, () => Promise<SectionModule>> = {
  general: () => import("./sections/GeneralSection.svelte"),
  dictionary: () => import("./sections/DictionarySection.svelte"),
  history: () => import("./sections/HistorySection.svelte"),
  settings: () => import("./sections/SettingsSection.svelte"),
};

// Bump when section UI changes materially (invalidates lazy-load cache in dev).
const SECTIONS_CACHE_VERSION = 2;
const cache = new Map<DashboardSection, Component>();
let cacheVersion = 0;

export function preloadDashboardSection(section: DashboardSection): void {
  void loadDashboardSection(section);
}

export async function loadDashboardSection(
  section: DashboardSection,
): Promise<Component> {
  if (cacheVersion !== SECTIONS_CACHE_VERSION) {
    cache.clear();
    cacheVersion = SECTIONS_CACHE_VERSION;
  }

  const cached = cache.get(section);
  if (cached) return cached;

  const mod = await loaders[section]();
  cache.set(section, mod.default);
  return mod.default;
}
