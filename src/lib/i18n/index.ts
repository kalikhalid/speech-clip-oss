import { writable } from "svelte/store";
import type { Messages } from "./en";

export type UiLocale = "en" | "ru";

const catalogCache: Partial<Record<UiLocale, Messages>> = {};

export const locale = writable<UiLocale>("en");

async function loadCatalog(loc: UiLocale): Promise<Messages> {
  const cached = catalogCache[loc];
  if (cached) return cached;

  if (loc === "ru") {
    const { ru } = await import("./ru");
    catalogCache.ru = ru;
    return ru;
  }
  const { en } = await import("./en");
  catalogCache.en = en;
  return en;
}

export function messagesFor(loc: UiLocale): Messages {
  const cached = catalogCache[loc] ?? catalogCache.en;
  if (!cached) {
    throw new Error(`Locale catalog not loaded: ${loc}`);
  }
  return cached;
}

export function normalizeLocale(value: string | undefined | null): UiLocale {
  return value === "ru" ? "ru" : "en";
}

/** Replace `{name}` placeholders in a template string. */
export function format(
  template: string,
  params?: Record<string, string | number>,
): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, key: string) => {
    const v = params[key];
    return v === undefined ? `{${key}}` : String(v);
  });
}

export function applyDocumentLocale(loc: UiLocale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = loc === "ru" ? "ru" : "en";
  }
}

export async function setLocale(loc: UiLocale) {
  await loadCatalog(loc);
  locale.set(loc);
  applyDocumentLocale(loc);
}

let settingsFetchPromise: Promise<{ ui_locale?: string }> | null = null;

/** Single-flight `get_settings` for app boot (layout + dashboard). */
export function fetchSettingsOnce<T extends { ui_locale?: string }>(): Promise<T> {
  if (!settingsFetchPromise) {
    settingsFetchPromise = import("@tauri-apps/api/core").then(({ invoke }) =>
      invoke<T>("get_settings"),
    );
  }
  return settingsFetchPromise as Promise<T>;
}

export async function initLocaleFromSettings(): Promise<UiLocale> {
  try {
    const settings = await fetchSettingsOnce<{ ui_locale?: string }>();
    const loc = normalizeLocale(settings.ui_locale);
    await setLocale(loc);
    return loc;
  } catch {
    await setLocale("en");
    return "en";
  }
}
