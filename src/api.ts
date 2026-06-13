// Typed wrappers around the Tauri command + plugin surface.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";

import type { AppConfig, Source, StatsHistory, UsageSnapshot } from "./types";

export const discoverSources = () => invoke<Source[]>("discover_sources");

export const fetchUsage = (path: string) =>
  invoke<UsageSnapshot>("fetch_usage", { path });

export const readStats = (path: string, days = 14) =>
  invoke<StatsHistory>("read_stats", { path, days });

export const getConfig = () => invoke<AppConfig>("get_config");

export const setConfig = (config: AppConfig) =>
  invoke<void>("set_config", { config });

/** Native file picker for adding a custom `.credentials.json` path. */
export async function browseCredentials(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    title: "Select .credentials.json",
    filters: [{ name: "credentials", extensions: ["json"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function hideWindow(): Promise<void> {
  await getCurrentWindow().hide();
}

export async function setAlwaysOnTop(value: boolean): Promise<void> {
  await getCurrentWindow().setAlwaysOnTop(value);
}

export async function setWindowSize(width: number, height: number): Promise<void> {
  await getCurrentWindow().setSize(new LogicalSize(width, height));
}

export async function startDragging(): Promise<void> {
  await getCurrentWindow().startDragging();
}
