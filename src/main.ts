import "./styles.css";

import * as api from "./api";
import { ago, formatCountdown } from "./format";
import { initialState, selectedSource, type UiState } from "./state";
import type { AppConfig, CompactStyle, Source } from "./types";
import { renderApp } from "./ui/app";

const STATS_REFRESH_MS = 5 * 60 * 1000;
const MIN_REFRESH = 15;
const MAX_REFRESH = 3600;

const SIZE_EXPANDED = { w: 300, h: 432 };
const SIZE_COMPACT: Record<CompactStyle, { w: number; h: number }> = {
  bars: { w: 240, h: 86 },
  rings: { w: 200, h: 124 },
};

const root = document.getElementById("app") as HTMLElement;

let state: UiState;
let usageTimer: number | undefined;
let statsTimer: number | undefined;

function render(): void {
  root.innerHTML = renderApp(state);
}

function setState(patch: Partial<UiState>): void {
  state = { ...state, ...patch };
  render();
}

// --- data loading ---------------------------------------------------------

function pickSelected(sources: Source[], preferredId: string | null): string | null {
  if (preferredId && sources.some((s) => s.id === preferredId)) return preferredId;
  const existing = sources.find((s) => s.exists);
  if (existing) return existing.id;
  return sources[0]?.id ?? null;
}

async function refreshSources(): Promise<void> {
  try {
    const sources = await api.discoverSources();
    const selectedId = pickSelected(sources, state.selectedId ?? state.config.selectedSourceId);
    setState({ sources, selectedId });
  } catch (e) {
    setState({ status: { kind: "error", message: errText(e) } });
  }
}

function errText(e: unknown): string {
  return typeof e === "string" ? e : String((e as { message?: string })?.message ?? e);
}

function classify(e: unknown): "unauthorized" | "error" {
  return /unauthor|expired|401|403/i.test(errText(e)) ? "unauthorized" : "error";
}

async function refreshUsage(): Promise<void> {
  const src = selectedSource(state);
  if (!src) {
    setState({ status: { kind: "no-source" } });
    return;
  }
  try {
    const usage = await api.fetchUsage(src.credentialsPath);
    setState({ usage, status: { kind: "ok" }, lastUpdatedMs: Date.now() });
  } catch (e) {
    const kind = classify(e);
    setState({ status: { kind, message: kind === "error" ? errText(e) : undefined } });
  }
}

async function refreshStats(): Promise<void> {
  const src = selectedSource(state);
  if (!src) return;
  try {
    const stats = await api.readStats(src.credentialsPath, 14);
    setState({ stats });
  } catch (e) {
    // Local history is optional; keep whatever we had.
    console.debug("stats unavailable:", e);
  }
}

// --- scheduling -----------------------------------------------------------

function scheduleUsage(): void {
  if (usageTimer) clearInterval(usageTimer);
  const seconds = Math.min(MAX_REFRESH, Math.max(MIN_REFRESH, state.config.refreshSeconds));
  usageTimer = window.setInterval(refreshUsage, seconds * 1000);
}

function scheduleStats(): void {
  if (statsTimer) clearInterval(statsTimer);
  statsTimer = window.setInterval(refreshStats, STATS_REFRESH_MS);
}

function startClock(): void {
  window.setInterval(() => {
    const now = Date.now();
    document.querySelectorAll<HTMLElement>(".reset[data-reset]").forEach((el) => {
      const iso = el.getAttribute("data-reset");
      if (iso) el.textContent = `resets in ${formatCountdown(iso, now)}`;
    });
    const statusText = document.querySelector<HTMLElement>(".status-text");
    if (statusText && state.status.kind === "ok" && state.lastUpdatedMs) {
      statusText.textContent = `updated ${ago(now - state.lastUpdatedMs)}`;
    }
  }, 1000);
}

// --- actions --------------------------------------------------------------

async function selectSource(id: string): Promise<void> {
  const config: AppConfig = { ...state.config, selectedSourceId: id };
  setState({ selectedId: id, config, usage: null, stats: null, status: { kind: "loading" } });
  await api.setConfig(config).catch(() => {});
  await Promise.all([refreshUsage(), refreshStats()]);
  scheduleUsage(); // realign the poll interval to this fresh fetch
}

async function setRefresh(value: number): Promise<void> {
  const refreshSeconds = Math.min(MAX_REFRESH, Math.max(MIN_REFRESH, Math.floor(value || 60)));
  const config: AppConfig = { ...state.config, refreshSeconds };
  setState({ config });
  await api.setConfig(config).catch(() => {});
  scheduleUsage();
}

