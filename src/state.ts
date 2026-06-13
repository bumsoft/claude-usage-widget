// Central UI state shape.

import type { AppConfig, Source, StatsHistory, Status, UsageSnapshot } from "./types";

export interface UiState {
  sources: Source[];
  selectedId: string | null;
  usage: UsageSnapshot | null;
  stats: StatsHistory | null;
  config: AppConfig;
  status: Status;
  settingsOpen: boolean;
  lastUpdatedMs: number | null;
}

export function initialState(config: AppConfig): UiState {
  return {
    sources: [],
    selectedId: config.selectedSourceId,
    usage: null,
    stats: null,
    config,
    status: { kind: "loading" },
    settingsOpen: false,
    lastUpdatedMs: null,
  };
}

export function selectedSource(state: UiState): Source | undefined {
  return state.sources.find((s) => s.id === state.selectedId);
}
