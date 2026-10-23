# TASK: Автообновление — tauri-plugin-updater и плашка «Доступна версия X»

## Цель
Takt сам узнаёт о новой версии в GitHub Releases (`xanymanywka/takt`), показывает плашку
«Доступна версия X — Обновить · Позже», по кнопке скачивает, ставит и перезапускается.
Проверка — при запуске и раз в сутки; её можно выключить в настройках. Канал один (stable).

## Объём
- `src-tauri/Cargo.toml`: `tauri-plugin-updater = "2"`, `tauri-plugin-process = "2"`
  (для перезапуска). `package.json`: `@tauri-apps/plugin-updater`, `@tauri-apps/plugin-process`.
- `src-tauri/src/lib.rs`: регистрация обоих плагинов (updater только для desktop).
- `src-tauri/tauri.conf.json`:
  - `bundle.createUpdaterArtifacts: true`;
  - `plugins.updater.endpoints = ["https://github.com/xanymanywka/takt/releases/latest/download/latest.json"]`;
  - `plugins.updater.pubkey` — **публичный** ключ. Файла `~/.tauri/takt.key.pub` сейчас нет: ключ
    генерирует владелец сам (`pnpm tauri signer generate -w ~/.tauri/takt.key`). Если файл появится
    к моменту работы — вставить его содержимое; если нет — оставить `"pubkey": ""` и описать в
    `docs/HANDOFF-updater.md` точный шаг для владельца. Приватный ключ и пароль в репо не класть.
- `src-tauri/capabilities/default.json`: `updater:default`, `process:default` (или минимально
  нужные `process:allow-restart`, `updater:allow-check`, `updater:allow-download-and-install`).
- `.github/workflows/release.yml`: уже есть `includeUpdaterJson: true` и секреты подписи —
  проверить, что `latest.json` попадёт в релиз, ничего лишнего не менять.
- Фронт:
  - `src/lib/updater.ts`: `checkForUpdate(): Promise<UpdateInfo | null>`,
    `installUpdate(onProgress)`; вне Tauri (`isTauriEnv()` из `src/lib/ipc.ts`) — возвращает
    `null`, без ошибок в консоли.
  - `src/store.ts`: поля `update: UpdateInfo | null`, `updateChecksEnabled: boolean`
    (localStorage `takt-update-checks`, по умолчанию `true`), `updateProgress`,
    действия `checkUpdate`, `installUpdate`, `dismissUpdate`, `setUpdateChecks`.
    Проверка при `init()` и затем каждые 24 ч (`setInterval`, чистится в cleanup).
    «Позже» прячет плашку до следующего запуска.
  - `src/components/UpdateBanner.tsx`: узкая плашка над `.win` (под шапкой `Lead`): текст
    «Доступна версия {version}», кнопки «Обновить» и «Позже», во время установки — прогресс
    «Скачивание … %» и блокировка кнопок; ошибка — красной строкой в той же плашке, без alert.
    Стили — в `src/styles/app.css` на токенах из `tokens.css` (лайм для кнопки «Обновить»,
    контраст текста ≥ 7:1, шрифт не меньше 12.5 px, обе темы).
  - Тумблер «Проверять обновления при запуске» и кнопка «Проверить сейчас» — в экране
    `src/screens/Start.tsx`, раздел «Главный агент» или новый пункт списка «Обновления» внизу
    (где логичнее, не ломая шаги онбординга).
- `docs/SPEC.md` §9: не переписывать, при расхождении с реализацией — поправить одной строкой.

## Не делать
- Не трогать `docs/design/`, `KANBAN.md`, `README*`, иконки, `src-tauri/src/{conductor,jobs,limits,pty,setup,projects,shell}.rs`.
- Не генерировать ключ подписи самому, не класть приватные ключи и пароли в репо и в `tauri.conf.json`.
- Не менять `identifier`, `productName`, `title` (должно остаться `Takt` / `space.xanyman.takt`).
- Не выставлять `ANTHROPIC_*`, не ставить прокси/роутеры. Не деплоить, не публиковать релиз, не пушить теги.

## Кто что правит
- Ты: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`,
  `src-tauri/capabilities/default.json`, `package.json` + `pnpm-lock.yaml`,
  `src/lib/updater.ts`, `src/store.ts`, `src/components/UpdateBanner.tsx`, `src/App.tsx`
  (вставка плашки), `src/screens/Start.tsx` (тумблер), `src/styles/app.css`,
  `docs/HANDOFF-updater.md` (если не доделал).
- Параллельных исполнителей на этих файлах сейчас нет.

## Контекст
- `docs/SPEC.md` §9 «Автообновление», `docs/DESIGN.md` — цвета, контраст, шрифты
  (Montserrat + Newsreader, графит `#080808` + лайм `#C8FF00`).
- Документацию `tauri-plugin-updater` v2 и `tauri-plugin-process` брать через Context7 MCP
  (нужны: `check()`, `downloadAndInstall(onEvent)`, события `Started/Progress/Finished`,
  `relaunch()`; формат `latest.json`, который делает `tauri-action` с `includeUpdaterJson`).
- Сборка: `pnpm install`, `pnpm build` (tsc + vite), `cd src-tauri && cargo check`;
  `cargo` лежит в `~/.local/bin` (симлинки на Homebrew rustup); PATH:
  `~/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin`.
- Проверить вживую: `pnpm tauri dev` — окно поднимается, в консоли нет ошибок от updater
  (с пустым pubkey/без релиза `check()` должен вернуть ошибку, которую мы глотаем в лог,
  а не падать). Плашку увидеть можно, временно подменив результат `checkForUpdate` на
  фиктивный `{ version: '9.9.9' }` — подмену убрать перед коммитом.
- Если упрёшься в лимит подписки — запиши состояние в `docs/HANDOFF-updater.md` (что сделано,
  что осталось, как проверить) и остановись.

## Приёмка
- [ ] `pnpm build` и `cd src-tauri && cargo check` без ошибок и предупреждений
- [ ] `pnpm tauri dev` поднимает окно Takt, в консоли и в терминале нет ошибок updater при
      отсутствии релиза (лог одной строкой «Обновления: …»)
- [ ] Плашка с фиктивной версией отрисовывается в обеих темах (скриншоты в отчёт), кнопки
      «Обновить»/«Позже» работают, «Позже» прячет плашку до перезапуска
- [ ] Тумблер «Проверять обновления при запуске» сохраняется в localStorage и выключает проверку
- [ ] `tauri.conf.json` содержит `endpoints` и `createUpdaterArtifacts`; приватных ключей в
      диффе нет (`git diff | grep -i "PRIVATE"` пусто)
- [ ] `coderabbit review --agent --gate major --dir .` → код выхода не 2 (один прогон; если
      CLI пишет rate limit — отметить в отчёте и не гонять повторно)
- [ ] коммиты от xanymanywka, `git log --format='%an %ae'` без других авторов

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="<email>" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах, коде и документах.
- Секреты в файлы и коммиты не класть. Никакого безымянного `git stash`.
- Продакшн не деплоить, релизы не публиковать — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
