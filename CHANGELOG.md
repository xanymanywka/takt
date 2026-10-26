# Changelog

All notable changes to Takt. Dates are release dates; versions follow the app bundle.

## 0.3.1 — Unreleased

### Fixed
- Chat history is now kept per project in `~/.orchestra/chats/<project>.json` and restored when the project
  is opened again; before, messages lived only in memory and vanished on restart or project switch.
- On first open the history is imported from the Claude Code transcript of that project directory
  (the Takt session when it exists, otherwise the newest conversation), so chats started from the terminal
  are visible too. Codex / Antigravity conversations are recorded from this version on.
- Conductor events are scoped to their project: a reply arriving for a background project no longer
  lands in the active chat.

### Changed
- App icon: the glyph now fills about 65 % of the canvas instead of 40 %, so Takt no longer looks smaller
  than neighbouring icons in Launchpad, the Dock and the Applications folder. Icon set regenerated from
  `docs/design/takt-icon.svg`.

## 0.3.0 — 2026-10-26

### Added
- Result panel in the chat (`⌘\`): web preview of a local dev server with phone / tablet / desktop frames,
  images with zoom and before/after compare, Markdown / HTML / SVG documents, coloured unified diff of files
  changed by an executor. Sources are discovered by the core: listening localhost ports, fresh images and
  documents in the project and its worktrees, `git status` of every worktree; a watcher pushes updates live.
- Selectable conductor: Claude Code, Codex CLI or Antigravity. Chosen in the first-run wizard and in Settings,
  stored in `~/.orchestra/conductor.json`; the core adapts each CLI's output to one event stream, so the chat
  works the same whichever agent conducts.
- Phone companion server (`companion.rs`): TLS server on a random port with a self-signed certificate, mDNS
  announcement, pairing by token or short code (5-minute TTL, rate limited), WebSocket with bearer auth, device
  registry and revocation, autostart. The phone client itself ships in a later release.
- Platform layer (`platform.rs`): executors can run in a built-in PTY runner without tmux, shell and terminal
  commands resolved per OS, Windows and Linux builds (`msi`, `nsis`, `deb`, `appimage`) in the release workflow,
  CI on macOS, Windows and Linux.

### Changed
- Content Security Policy allows `frame-src` for localhost and the asset protocol for images and documents.
- Setup screen reports a warning instead of a failure where tmux is not available.

## 0.2.2 — 2026-10-26

### Added
- First-run onboarding wizard: welcome, terminal tools, projects, access, instructions, done. Shown once
  (`takt-onboarded`), can be restarted from Settings.
- Settings screen (gear in the header): language, theme, update checks, links to Start / Models / About.
- Styled DMG installer window: graphite background, lime arrow, hint about the first launch
  (`src-tauri/dmg/background.html`, rendered by `scripts/dmg-background.mjs`).
- Wizard checks `~/.orchestra/agents.py` and reports honestly when the conductor script is missing.
- Acceptance checklist header in task files is recognised in three languages: `## Приёмка`, `## Приймання`,
  `## Acceptance`.
- CI workflow (`ci.yml`): `tsc --noEmit`, i18n key check, `cargo test` on every pull request and push to `main`.
- This changelog.

### Changed
- Full-bleed layout: the window fills the screen instead of a fixed card in the middle.
- `open_in_terminal` accepts only allow-listed action keys (`install:tmux`, `login:codex`, …); the UI can no longer
  pass arbitrary shell text to Terminal. `~` is resolved to the home folder.
- Content Security Policy enabled for the webview (`app.security.csp` and `devCsp`).
- README states plainly that the wizard installs nothing and never types passwords.

### Removed
- Fake traffic-light dots in the sidebar.

## 0.2.1 — 2026-10-23

- Updater signing key pair; releases publish `latest.json` and signed `.app.tar.gz`.
- Brand logos (Claude, Codex, Antigravity) on agent avatars.
- `scripts/install.sh`: one-line install into `/Applications` without the Gatekeeper dialog.

## 0.2.0 — 2026-10-23

- Models screen: lists from `claude`, Codex cache, `agy models`, `ollama list`; choice saved to
  `~/.orchestra/models.json` and passed to `agents.py` as `--model`.
- CodeRabbit UI fixes: real buttons, `role="tab"`, bundled Montserrat / Newsreader via `@fontsource-variable`.
- Auto-update via `tauri-plugin-updater`, update banner, 24-hour check.
- i18n uk / en / ru, About screen, Support card.

## 0.1.0 — 2026-10-23

- Tauri 2 + React 19 scaffold, «Score» interface: Chat, Code (xterm), Orchestra, Access.
- Rust core: jobs, limits, conductor stream, PTY, setup inspection.
- Two themes, high-contrast typography, animated architecture diagram in the README.
