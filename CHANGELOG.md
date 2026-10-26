# Changelog

All notable changes to Takt. Dates are release dates; versions follow the app bundle.

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
