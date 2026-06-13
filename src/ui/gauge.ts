// SVG ring gauge + mini progress bar.

import { fmtPct, utilColor } from "../format";

interface GaugeOpts {
  size?: number;
  stroke?: number;
  label?: string;
}

/** A circular utilization gauge with a centered percentage. */
export function ringGauge(percent: number, opts: GaugeOpts = {}): string {
  const size = opts.size ?? 116;
  const stroke = opts.stroke ?? 11;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const pct = Math.max(0, Math.min(100, percent));
  const offset = c * (1 - pct / 100);
  const color = utilColor(pct);
  const cx = size / 2;
  const cy = size / 2;

  return `
  <svg class="gauge" viewBox="0 0 ${size} ${size}" width="${size}" height="${size}" role="img" aria-label="${fmtPct(pct)} percent">
    <circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="rgba(255,255,255,0.09)" stroke-width="${stroke}"/>
    <circle cx="${cx}" cy="${cy}" r="${r}" fill="none" stroke="${color}" stroke-width="${stroke}"
      stroke-linecap="round" stroke-dasharray="${c.toFixed(2)}" stroke-dashoffset="${offset.toFixed(2)}"
      transform="rotate(-90 ${cx} ${cy})"/>
    <text x="${cx}" y="${cy - 4}" text-anchor="middle" dominant-baseline="central" class="gauge-pct">${fmtPct(
      pct,
    )}<tspan class="gauge-unit">%</tspan></text>
    ${
      opts.label
        ? `<text x="${cx}" y="${cy + 20}" text-anchor="middle" class="gauge-label">${opts.label}</text>`
        : ""
    }
  </svg>`;
}

/** A compact labeled progress bar (used for the per-model weekly windows). */
export function miniBar(label: string, percent: number): string {
  const pct = Math.max(0, Math.min(100, percent));
  const color = utilColor(pct);
  return `
  <div class="mini">
    <span class="mini-label">${label}</span>
    <div class="mini-track"><div class="mini-fill" style="width:${pct}%;background:${color}"></div></div>
    <span class="mini-pct">${fmtPct(pct)}%</span>
  </div>`;
}
