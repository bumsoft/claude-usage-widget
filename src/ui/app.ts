// Builds the widget's HTML from UI state. Pure: no side effects, no events.

import { ago, escapeHtml, fmtPct, fmtUsd, formatCountdown, utilColor } from "../format";
import { selectedSource, type UiState } from "../state";
import type { ExtraUsage, Source, StatsHistory, UsageSnapshot, UsageWindow } from "../types";
import { historyChart } from "./chart";
import { miniBar, ringGauge } from "./gauge";

export function renderApp(state: UiState): string {
  const now = Date.now();
  if (state.config.compact) {
    return renderCompact(state);
  }
  return `
  <div class="widget${state.settingsOpen ? " with-settings" : ""}">
    ${renderTopbar(state)}
    <main class="body">${renderBody(state, now)}</main>
    ${renderStatusbar(state, now)}
    ${state.settingsOpen ? renderSettings(state) : ""}
  </div>`;
}

/** The small minimized layout: two metrics as bars or mini rings. */
function renderCompact(state: UiState): string {
  const u = state.usage;
  const style = state.config.compactStyle;
  const five = u?.fiveHour?.utilization ?? 0;
  const week = u?.sevenDay?.utilization ?? 0;

  const content =
    style === "rings"
      ? `<div class="crings">${compactRing("5h", five)}${compactRing("Wk", week)}</div>`
      : `<div class="cbars">${compactBar("5h", five)}${compactBar("7d", week)}</div>`;

  const dimmed = u ? "" : " dimmed";
  return `
  <div class="widget compact ${style}${dimmed}">
    <button class="iconbtn expand-btn" data-action="toggle-compact" title="Expand">&#x26F6;</button>
    ${content}
  </div>`;
}

function compactBar(label: string, pct: number): string {
  const p = Math.max(0, Math.min(100, pct));
  return `
  <div class="cbar">
    <span class="cbar-label">${label}</span>
    <div class="mini-track"><div class="mini-fill" style="width:${p}%;background:${utilColor(p)}"></div></div>
    <span class="cbar-pct">${fmtPct(p)}%</span>
  </div>`;
}

function compactRing(label: string, pct: number): string {
  return `<div class="cring">${ringGauge(pct, { size: 60, stroke: 7 })}<span class="cring-label">${label}</span></div>`;
}

function renderTopbar(state: UiState): string {
  const plan = state.usage?.plan ?? "Claude";
  const options =
    state.sources.length === 0
      ? `<option>No sources</option>`
      : state.sources
          .map(
            (s) =>
              `<option value="${escapeHtml(s.id)}" ${
                s.id === state.selectedId ? "selected" : ""
              }>${escapeHtml(s.label)}${s.exists ? "" : " (missing)"}</option>`,
          )
          .join("");

  return `
  <header class="topbar">
    <div class="brand">
      <span class="plan">${escapeHtml(plan)}</span>
      <select class="source-select" data-action="select-source" title="Credential source">${options}</select>
    </div>
    <div class="topbar-btns">
      <button class="iconbtn" data-action="refresh" title="Refresh now">&#x21bb;</button>
      <button class="iconbtn" data-action="toggle-compact" title="Minimize">&#x2013;</button>
      <button class="iconbtn" data-action="toggle-settings" title="Settings">&#x2699;</button>
      <button class="iconbtn" data-action="hide-window" title="Hide to tray">&#x2715;</button>
    </div>
  </header>`;
}

function renderBody(state: UiState, now: number): string {
  const src = selectedSource(state);

  if (!src) {
    return emptyState(
      "No credential source",
      "Add a .credentials.json in settings, or run Claude Code first.",
      "Open settings",
    );
  }
  if (state.usage) {
    return renderUsage(state.usage, state.stats, now);
  }
  if (state.status.kind === "unauthorized") {
    return emptyState(
      "Token expired",
      "Run Claude Code once to refresh the login, then press refresh.",
      "Refresh",
      "refresh",
    );
  }
  if (state.status.kind === "error") {
    return emptyState(
      "Couldn’t load usage",
      state.status.message ?? "Unknown error.",
      "Retry",
      "refresh",
    );
  }
  return `<div class="loading">Loading usage…</div>`;
}

function renderUsage(u: UsageSnapshot, stats: StatsHistory | null, now: number): string {
  const gauges = `
    <div class="gauges">
      ${gaugeCol("5-hour", u.fiveHour, now)}
      ${gaugeCol("Week", u.sevenDay, now)}
    </div>`;

  const models: string[] = [];
  if (u.sevenDayOpus) models.push(miniBar("Opus", u.sevenDayOpus.utilization));
  if (u.sevenDaySonnet) models.push(miniBar("Sonnet", u.sevenDaySonnet.utilization));
  const modelBlock = models.length
    ? `<div class="models">${models.join("")}</div>`
    : "";

  return `${gauges}${modelBlock}${extraUsageBlock(u.extraUsage)}${historyChart(stats)}`;
}

