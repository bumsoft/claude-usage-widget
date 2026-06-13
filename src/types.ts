// Mirrors the Rust types returned by the Tauri commands.

export interface UsageWindow {
  utilization: number;
  resets_at: string | null;
}

export interface ExtraUsage {
  is_enabled: boolean;
  monthly_limit: number | null;
  used_credits: number | null;
  utilization: number | null;
  currency: string | null;
}

export interface UsageSnapshot {
  plan: string;
  subscriptionType: string | null;
  rateLimitTier: string | null;
  fiveHour: UsageWindow | null;
  sevenDay: UsageWindow | null;
  sevenDayOpus: UsageWindow | null;
  sevenDaySonnet: UsageWindow | null;
  extraUsage: ExtraUsage | null;
  fetchedAtMs: number;
}

export type SourceKind = "windows" | "wsl" | "custom";

export interface Source {
  id: string;
  label: string;
  kind: SourceKind;
  credentialsPath: string;
  exists: boolean;
}

export interface DailyTokens {
  date: string;
  tokensByModel: Record<string, number>;
  costUsd: number;
}

export interface ModelUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens: number;
  cacheCreationInputTokens: number;
}

export interface StatsHistory {
  days: DailyTokens[];
  models: string[];
  modelUsage: Record<string, ModelUsage>;
  totalSessions: number;
  totalMessages: number;
  costUsd: number;
}

export type CompactStyle = "bars" | "rings";

export interface AppConfig {
  selectedSourceId: string | null;
  customPaths: string[];
  refreshSeconds: number;
  alwaysOnTop: boolean;
  compact: boolean;
  compactStyle: CompactStyle;
}

export type StatusKind = "loading" | "ok" | "error" | "unauthorized" | "no-source";

export interface Status {
  kind: StatusKind;
  message?: string;
}
