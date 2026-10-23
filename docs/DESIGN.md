# Takt — дизайн «Партитура»

Эталон: `docs/design/orchestra-hand.html` (две темы, переключатель в шапке). Любое расхождение
с этим файлом — ошибка фронта, а не дизайна.

## Идея

Одна запоминающаяся вещь — **партитура**: каждый исполнитель — своя строка, каждый шаг плана —
такт. Сыгранные такты закрашены, текущий «дышит» (анимация бегущего блика), вопрос к пользователю
стоит знаком `?` в рамке цвета предупреждения, будущие — пустые.
Дирижёр говорит **голосом** (Newsreader), всё остальное — Montserrat. Карточек ради карточек нет:
спутники и партитура лежат на графите тоном, не рамкой. Лайм только там, где рука: дирижёр,
отправить, следующий шаг. Стекло одно — строка ввода.

## Токены

Шрифты: `--ui: "Montserrat", system-ui, sans-serif`; `--voice: "Newsreader", Georgia, serif`
(Google Fonts, у Newsreader включать `opsz`).
Радиусы `--r:14px --r2:10px --r3:7px`. Easing `--ease: cubic-bezier(.23,1,.32,1)`,
`--ease2: cubic-bezier(.32,.72,0,1)`. Нажатие: `:active{transform:scale(.97)}`.
`prefers-reduced-motion` отключает бегущий блик.

### День (`:root`, color-scheme light)

```
--ink:8,8,8; --k:.4;
--bg:#e6e9e0; --bg2:#f4f5f0; --surface:#ffffff; --raised:#e3e7db; --hover:#d9ded0;
--fg:#0b0d0c; --fg2:#1f231e; --muted:#3c413b; --faint:#51564f;
--hair:rgba(8,8,8,.11); --hair2:rgba(8,8,8,.2);
--acc:#b3e600; --acc2:#3f5a00; --acc-dim:rgba(179,230,0,.32); --acc-glow:rgba(120,170,0,.5); --on-acc:#080808;
--glow:rgba(179,230,0,.22); --sel:rgba(200,255,0,.55);
--codex:#0f7c6d; --codex-dim:rgba(15,124,109,.13);
--agy:#2f4fd0; --agy-dim:rgba(47,79,208,.13);
--ok:#1f7a3d; --warn:#8f4d00; --warn-dim:rgba(143,77,0,.1); --warn-dim2:rgba(143,77,0,.22);
--glass:rgba(255,255,255,.82); --lights:#c9cdc3;
--shadow:0 0 0 1px rgba(8,8,8,.07), 0 10px 30px -12px rgba(20,30,10,.22), 0 2px 6px -2px rgba(20,30,10,.12);
--win-shadow:0 0 0 1px rgba(8,8,8,.1),0 30px 80px -30px rgba(20,30,10,.4),0 10px 30px -10px rgba(20,30,10,.18);
```

### Ночь (`:root[data-theme=dark]` и `@media (prefers-color-scheme: dark) :root:not([data-theme=light])`)

```
--ink:255,255,255; --k:1;
--bg:#080808; --bg2:#0e0f0f; --surface:#151616; --raised:#1c1d1d; --hover:#242626;
--fg:#f4f5f0; --fg2:#d3d7cd; --muted:#a6ab9f; --faint:#979c93;
--hair:rgba(242,243,238,.08); --hair2:rgba(242,243,238,.14);
--acc:#c8ff00; --acc2:#d6ff3d; --acc-dim:rgba(200,255,0,.16); --acc-glow:rgba(200,255,0,.45); --on-acc:#080808;
--glow:rgba(200,255,0,.07); --sel:rgba(200,255,0,.35);
--codex:#6fd3c3; --codex-dim:rgba(111,211,195,.14);
--agy:#8ea6ff; --agy-dim:rgba(142,166,255,.14);
--ok:#7ed69a; --warn:#ffb648; --warn-dim:rgba(255,182,72,.08); --warn-dim2:rgba(255,182,72,.2);
--glass:rgba(22,23,23,.72); --lights:#2a2c2b;
--shadow:0 1px 0 rgba(255,255,255,.04) inset, 0 10px 30px -12px rgba(0,0,0,.6), 0 2px 6px -2px rgba(0,0,0,.4);
--win-shadow:0 0 0 1px rgba(255,255,255,.06),0 30px 80px -30px rgba(0,0,0,.8),0 10px 30px -10px rgba(0,0,0,.5);
```

`rgba(var(--ink), .06)` — подсветка строк/ховеров; чёрные тени умножаются на `--k`:
`rgba(0,0,0,calc(.35*var(--k)))`.

## Типографика и доступность

У владельца слабое зрение. Правила обязательные:
- вторичный текст (`--muted`, `--faint`) контраст **≥ 7:1** на своём фоне, основной ≥ 12:1;
- **минимальный кегль 12.5 px**, интерфейс 15 px, реплики дирижёра 18.5 px, заголовки Newsreader 34 px;
- никаких подписей заглавными, никаких «eyebrow»-ярлыков, никаких плашек с прозрачностью текста;
- иконки — рисованные SVG, одна толщина штриха 1.5 px, 16–18 px;
- фокус клавиатуры видим (`outline: 2px solid var(--acc); outline-offset: 2px`).

## Цвета ролей

Claude/дирижёр — лайм `--acc`; Codex — мята `--codex`; Antigravity — синий `--agy`;
вопрос/внимание — `--warn`; готово — `--ok`. Больше цветов не вводить.

## Движение

Появление спутника: `opacity 0→1, translateY(6px)→0, 220ms var(--ease)`.
Бегущий блик текущего такта: `@keyframes head` 1.6 s linear infinite, только на `.now`.
Переходы ховеров 160–180 ms. Ничего не анимировать при смене вкладки по клавиатуре.
