# TASK: Язык интерфейса (uk/en/ru), «О программе», «Поддержать автора»

## Цель
Интерфейс Takt говорит на украинском, английском и русском: язык берётся из системной локали,
меняется в настройках, все строки идут через `t()`. Появляется экран «О программе» с автором,
версией, лицензией и кнопкой «Проверить обновления», и плашка «Поддержать автора» с данными
из `src/about/support.json` (пустые поля скрыты).

## Объём
- `src/i18n/`:
  - `uk.json`, `en.json`, `ru.json` — плоские ключи вида `lead.title`, `tabs.chat`,
    `jobs.state.running`, одинаковый набор ключей во всех трёх (проверка скриптом или тестом);
  - `index.ts`: `t(key, params?)`, `useT()`, `setLocale`, `getLocale`, определение по
    `navigator.language` (uk → uk, ru/be → ru, иначе en), сохранение в localStorage
    `takt-locale`; плюрализация через `Intl.PluralRules`, даты и числа через `Intl`.
- Заменить **все** сырые строки интерфейса в `src/**/*.tsx` и `src/lib/labels.ts` на `t()`
  (включая `aria-label`, placeholder, подписи состояний задач). `src/lib/mock.ts` — не трогать
  (это демо-данные, не интерфейс). Заголовок «Takt. Партитура, а не дашборд.» переводится,
  слово Takt — всегда латиницей.
- Выбор языка: в `src/screens/Start.tsx` новый пункт «Язык» (три кнопки uk/en/ru с названиями
  на самих языках) и компактный переключатель в `src/components/ThemeToggle.tsx` рядом с темой
  (или отдельный `LocaleToggle.tsx`).
- «О программе»: `src/screens/About.tsx`, вход — пункт внизу боковой панели `Sidebar.tsx` и
  последний пункт списка в `Start.tsx`. Содержимое: имя Takt, версия (из `@tauri-apps/api/app`
  `getVersion()`, вне Tauri — из `package.json` через vite `define`), автор Андрей (xanymanywka),
  ссылки https://xanyman.space, Telegram https://t.me/xanymanywka, GitHub
  https://github.com/xanymanywka/takt (открывать через `@tauri-apps/plugin-opener`, вне Tauri —
  `window.open`), абзац о проекте (одно из `about.description` в i18n), лицензия
  PolyForm Noncommercial 1.0.0 со ссылкой на `LICENSE`, кнопка «Проверить обновления»
  (вызывает `checkUpdate` из `src/store.ts`, если он уже есть после задачи updater; иначе —
  кнопка-заглушка с `t('about.updatesSoon')`).
- «Поддержать автора»: `src/about/support.json` вида
  `{ "card": "", "iban": "", "paypal": "", "binance": "", "usdtTrc20": "" }` — все пустые,
  владелец заполнит сам. `src/components/Support.tsx`: карточка в «О программе» и короткая
  плашка внизу `Sidebar.tsx`; показываются только непустые поля; клик по значению копирует
  в буфер (`navigator.clipboard`) и на 1.5 с показывает «Скопировано»; для крипто-адресов —
  QR (`qrcode` npm, SVG, без сетевых запросов).
- `docs/SPEC.md` §10: при расхождении с реализацией поправить одной строкой, не переписывать.

## Не делать
- Не трогать `src-tauri/**`, `docs/design/`, `KANBAN.md`, `README*`, `src/lib/mock.ts`,
  `src/lib/ipc.ts` (контракт).
- Не ставить i18next/react-intl — своя тонкая реализация на JSON + `Intl`.
- Не придумывать реквизиты для `support.json`, не вставлять чужие.
- Не выставлять `ANTHROPIC_*`, не ставить прокси/роутеры. Не деплоить, не публиковать.

## Кто что правит
- Ты: `src/i18n/**`, `src/about/**`, `src/screens/About.tsx`, `src/components/{Support,LocaleToggle}.tsx`,
  правки строк во всех `src/**/*.tsx`, `src/lib/labels.ts`, `src/store.ts` (поле `locale`,
  экран `about`), `src/styles/app.css`, `package.json` + `pnpm-lock.yaml` (только `qrcode`),
  `vite.config.ts` (define версии), `docs/HANDOFF-i18n-about.md` (если не доделал).
- Параллельно исполнитель updater правит `src/store.ts`, `src/App.tsx`, `src/screens/Start.tsx`,
  `src/components/UpdateBanner.tsx`, `src/styles/app.css`. Если его ветка `updater` ещё не
  смёржена в main — в `store.ts`/`App.tsx`/`Start.tsx` вносить **минимальные** правки
  (добавлять, не переформатировать), чтобы мёрж прошёл без конфликтов.

## Контекст
- `docs/SPEC.md` §10, `docs/DESIGN.md` (графит `#080808` + лайм `#C8FF00`, Montserrat +
  Newsreader, контраст ≥ 7:1, кегль ≥ 12.5 px, две темы).
- Все экраны: `src/App.tsx`, `src/screens/{Chat,Code,Orchestra,Start}.tsx`,
  `src/components/*.tsx`. Состояние — `src/store.ts` (zustand).
- Сборка: `pnpm install`, `pnpm build` (tsc + vite); проверка вживую — `pnpm dev`
  (мок-данные, http://localhost:1420) и `pnpm tauri dev`; `cargo` в `~/.local/bin`.
- Если упрёшься в лимит подписки — запиши состояние в `docs/HANDOFF-i18n-about.md` и остановись.

## Приёмка
- [ ] `pnpm build` без ошибок; `grep -rn "[А-Яа-яЁёІіЇїЄє]" src --include=*.tsx | grep -v i18n`
      не находит сырых строк интерфейса (кроме комментариев и `mock.ts`)
- [ ] Наборы ключей `uk/en/ru.json` совпадают (скрипт `scripts/check-i18n.mjs` или тест, вывод в отчёт)
- [ ] Переключение языка меняет все экраны без перезагрузки, выбор переживает перезапуск
- [ ] При системной локали `uk-UA` первый запуск — украинский; `de-DE` — английский
- [ ] «О программе» показывает версию из Tauri, ссылки открываются во внешнем браузере
- [ ] `support.json` пустой → плашка «Поддержать» не показывается; с одним заполненным полем —
      показывается только оно, копирование работает (скриншоты обеих тем в отчёт)
- [ ] `coderabbit review --agent --gate major --dir .` → код выхода не 2 (один прогон)
- [ ] коммиты от xanymanywka, `git log --format='%an %ae'` без других авторов

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="<email>" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах, коде и документах.
- Секреты и реквизиты в коммиты не класть. Никакого безымянного `git stash`.
- Продакшн не деплоить — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