async function setAot(value: boolean): Promise<void> {
  const config: AppConfig = { ...state.config, alwaysOnTop: value };
  setState({ config });
  await api.setConfig(config).catch(() => {});
  await api.setAlwaysOnTop(value).catch(() => {});
}

async function applyWindowSize(): Promise<void> {
  const s = state.config.compact ? SIZE_COMPACT[state.config.compactStyle] : SIZE_EXPANDED;
  await api.setWindowSize(s.w, s.h).catch(() => {});
}

async function toggleCompact(): Promise<void> {
  const config: AppConfig = { ...state.config, compact: !state.config.compact };
  setState({ config, settingsOpen: false });
  await api.setConfig(config).catch(() => {});
  await applyWindowSize();
}

async function setCompactStyle(style: CompactStyle): Promise<void> {
  const config: AppConfig = { ...state.config, compactStyle: style };
  setState({ config });
  await api.setConfig(config).catch(() => {});
  if (config.compact) await applyWindowSize();
}

async function addCustom(): Promise<void> {
  let path: string | null = null;
  try {
    path = await api.browseCredentials();
  } catch {
    return;
  }
  if (!path || state.config.customPaths.includes(path)) return;
  const config: AppConfig = {
    ...state.config,
    customPaths: [...state.config.customPaths, path],
  };
  setState({ config });
  await api.setConfig(config).catch(() => {});
  await refreshSources();
  const added = state.sources.find(
    (s) => s.credentialsPath.toLowerCase() === path!.toLowerCase(),
  );
  if (added) await selectSource(added.id);
}

async function removeCustom(index: number): Promise<void> {
  const customPaths = state.config.customPaths.filter((_, i) => i !== index);
  const config: AppConfig = { ...state.config, customPaths };
  setState({ config });
  await api.setConfig(config).catch(() => {});
  await refreshSources();
}

// --- events ---------------------------------------------------------------

function onClick(e: MouseEvent): void {
  const el = (e.target as HTMLElement).closest<HTMLElement>("[data-action]");
  if (!el) return;
  switch (el.getAttribute("data-action")) {
    case "toggle-settings":
      setState({ settingsOpen: !state.settingsOpen });
      break;
    case "hide-window":
      api.hideWindow().catch(() => {});
      break;
    case "toggle-compact":
      void toggleCompact();
      break;
    case "refresh":
      void refreshUsage();
      void refreshStats();
      break;
    case "add-custom":
      void addCustom();
      break;
    case "remove-custom":
      void removeCustom(Number(el.getAttribute("data-index")));
      break;
  }
}

// Start a window drag from anywhere except interactive controls, so the whole
// widget surface is grabbable.
const NO_DRAG = "button, a, select, input, textarea, .settings, [data-no-drag]";

function onMouseDown(e: MouseEvent): void {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest(NO_DRAG)) return;
  void api.startDragging();
}

function onChange(e: Event): void {
  const el = e.target as HTMLElement;
  switch (el.getAttribute("data-action")) {
    case "select-source":
      void selectSource((el as HTMLSelectElement | HTMLInputElement).value);
      break;
    case "set-refresh":
      void setRefresh(Number((el as HTMLInputElement).value));
      break;
    case "toggle-aot":
      void setAot((el as HTMLInputElement).checked);
      break;
    case "set-compact-style":
      void setCompactStyle((el as HTMLInputElement).value as CompactStyle);
      break;
  }
}

// --- bootstrap ------------------------------------------------------------

async function init(): Promise<void> {
  let config: AppConfig;
  try {
    config = await api.getConfig();
  } catch {
    config = {
      selectedSourceId: null,
      customPaths: [],
      refreshSeconds: 60,
      alwaysOnTop: true,
      compact: false,
      compactStyle: "bars",
    };
  }
  state = initialState(config);
  render();

  root.addEventListener("click", onClick);
  root.addEventListener("change", onChange);
  root.addEventListener("mousedown", onMouseDown);
  startClock();

  await refreshSources();
  await Promise.all([refreshUsage(), refreshStats()]);
  scheduleUsage(); // start intervals from a known-good first fetch
  scheduleStats();
  await api.setAlwaysOnTop(config.alwaysOnTop).catch(() => {});
  await applyWindowSize(); // restore compact/expanded size from last session
}

void init();
