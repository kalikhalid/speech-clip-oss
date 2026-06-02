import { get, writable } from "svelte/store";
import { en, type Messages } from "./en";
import { ru } from "./ru";

export type UiLocale = "en" | "ru";

const catalogs: Record<UiLocale, Messages> = { en, ru };

export const locale = writable<UiLocale>("en");

export function normalizeLocale(value: string | undefined | null): UiLocale {
  return value === "ru" ? "ru" : "en";
}

export function messagesFor(loc: UiLocale): Messages {
  return catalogs[loc];
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

export function t(
  key: string,
  params?: Record<string, string | number>,
  loc?: UiLocale,
): string {
  const messages = messagesFor(loc ?? get(locale));
  const parts = key.split(".");
  let node: unknown = messages;
  for (const part of parts) {
    if (node && typeof node === "object" && part in (node as object)) {
      node = (node as Record<string, unknown>)[part];
    } else {
      return key;
    }
  }
  if (typeof node !== "string") return key;
  return format(node, params);
}

export function applyDocumentLocale(loc: UiLocale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = loc === "ru" ? "ru" : "en";
  }
}

export function setLocale(loc: UiLocale) {
  locale.set(loc);
  applyDocumentLocale(loc);
}

export async function initLocaleFromSettings(): Promise<UiLocale> {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const settings = await invoke<{ ui_locale?: string }>("get_settings");
    const loc = normalizeLocale(settings.ui_locale);
    setLocale(loc);
    return loc;
  } catch {
    setLocale("en");
    return "en";
  }
}
