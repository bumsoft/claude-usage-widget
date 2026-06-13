# Claude Usage Widget

An always-on-top Windows desktop widget that shows your **Claude subscription usage** —
the same rolling-window limits you see in Claude Code's `/usage`, plus your local
token history.

![icon](src-tauri/app-icon.png)

## Install

**[⬇ Download the latest installer](https://github.com/bumsoft/claude-usage-widget/releases/latest)**
&nbsp;·&nbsp; [Landing page](https://bumsoft.github.io/claude-usage-widget/)

Grab the `*-setup.exe` from the latest release and run it. The build is unsigned,
so Windows SmartScreen may warn on first launch — click **More info → Run anyway**.
Requires Windows 10/11 (WebView2, preinstalled on Win 11).

Prefer to build it yourself? See [Build](#build) below.

## What it shows

- **5-hour window** and **weekly window** utilization, as ring gauges with live
  reset countdowns.
- **Per-model weekly windows** (Opus / Sonnet) as compact bars when present.
- **Local token history** — a stacked daily chart from `stats-cache.json`.
- **Source switcher** — pick between your Windows install and any WSL distro, or
  add a custom `.credentials.json` path. One source shown at a time; switch from
  the dropdown or the tray.

## How it works

```
.credentials.json ──(read-only)──▶ Rust backend ──HTTPS──▶ api.anthropic.com/api/oauth/usage
                                        │
stats-cache.json  ──(read-only)──▶ Rust backend ──IPC──▶ WebView UI (gauges + chart)
```

- The OAuth access token is read from `.credentials.json` and used **only** to call
  `https://api.anthropic.com/api/oauth/usage`. It never leaves the Rust backend —
  the WebView only ever receives utilization numbers, reset times, and a plan label.
- `.credentials.json` is treated as **read-only**. Token refresh is owned by Claude
  Code, so the widget never writes the file and can't disrupt your CLI session. If
  the token has expired, the widget shows "Token expired — run Claude Code once".

## Prerequisites (build on Windows)

The artifact is a native Windows app, so build it **on Windows** (not inside WSL):

1. **Node.js** 18+ — <https://nodejs.org>
2. **Rust** (stable, MSVC) — <https://rustup.rs>
3. **WebView2 runtime** — preinstalled on Windows 11; on Windows 10 install the
   [Evergreen runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
4. **MSVC build tools** — "Desktop development with C++" from the Visual Studio
   Build Tools (Rust's MSVC toolchain needs the linker).

> The project currently lives in WSL at
> `\\wsl.localhost\Ubuntu\home\wlsbum\projects\widget\windows`. For a smooth build,
> copy it to a native Windows path (e.g. `C:\dev\claude-usage-widget`). Building
> Rust over the `\\wsl.localhost` UNC path works but is slow.
>
> Run `npm install` **on Windows** — don't reuse the `node_modules` produced under
> WSL (its `esbuild`/`rollup` native binaries are Linux-only).

## Build

```powershell
npm install
npm run tauri build
```

The installer is written to:

```
src-tauri\target\release\bundle\nsis\Claude Usage Widget_0.1.0_x64-setup.exe
```

(The standalone `.exe` is at `src-tauri\target\release\claude-usage-widget.exe`.)

## Develop

```powershell
npm run tauri dev      # hot-reloading dev build
```

## Quick visual preview (no Rust/WebView2 needed)

```bash
npm run preview:html   # writes preview.html with mock data
```

Open `preview.html` in any browser to review the layout and styling. Countdowns and
buttons are inert in this static preview.

## Using the widget

- The window is frameless and stays on top. **Drag** it by the title bar.
- **⟳** refresh now · **⚙** settings · **✕** hide to tray.
- Closing hides to the **system tray** (polling keeps running). Left-click the tray
  icon to toggle the window; right-click for **Show / Hide** and **Quit**.
- **Settings** lets you choose the credential source, add a custom `.credentials.json`,
  set the refresh interval, and toggle always-on-top.

### Source detection

On launch the widget scans:

- `%USERPROFILE%\.claude\.credentials.json` (Windows host)
- every WSL distro via `wsl.exe -l -q`, reached at
  `\\wsl.localhost\<distro>\<home>\.claude\.credentials.json`

If a distro isn't auto-detected, add its path manually in **Settings → Add
credentials file…**.

## Project layout

```
src/                     # frontend (TypeScript, no framework)
  main.ts                #   state, events, polling, countdown clock
  ui/app.ts              #   pure render: topbar, gauges, chart, settings
  ui/gauge.ts            #   SVG ring gauge + mini bars
  ui/chart.ts            #   stacked daily token chart
  api.ts format.ts ...   #   Tauri command wrappers, helpers, types
src-tauri/
  src/                   # Tauri shell: commands, discovery, config, tray
  core/                  # platform-agnostic crate (parsing + usage API)
    src/{credentials,usage,stats,wsl,sources}.rs
```

## Tests

The platform-agnostic core logic (credential/usage/stats parsing, WSL path building)
is unit tested and runs on any OS:

```bash
cargo test -p claude-usage-core --manifest-path src-tauri/Cargo.toml
```

## Releasing

A GitHub Actions workflow (`.github/workflows/release.yml`) builds the Windows
installer and publishes a Release whenever a version tag is pushed:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The landing page in `docs/` is served via GitHub Pages (Settings → Pages →
Branch: `main`, Folder: `/docs`). Its "Download" button points at
`releases/latest`, so it always tracks the newest build.

## Notes & limitations

- The usage endpoint and the `anthropic-beta: oauth-2025-04-20` header are Claude
  Code's internal usage API; Anthropic may change them. The parser ignores unknown
  fields so additions won't break it.
- The API-cost figure is an **estimate** using public per-model list prices
  (subscriptions have no real per-token cost). Cache reads are excluded from the
  daily bars to keep the trend readable.
