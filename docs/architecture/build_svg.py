#!/usr/bin/env python3
"""Generate the animated architecture SVGs embedded in README (ru / en / uk).

Run: python3 -I docs/architecture/build_svg.py
Output: docs/architecture/takt-{ru,en,uk}.svg
"""
from pathlib import Path

OUT = Path(__file__).resolve().parent

W, H = 1380, 900
NW, NH = 184, 72
ROW = {"top": 96, "mid": 270, "bot": 444, "r4": 640, "r5": 800}
COL = {"c0": 60, "c1": 390, "c2": 720, "c3": 1050}

LANG = {
    "ru": {
        "title": "Как «Такт» дирижирует агентами",
        "owner": ("Вы", "за клавиатурой"),
        "phone": ("Телефон", "PWA в той же сети"),
        "ui": ("Интерфейс Такта", "React 19 · две темы"),
        "preview": ("Панель результата", "webview, картинки"),
        "releases": ("GitHub Releases", "подписанный latest.json"),
        "core": ("Ядро Такта", "Rust · Tauri 2"),
        "conductor": ("Дирижёр", "claude -p stream-json"),
        "memory": ("Память дирижёра", "промпт + id сессий"),
        "jobs": ("jobs.json", "реестр ~/.orchestra"),
        "agents": ("agents.py", "оркестратор tmux"),
        "executors": ("Исполнители", "Codex · agy · Claude"),
        "worktrees": ("Git-ворктри", "ветка на задачу"),
        "region_app": "Десктоп-приложение «Такт» (Tauri)",
        "region_orch": "~/.orchestra",
        "e_owner_ui": "чат с дирижёром",
        "e_phone_ui": "тот же UI по LAN",
        "e_ui_preview": "localhost, картинки",
        "e_ui_core": "IPC: команды, события",
        "e_releases_core": "проверка обновлений",
        "e_core_cond": "stream-json через stdin",
        "e_cond_memory": "системный промпт, --resume",
        "e_cond_agents": "run · send · tail",
        "e_agents_jobs": "состояние задач",
        "e_agents_exec": "сессии tmux",
        "e_exec_wt": "коммиты в своей ветке",
        "e_core_agents": "экран, лимиты",
    },
    "en": {
        "title": "How Takt conducts coding agents",
        "owner": ("You", "at the keyboard"),
        "phone": ("Phone", "PWA on the same Wi-Fi"),
        "ui": ("Takt UI", "React 19 · two themes"),
        "preview": ("Preview pane", "webview, image viewer"),
        "releases": ("GitHub Releases", "signed latest.json"),
        "core": ("Takt core", "Rust · Tauri 2"),
        "conductor": ("Conductor", "claude -p stream-json"),
        "memory": ("Conductor memory", "prompt + session ids"),
        "jobs": ("jobs.json", "~/.orchestra registry"),
        "agents": ("agents.py", "tmux orchestrator"),
        "executors": ("Executors", "Codex · agy · Claude"),
        "worktrees": ("Git worktrees", "one branch per task"),
        "region_app": "Takt desktop app (Tauri)",
        "region_orch": "~/.orchestra",
        "e_owner_ui": "chat with conductor",
        "e_phone_ui": "same UI over LAN",
        "e_ui_preview": "localhost URL, images",
        "e_ui_core": "IPC: commands, events",
        "e_releases_core": "update check",
        "e_core_cond": "stream-json via stdin",
        "e_cond_memory": "system prompt, --resume",
        "e_cond_agents": "run · send · tail",
        "e_agents_jobs": "task state",
        "e_agents_exec": "tmux sessions",
        "e_exec_wt": "commits on own branch",
        "e_core_agents": "screen, limits",
    },
    "uk": {
        "title": "Як «Такт» диригує агентами",
        "owner": ("Ви", "за клавіатурою"),
        "phone": ("Телефон", "PWA в тій самій мережі"),
        "ui": ("Інтерфейс Такту", "React 19 · дві теми"),
        "preview": ("Панель результату", "webview, зображення"),
        "releases": ("GitHub Releases", "підписаний latest.json"),
        "core": ("Ядро Такту", "Rust · Tauri 2"),
        "conductor": ("Диригент", "claude -p stream-json"),
        "memory": ("Пам'ять диригента", "промпт + id сесій"),
        "jobs": ("jobs.json", "реєстр ~/.orchestra"),
        "agents": ("agents.py", "оркестратор tmux"),
        "executors": ("Виконавці", "Codex · agy · Claude"),
        "worktrees": ("Git-ворктрі", "гілка на задачу"),
        "region_app": "Десктоп-застосунок «Такт» (Tauri)",
        "region_orch": "~/.orchestra",
        "e_owner_ui": "чат з диригентом",
        "e_phone_ui": "той самий UI по LAN",
        "e_ui_preview": "localhost, зображення",
        "e_ui_core": "IPC: команди, події",
        "e_releases_core": "перевірка оновлень",
        "e_core_cond": "stream-json через stdin",
        "e_cond_memory": "системний промпт, --resume",
        "e_cond_agents": "run · send · tail",
        "e_agents_jobs": "стан задач",
        "e_agents_exec": "сесії tmux",
        "e_exec_wt": "коміти у своїй гілці",
        "e_core_agents": "екран, ліміти",
    },
}

