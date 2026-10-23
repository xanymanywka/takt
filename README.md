# Такт

Десктоп-приложение для дирижирования агентами: чат с главным агентом, под его репликой —
спутники-исполнители (Codex, Antigravity, Claude) с честным прогрессом по шагам.

- Архитектура и IPC-контракт: `docs/SPEC.md`
- Дизайн «Партитура»: `docs/DESIGN.md`, эталон `docs/design/orchestra-hand.html`
- Доска: `KANBAN.md`

```bash
pnpm install
pnpm dev          # фронт на моках, http://localhost:1420
pnpm tauri dev    # приложение целиком (нужен Rust)
```
