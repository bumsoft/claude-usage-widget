// Stacked daily token-usage bar chart built from local stats-cache data.

import { escapeHtml, fmtTokens, fmtUsd, modelColor, shortModel } from "../format";
import type { DailyTokens, StatsHistory } from "../types";

function dayTotal(day: DailyTokens): number {
  return Object.values(day.tokensByModel).reduce((a, b) => a + b, 0);
}

const MAX_BAR_PX = 54;

export function historyChart(stats: StatsHistory | null): string {
  if (!stats || stats.days.length === 0) {
    return `<div class="chart-empty">No local token history yet</div>`;
  }

  const totals = stats.days.map(dayTotal);
  const max = Math.max(...totals, 1);

  const bars = stats.days
    .map((day, i) => {
      const total = totals[i];
      const h = Math.max(total > 0 ? 2 : 0, Math.round((total / max) * MAX_BAR_PX));
      const segs = stats.models
        .filter((m) => day.tokensByModel[m])
        .map((m) => {
          const v = day.tokensByModel[m];
          const sh = total ? (v / total) * h : 0;
          return `<div class="seg" style="height:${sh.toFixed(1)}px;background:${modelColor(
            m,
          )}"></div>`;
        })
        .join("");
      const title = `${day.date} · ${fmtTokens(total)} tok · ≈${fmtUsd(day.costUsd)}`;
      return `<div class="bar" style="height:${h}px" title="${escapeHtml(title)}">${segs}</div>`;
    })
    .join("");

  const legend = stats.models
    .map(
      (m) =>
        `<span class="lg"><i style="background:${modelColor(m)}"></i>${escapeHtml(
          shortModel(m),
        )}</span>`,
    )
    .join("");

  const grand = totals.reduce((a, b) => a + b, 0);

  return `
  <div class="chart">
    <div class="chart-head">
      <span class="chart-title">Tokens · last ${stats.days.length}d</span>
      <span class="chart-sum">${fmtTokens(grand)} · <b class="cost">≈${fmtUsd(stats.costUsd)}</b></span>
    </div>
    <div class="bars">${bars}</div>
    <div class="legend">${legend}</div>
  </div>`;
}