# id -> (col, row, kind)
NODES = {
    "owner": ("c0", "mid", "ext"),
    "phone": ("c1", "top", "ext"),
    "ui": ("c1", "mid", "front"),
    "preview": ("c1", "bot", "front"),
    "releases": ("c2", "top", "cloud"),
    "core": ("c2", "mid", "back"),
    "conductor": ("c3", "mid", "back"),
    "memory": ("c2", "bot", "db"),
    "agents": ("c3", "r4", "back"),
    "jobs": ("c3", "r5", "db"),
    "executors": ("c2", "r4", "ext"),
    "worktrees": ("c1", "r4", "db"),
}

# (from, to, label key, style)  style: main | dashed | plain
EDGES = [
    ("owner", "ui", "e_owner_ui", "main"),
    ("phone", "ui", "e_phone_ui", "dashed"),
    ("ui", "preview", "e_ui_preview", "plain"),
    ("ui", "core", "e_ui_core", "main"),
    ("releases", "core", "e_releases_core", "dashed"),
    ("core", "conductor", "e_core_cond", "main"),
    ("conductor", "memory", "e_cond_memory", "plain"),
    ("conductor", "agents", "e_cond_agents", "main"),
    ("agents", "jobs", "e_agents_jobs", "plain"),
    ("agents", "executors", "e_agents_exec", "main"),
    ("executors", "worktrees", "e_exec_wt", "plain"),
]

KIND_STROKE = {
    "ext": "#8a8f98",
    "front": "#C8FF00",
    "back": "#C8FF00",
    "cloud": "#7dd3fc",
    "db": "#f0abfc",
}


def box(nid):
    col, row, _ = NODES[nid]
    return COL[col], ROW[row]


def center(nid):
    x, y = box(nid)
    return x + NW / 2, y + NH / 2


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def edge_path(a, b):
    ax, ay = box(a)
    bx, by = box(b)
    if ay == by:  # horizontal
        y = ay + NH / 2
        if bx > ax:
            x1, x2 = ax + NW, bx
        else:
            x1, x2 = ax, bx + NW
        return f"M{x1},{y} L{x2},{y}", ((x1 + x2) / 2, y - 12), "mid"
    if ax == bx:  # vertical
        cx = ax + NW / 2
        if by > ay:
            y1, y2 = ay + NH, by
        else:
            y1, y2 = ay, by + NH
        return f"M{cx},{y1} L{cx},{y2}", (cx + 12, (y1 + y2) / 2 + 5), "start"
    # L-shaped: down/up from a, then sideways into b
    cx = ax + NW / 2
    ym = by + NH / 2
    y1 = ay + NH if by > ay else ay
    x2 = bx if bx > ax else bx + NW
    return f"M{cx},{y1} L{cx},{ym} L{x2},{ym}", ((cx + x2) / 2, ym - 12), "mid"


def wrap(label):
    """Split a horizontal edge label into two lines at a natural break."""
    if len(label) <= 16:
        return [label]
    for sep in (", ", " через ", " via ", " по ", " over "):
        if sep in label:
            a, b = label.split(sep, 1)
            return [a + sep.rstrip(), b] if sep == ", " else [a, sep.strip() + " " + b]
    words = label.split()
    mid = len(words) // 2
    return [" ".join(words[:mid]), " ".join(words[mid:])]