function extraUsageBlock(extra: ExtraUsage | null): string {
  if (!extra || !extra.is_enabled) return "";
  const cur = extra.currency ?? "USD";
  const used = extra.used_credits ?? 0;
  const limit = extra.monthly_limit;
  const pct =
    extra.utilization ?? (limit && limit > 0 ? (used / limit) * 100 : 0);
  const amount = `${fmtUsd(used, cur)} / ${limit != null ? fmtUsd(limit, cur) : "∞"}`;
  return `
  <div class="extra">
    <div class="extra-head"><span>Extra usage</span><span class="extra-amt">${escapeHtml(amount)}</span></div>
    <div class="mini-track"><div class="mini-fill" style="width:${Math.max(
      0,
      Math.min(100, pct),
    )}%;background:${utilColor(pct)}"></div></div>
  </div>`;
}

function gaugeCol(title: string, win: UsageWindow | null | undefined, now: number): string {
  const pct = win?.utilization ?? 0;
  const reset = win?.resets_at ?? null;
  return `
    <div class="gcol">
      ${ringGauge(pct)}
      <div class="gmeta">
        <span class="gtitle">${title}</span>
        <span class="reset"${reset ? ` data-reset="${reset}"` : ""}>${
          reset ? `resets in ${formatCountdown(reset, now)}` : "no reset"
        }</span>
      </div>
    </div>`;
}

function renderStatusbar(state: UiState, now: number): string {
  const kind = state.status.kind;
  let text: string;
  if (kind === "unauthorized") text = "token expired";
  else if (kind === "rate-limited") text = "rate limited";
  else if (kind === "error") text = state.status.message ?? "error";
  else if (kind === "no-source") text = "no source";
  else if (state.lastUpdatedMs) text = `updated ${ago(now - state.lastUpdatedMs)}`;
  else text = "loading…";

  return `
  <footer class="statusbar">
    <span class="dot ${kind}"></span>
    <span class="status-text" title="${escapeHtml(text)}">${escapeHtml(text)}</span>
  </footer>`;
}

function renderSettings(state: UiState): string {
  const sources = state.sources.length
    ? state.sources.map((s) => sourceRow(s, s.id === state.selectedId)).join("")
    : `<div class="muted">No sources detected.</div>`;

  const customs = state.config.customPaths.length
    ? state.config.customPaths
        .map(
          (p, i) =>
            `<div class="custom-row"><span title="${escapeHtml(p)}">${escapeHtml(
              p,
            )}</span><button class="linkbtn" data-action="remove-custom" data-index="${i}">remove</button></div>`,
        )
        .join("")
    : "";

  return `
  <div class="settings">
    <div class="settings-head">
      <span>Settings</span>
      <button class="iconbtn" data-action="toggle-settings" title="Close">&#x2715;</button>
    </div>
    <div class="settings-body">
      <div class="field">
        <span class="field-label">Source</span>
        <div class="source-list">${sources}</div>
      </div>
      <button class="btn" data-action="add-custom">+ Add credentials file…</button>
      ${customs ? `<div class="custom-list">${customs}</div>` : ""}
      <div class="field row">
        <span class="field-label">Refresh (sec)</span>
        <input class="num" type="number" min="30" max="3600" step="5"
          value="${state.config.refreshSeconds}" data-action="set-refresh" />
      </div>
      <label class="field row">
        <span class="field-label">Always on top</span>
        <input type="checkbox" data-action="toggle-aot" ${
          state.config.alwaysOnTop ? "checked" : ""
        } />
      </label>
      <div class="field row">
        <span class="field-label">Minimized style</span>
        <div class="seg-toggle">
          <label class="seg ${state.config.compactStyle === "bars" ? "on" : ""}">
            <input type="radio" name="cstyle" value="bars" ${
              state.config.compactStyle === "bars" ? "checked" : ""
            } data-action="set-compact-style" />Bars
          </label>
          <label class="seg ${state.config.compactStyle === "rings" ? "on" : ""}">
            <input type="radio" name="cstyle" value="rings" ${
              state.config.compactStyle === "rings" ? "checked" : ""
            } data-action="set-compact-style" />Rings
          </label>
        </div>
      </div>
      <div class="hint">Token stays local and is sent only to api.anthropic.com. Refresh is owned by Claude Code.</div>
    </div>
  </div>`;
}

function sourceRow(s: Source, selected: boolean): string {
  return `
    <label class="source-row${s.exists ? "" : " missing"}">
      <input type="radio" name="src" value="${escapeHtml(s.id)}" ${selected ? "checked" : ""}
        data-action="select-source" />
      <span class="source-name">${escapeHtml(s.label)}</span>
      <span class="source-state">${s.exists ? "" : "not found"}</span>
    </label>`;
}

function emptyState(title: string, body: string, action: string, actionName = "toggle-settings"): string {
  return `
  <div class="empty">
    <div class="empty-title">${escapeHtml(title)}</div>
    <div class="empty-body">${escapeHtml(body)}</div>
    <button class="btn" data-action="${actionName}">${escapeHtml(action)}</button>
  </div>`;
}
