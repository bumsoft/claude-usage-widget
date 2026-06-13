// Composes a polished hero image (expanded + both compact styles) for the
// landing page. Writes .hero.html; a screenshot step turns it into docs/hero.png.
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
const mod = await import(
  "data:text/javascript;base64," + Buffer.from(res.outputFiles[0].text).toString("base64")
);
const frames = Object.fromEntries(mod.previewFrames().map((f) => [f.label, f]));
const css = readFileSync(join(root, "src/styles.css"), "utf8");

const expanded = frames["Expanded"];
const bars = frames["Compact · Bars"];
const rings = frames["Compact · Rings"];

const frameDiv = (f) =>
  `<div class="shot" style="width:${f.w}px;height:${f.h}px"><div id="app">${f.html}</div></div>`;

const html = `<!doctype html><html><head><meta charset="UTF-8"><style>
${css}
html,body{height:100%;margin:0}
body{
  width:760px;height:560px;
  display:flex;align-items:center;justify-content:center;gap:48px;
  background:radial-gradient(130% 130% at 25% 15%, #424b5c, #20242c 55%, #14161b);
}
.col{display:flex;flex-direction:column;gap:36px;}
.shot{ border-radius:14px; box-shadow:0 30px 70px rgba(0,0,0,.55), 0 4px 12px rgba(0,0,0,.4); }
.shot #app, .shot .widget{height:100%!important}
</style></head><body>
  ${frameDiv(expanded)}
  <div class="col">${frameDiv(bars)}${frameDiv(rings)}</div>
</body></html>`;

writeFileSync(join(root, ".hero.html"), html);
console.log("wrote .hero.html");
