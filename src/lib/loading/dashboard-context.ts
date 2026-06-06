import { getContext, setContext } from "svelte";
import type { DashboardContext } from "./dashboard-types";

const DASHBOARD_CONTEXT_KEY = Symbol("speech-clip-dashboard");

export function setDashboardContext(ctx: DashboardContext): void {
  setContext(DASHBOARD_CONTEXT_KEY, ctx);
}

export function useDashboard(): DashboardContext {
  return getContext(DASHBOARD_CONTEXT_KEY);
}
