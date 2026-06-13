// Small formatting + color helpers shared across the UI.

/** Format a utilization percentage: one decimal under 10, integer otherwise. */
export function fmtPct(p: number): string {
  const v = Math.max(0, Math.min(100, p));
  return v >= 10 ? String(Math.round(v)) : String(Math.round(v * 10) / 10);
}

/** Human countdown to an RFC3339 reset timestamp. */
export function formatCountdown(iso: string | null, nowMs: number): string {
  if (!iso) return "—";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "—";
  let s = Math.floor((t - nowMs) / 1000);
  if (s <= 0) return "now";
  const d = Math.floor(s / 86400);
  s -= d * 86400;
  const h = Math.floor(s / 3600);
  s -= h * 3600;
  const m = Math.floor(s / 60);
  const sec = s - m * 60;
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${sec}s`;
  return `${sec}s`;
}

/** Color a gauge/bar by how full it is. */
export function utilColor(pct: number): string {
  if (pct >= 80) return "#e8615a";
  if (pct >= 50) return "#e6a93a";
  return "#46c79b";
}

export function fmtTokens(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${Math.round(n / 1e3)}k`;
  return String(n);
}

/** Compact USD formatting: "$3.42", "$48", "$1.2k". */
export function fmtUsd(n: number, currency = "USD"): string {
  const sym = currency === "USD" || !currency ? "$" : `${currency} `;
  if (n >= 1000) return `${sym}${(n / 1000).toFixed(1)}k`;
  if (n >= 100) return `${sym}${Math.round(n)}`;
  return `${sym}${n.toFixed(2)}`;
}

/** Drop the `claude-` prefix for compact model labels. */
export function shortModel(model: string): string {
  return model.replace(/^claude-/, "");
}

/** Stable color per model family. */
export function modelColor(model: string): string {
  if (model.includes("opus")) return "#cf8a5b";
  if (model.includes("sonnet")) return "#6ea8e6";
  if (model.includes("haiku")) return "#69c08a";
  if (model.includes("fable")) return "#b98ad6";
  return "#8a8f98";
}

/** Relative "x ago" label for the last-updated timestamp. */
export function ago(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  return `${Math.floor(m / 60)}h ago`;
}

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
