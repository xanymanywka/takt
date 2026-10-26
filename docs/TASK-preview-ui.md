# TASK: Панель «Результат» в чате (SPEC §11, часть фронта)

## Цель
В чате Takt появляется сворачиваемая панель «Результат» справа: веб-предпросмотр локального dev-сервера,
картинки, документы (md/html/svg/pdf) и дифф файлов из ворктри исполнителя — без выхода из приложения.
Работает на моках в браузере (`pnpm dev`); ядро (другой исполнитель, другая ворктри) реализует команды по
контракту ниже, фронт к нему подключается через `src/lib/ipc.ts`.

## Контракт ядра (пишется параллельно, здесь только типы и моки)
```ts
export type PreviewKind = 'url' | 'image' | 'doc' | 'diff';
export interface PreviewSource { kind: PreviewKind; path?: string; url?: string; title: string; mtime: string }
preview_sources(projectPath: string): PreviewSource[]
preview_diff(path: string, worktree: string): string          // unified diff, уже обрезан ядром
preview_read(path: string): string                             // текст md/html/svg
событие 'preview://changed' payload { projectPath: string; sources: PreviewSource[] }
```
Картинки и pdf открываются через `convertFileSrc(path)` из `@tauri-apps/api/core` (asset protocol), в моке —
обычные URL/data-URI.

## Объём
- `src/lib/ipc.ts`: типы и три обёртки + мок-ветки. `src/lib/mock.ts`: 6–8 правдоподобных источников
  (vite :1420, скриншот png, `docs/TASK-*.md`, svg схема, дифф `src/App.tsx`), событие `preview://changed`
  раз в ~20 с добавляет новый источник.
- `src/store.ts`: `previewOpen`, `previewWidth` (px, 320…60 % окна, по умолчанию 40 %), `previewSources`,
  `previewActive: PreviewSource | null`, `previewDevice: 'phone'|'tablet'|'desktop'`; действия
  `togglePreview`, `setPreviewWidth`, `openPreview(source)`, `loadPreviewSources()`; последний открытый источник
  на проект — в `localStorage` (`takt-preview:<projectPath>`). Подписка на `preview://changed` в `init`.
- Новый компонент `src/components/Preview.tsx` (+ подкомпоненты в том же файле или `src/components/preview/`):
  - шапка: список источников (чипы с иконкой типа, заголовком и временем «5 мин назад» через `Intl.RelativeTimeFormat`),
    кнопка «Открыть снаружи» (`open` из `@tauri-apps/plugin-opener`, в моке `window.open`), кнопка свернуть;
  - режим **веб**: `<iframe>` на url, адресная строка (редактируемая, только `http://localhost|127.0.0.1`),
    кнопка перезагрузки, переключатель размеров «телефон 390 / планшет 820 / десктоп 100 %» — рамка с
    масштабированием `transform: scale()` чтобы влезало в панель;
  - режим **картинка**: `<img>` с зумом (колесо/кнопки 50–400 %), «до/после» — если в источниках есть два файла с
    одним именем в разных ворктри или в `.git` (упрощённо: ползунок между двумя выбранными картинками);
  - режим **документ**: md → html (`marked` или `markdown-it`, санитизация `DOMPurify`), html/svg — в `iframe
    sandbox` через `srcdoc`, pdf — `<iframe src=convertFileSrc(path)>`;
  - режим **дифф**: раскраска unified diff своими стилями (+ зелёный / − красный по токенам темы), моноширинный,
    номера строк, сворачивание больших ханков;
  - пустое состояние: «Пока нечего показать. Запусти dev-сервер или дождись файлов от исполнителя» — текст из i18n.
- Раскладка: в `App.tsx`/`Chat.tsx` панель справа от `<main className="main">`, ширина тянется разделителем
  (pointer events, min 320px), сворачивается в вертикальную полоску-кнопку «Результат» с бейджем числа новых
  источников. Кнопка открытия также в шапке чата рядом с пилюлей исполнителей. Горячая клавиша `⌘\`.
- Карточки исполнителей (`Score.tsx`/`Satellite.tsx`): если у задачи есть источники из её ворктри — маленькая
  кнопка «Результат» открывает панель на первом из них (сопоставление по `job.worktree` и `source.path`).
- i18n: все строки в `src/i18n/{en,ru,uk}.json`, `node scripts/check-i18n.mjs` зелёный.
- Стили в `src/styles/app.css` токенами из `tokens.css`; обе темы; контраст текста ≥ 7:1, кегль ≥ 12.5 px;
  шрифты Montserrat/Newsreader уже подключены. Анимация открытия — `transform`/`opacity`, 200 мс, `ease-out`.

## Не делать
- Не трогать `src-tauri/` (ядро пишет соседний исполнитель).
- Не менять экраны Settings/Onboarding/Models/About, не менять версию и CHANGELOG.
- Не добавлять тяжёлые зависимости кроме `marked` (или `markdown-it`) и `dompurify`; без Monaco/CodeMirror.

## Кто что правит
- Ты: `src/components/Preview.tsx` (новый), `src/lib/ipc.ts`, `src/lib/mock.ts`, `src/store.ts`, `src/screens/Chat.tsx`,
  `src/App.tsx`, `src/components/Score.tsx`, `src/components/Satellite.tsx`, `src/components/Icons.tsx`,
  `src/styles/app.css`, `src/i18n/*.json`, `package.json`/`pnpm-lock.yaml` (только новые зависимости).
- Не трогать: `src-tauri/**`, `.github/**`, `docs/**` кроме этого файла.

## Контекст
- Спека: `docs/SPEC.md` §6, §7, §11; дизайн-система `docs/DESIGN.md`, токены `src/styles/tokens.css`.
- Паттерны: `src/components/UpdateBanner.tsx` (плашка), `src/screens/Code.tsx` (как встроен xterm в раскладку),
  `src/store.ts` (`init` подписывается на события через `listen`), `src/i18n/index.ts` (`t()` с плюралами).
- Проверка в браузере: `pnpm dev` → http://localhost:1420 (чтобы пропустить мастер первого запуска:
  `localStorage.setItem('takt-onboarded','true')`). Скриншоты обеих тем для отчёта — `pnpm exec playwright`
  не установлен, достаточно описать и приложить `screencapture` или вывод DOM-проверок.
- CSP в `tauri.conf.json` сейчас не разрешает `frame-src localhost` — это добавляет соседний исполнитель; в
  браузере на моках ограничения нет.

## Приёмка
- [ ] `pnpm exec tsc --noEmit` → без ошибок
- [ ] `node scripts/check-i18n.mjs` → все ключи в трёх языках
- [ ] `pnpm build` → собирается
- [ ] В `pnpm dev`: панель открывается кнопкой и `⌘\`, тянется, сворачивается; все четыре режима показывают мок-данные;
  переключение «телефон/планшет/десктоп» меняет рамку; выбранный источник запоминается после перезагрузки
- [ ] Обе темы: текст панели контрастный (проверить `getComputedStyle` на заголовке и тексте диффа), кегль ≥ 12.5 px
- [ ] Карточка исполнителя с мок-ворктри показывает кнопку «Результат» и открывает панель на нужном источнике

## Обязательные правила
- Коммиты только от xanymanywka
  (`git -c user.name="xanymanywka" -c user.email="faust.9o8@gmail.com" commit ...`),
  без Co-Authored-By, без упоминаний ИИ в коммитах и README.
- Секреты в файлы и коммиты не класть.
- Никакого безымянного `git stash`.
- Продакшн не деплоить, теги не ставить, `main` не трогать — в конце отчёт дирижёру по пунктам приёмки с выводом команд.