def render(lang):
    t = LANG[lang]
    parts = []
    parts.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
        f'font-family="Montserrat, -apple-system, Segoe UI, Helvetica, Arial, sans-serif" role="img" aria-label="{esc(t["title"])}">'
    )
    parts.append(
        """<style>
  @keyframes flow { to { stroke-dashoffset: -28; } }
  @keyframes pulse { 0%,100% { opacity: .55 } 50% { opacity: 1 } }
  @keyframes rise { from { opacity: 0; transform: translateY(8px) } to { opacity: 1; transform: translateY(0) } }
  .edge { fill: none; stroke-width: 2.2; stroke-linecap: round; stroke-linejoin: round; marker-end: url(#arrow); }
  .edge.main { stroke: #C8FF00; stroke-dasharray: 10 18; animation: flow 1.1s linear infinite; }
  .edge.plain { stroke: #5c6069; stroke-dasharray: 6 10; animation: flow 1.6s linear infinite; }
  .edge.dashed { stroke: #3d4149; stroke-dasharray: 3 9; animation: flow 2.4s linear infinite; }
  .node { animation: rise .6s cubic-bezier(.23,1,.32,1) both; }
  .node rect { fill: #121316; stroke-width: 1.5; }
  .node .lab { fill: #f3f4f6; font-size: 19px; font-weight: 600; }
  .node .sub { fill: #a8adb6; font-size: 14.5px; }
  .elab { fill: #d2d6dd; font-size: 14px; }
  .region { fill: rgba(200,255,0,.035); stroke: rgba(200,255,0,.35); stroke-dasharray: 4 6; }
  .rlab { fill: #C8FF00; font-size: 13px; letter-spacing: .4px; }
  .dot { fill: #C8FF00; animation: pulse 1.6s ease-in-out infinite; }
  .title { fill: #f3f4f6; font-size: 26px; font-weight: 600; }
</style>"""
    )
    parts.append(f'<rect width="{W}" height="{H}" rx="24" fill="#080808"/>')
    parts.append('<defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="#C8FF00"/></marker></defs>')
    parts.append(f'<text class="title" x="40" y="48">{esc(t["title"])}</text>')

    # regions
    def region(ids, label):
        xs = [box(i)[0] for i in ids]
        ys = [box(i)[1] for i in ids]
        x0, y0 = min(xs) - 22, min(ys) - 34
        x1, y1 = max(xs) + NW + 22, max(ys) + NH + 20
        parts.append(f'<rect class="region" x="{x0}" y="{y0}" width="{x1 - x0}" height="{y1 - y0}" rx="18"/>')
        parts.append(f'<text class="rlab" x="{x0 + 14}" y="{y0 + 20}">{esc(label)}</text>')

    region(["ui", "preview", "core", "conductor", "memory"], t["region_app"])
    region(["jobs", "agents"], t["region_orch"])

    # edges
    for a, b, key, style in EDGES:
        d, (lx, ly), anchor = edge_path(a, b)
        parts.append(f'<path class="edge {style}" d="{d}"/>')
        if anchor == "mid":
            lines = wrap(t[key])
            ly -= 17 * (len(lines) - 1)
            spans = "".join(f'<tspan x="{lx}" dy="{0 if i == 0 else 17}">{esc(l)}</tspan>' for i, l in enumerate(lines))
            parts.append(f'<text class="elab" x="{lx}" y="{ly}" text-anchor="middle">{spans}</text>')
        else:
            parts.append(f'<text class="elab" x="{lx}" y="{ly}">{esc(t[key])}</text>')
        if style == "main":
            parts.append(
                f'<circle class="dot" r="4"><animateMotion dur="1.8s" repeatCount="indefinite" path="{d}"/></circle>'
            )

    # nodes
    for i, (nid, (col, row, kind)) in enumerate(NODES.items()):
        x, y = box(nid)
        lab, sub = t[nid]
        parts.append(
            f'<g class="node" style="animation-delay:{i * 70}ms">'
            f'<rect x="{x}" y="{y}" width="{NW}" height="{NH}" rx="14" stroke="{KIND_STROKE[kind]}"/>'
            f'<text class="lab" x="{x + 16}" y="{y + 31}">{esc(lab)}</text>'
            f'<text class="sub" x="{x + 16}" y="{y + 54}">{esc(sub)}</text>'
            f"</g>"
        )

    parts.append("</svg>")
    return "\n".join(parts)


for lang in LANG:
    (OUT / f"takt-{lang}.svg").write_text(render(lang), encoding="utf-8")
    print("wrote", f"takt-{lang}.svg")
