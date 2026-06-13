// Builds a self-contained preview.html from the real widget components,
// so the look can be reviewed in any browser without WebView2/Rust.
import { build } from "esbuild";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const res = await build({
  entryPoints: [join(root, "src/preview.ts")],
  bundle: true,
  format: "esm",
  write: false,
  platform: "neutral",
});
const code = res.outputFiles[0].text;
const mod = await import("data:text/javascript;base64," + Buffer.from(code).toString("base64"));
const frames = mod.previewFrames();
const css = readFileSync(join(root, "src/styles.css"), "utf8");

const framesHtml = frames
  .map(
    (f) => `
    <figure class="frame-wrap">
      <div class="frame" style="width:${f.w}px;height:${f.h}px"><div id="app">${f.html}</div></div>
      <figcaption>${f.label} · ${f.w}×${f.h}</figcaption>
    </figure>`,
  )
  .join("");

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<title>Claude Usage — preview</title>
<style>
${css}
/* preview-only overrides (the real window is transparent + 100vh) */
html, body { height: 100%; }
.desk {
  min-height: 100vh;
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: center;
  gap: 36px;
  padding: 48px 24px 64px;
  background: radial-gradient(120% 120% at 30% 20%, #3a4150, #20242c 60%, #16181d);
}
.frame-wrap { display: flex; flex-direction: column; align-items: center; gap: 10px; }
.frame #app, .frame .widget { height: 100% !important; }
figcaption { color: #aeb4be; font: 12px "Segoe UI", sans-serif; }
.note { position: fixed; bottom: 10px; left: 0; right: 0; text-align: center; }
.note span { color: #8b919b; font: 11px "Segoe UI", sans-serif; }
</style>
</head>
<body>
  <div class="desk">${framesHtml}</div>
  <div class="note"><span>Static preview · countdowns & buttons are inert here</span></div>
</body>
</html>`;

const out = join(root, "preview.html");
writeFileSync(out, html);
console.log(`Wrote ${out} (${html.length} bytes)`);
