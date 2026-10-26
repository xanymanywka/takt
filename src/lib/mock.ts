import type {
  ConductorAgent,
  ConductorConfig,
  HistoryMessage,
  Job,
  Limit,
  ModelProviderId,
  ModelSelection,
  ModelsInfo,
  PreviewSource,
  Project,
  Setup,
  UnlistenFn,
} from './ipc';

export const MOCK_PROJECTS: Project[] = [
  { name: 'Takt', path: '~/orchestra/takt', group: 'takt', color: '#9b86d6' },
  { name: 'Acme · CRM', path: '~/projects/acme-crm', group: 'acme', color: '#9b86d6' },
  { name: 'Acme · Shop', path: '~/projects/acme-shop', group: 'acme', color: '#9b86d6' },
  { name: 'Roadmap', path: '~/projects/roadmap', group: 'roadmap', color: '#7fb98f' },
  { name: 'Client · NDA', path: '~/projects/client-nda', group: 'nda', color: '#e0a05f' },
  { name: 'Personal', path: '~/projects/personal', group: 'personal', color: '#7fa6d6' },
];

export const MOCK_LIMITS: Limit[] = [
  { provider: 'claude', sessionLeft: 62, raw: 'Claude 62%' },
  { provider: 'codex', sessionLeft: 41, raw: 'Codex 41%' },
  { provider: 'antigravity', sessionLeft: 88, raw: 'Antigravity 88%' },
];

export const MOCK_JOBS: Job[] = [
  {
    name: 'pty-bridge',
    agent: 'codex',
    worktree: 'pty-bridge',
    task: 'docs/TASK-core.md',
    startedAt: '14:02',
    state: 'running',
    model: 'gpt-5-codex',
    lastLine: 'Пишет src/pty/bridge.rs: resize и сигналы',
    steps: [
      { text: 'Прочитал spec/pty.md и tauri.conf.json', done: true },
      { text: 'Добавил зависимость portable-pty 0.8', done: true },
      { text: 'Пишет src/pty/bridge.rs: resize и сигналы', done: false },
      { text: 'тесты на закрытие сессии', done: false },
      { text: 'CodeRabbit gate major', done: false },
    ],
    logLines: [
      { time: '14:02', text: 'Прочитал spec/pty.md и tauri.conf.json' },
      { time: '14:04', text: 'Добавил зависимость portable-pty 0.8' },
      { time: '14:06', text: 'Пишет src/pty/bridge.rs: resize и сигналы' },
    ],
  },
  {
    name: 'terminal-pane',
    agent: 'gemini',
    worktree: 'terminal-pane',
    task: 'docs/TASK-ui.md',
    startedAt: '14:03',
    state: 'asking',
    model: 'Gemini 2.5 Pro',
    lastLine: 'Нашёл арабскую локаль в i18n, остановился',
    question: 'Панель терминала должна поддерживать RTL для арабского, или оставить только LTR? Это влияет на раскладку табов.',
    chips: ['Только LTR', 'RTL и LTR', 'Решу позже'],
    steps: [
      { text: 'Разобрал компонент TerminalPane.tsx', done: true },
      { text: 'вёрстка табов', done: false },
      { text: 'Интеграция xterm + fit', done: false },
      { text: 'Приёмка по дизайну', done: false },
      { text: 'Проверка тестов', done: false },
    ],
    logLines: [
      { time: '14:03', text: 'Разобрал компонент TerminalPane.tsx' },
      { time: '14:05', text: 'Нашёл арабскую локаль в i18n, остановился' },
    ],
  },
];

export const MOCK_SETUP: Setup = {
  instructionsExists: false,
  projectsExists: true,
  tools: [
    {
      id: 'gh',
      found: true,
      loggedIn: true,
      account: 'you и you-work, переключение по проекту',
      version: '2.68.0',
      statusText: 'вошли',
      agents: { claude: true, codex: true, agy: true },
    },
    {
      id: 'coderabbit',
      found: true,
      loggedIn: true,
      account: 'ревью перед мёржем, стоп-кран на major',
      version: '1.2.0',
      statusText: 'вошли',
      agents: { claude: true, codex: true, agy: true },
    },
    {
      id: 'claude',
      found: true,
      loggedIn: false,
      account: 'подписка Max, you',
      version: '2.0.32',
      statusText: 'вход истёк, откроем окно входа',
      warn: true,
      agents: { claude: true, codex: false, agy: false },
    },
    { id: 'agents', found: true, path: '~/.orchestra/agents.py' },
  ],
  ssh: [
    {
      host: 'prod-1',
      hostname: '203.0.113.10',
      user: 'deploy',
      port: 22,
      identityFile: 'id_ed25519_deploy',
      status: 'подключается',
      checked: 'сегодня, 13:48',
      usedBy: 'CRM, Finance',
      rule: 'Antigravity на этот хост не ходит, здесь лежат файлы клиента под NDA.',
      agents: { claude: true, codex: true, agy: false },
    },
    {
      host: 'web',
      hostname: 'web.example.com',
      user: 'www',
      port: 22,
      status: 'подключается',
      identityFile: 'id_ed25519_web',
      checked: 'вчера, 18:20',
      usedBy: 'Landing, Blog',
      agents: { claude: true, codex: true, agy: true },
    },
    {
      host: 'shop',
      hostname: 'shop.example.net',
      user: 'shop',
      port: 22,
      status: 'ключ истекает через 9 дней',
      identityFile: 'id_rsa_shop',
      checked: 'сегодня, 11:15',
      usedBy: 'Portfolio, Store',
      warn: true,
      agents: { claude: true, codex: true, agy: true },
    },
  ],
  mcp: [
    { name: 'context7', source: 'agy' },
    { name: 'github', source: 'codex' },
    { name: 'postgres', source: 'codex' },
    { name: 'filesystem', source: 'agy' },
  ],
};

let currentProjects = [...MOCK_PROJECTS];
let currentJobs = JSON.parse(JSON.stringify(MOCK_JOBS)) as Job[];
const currentLimits = [...MOCK_LIMITS];

type Handler<T = unknown> = (ev: { event: string; payload: T }) => void;
const listeners = new Map<string, Set<Handler>>();

export function emit<T>(event: string, payload: T): void {
  const set = listeners.get(event);
  if (set) {
    set.forEach((handler) => {
      try {
        handler({ event, payload });
      } catch (err) {
        console.error('Error in mock event handler:', err);
      }
    });
  }
}

function emitJobs(): void {
  emit('jobs://changed', JSON.parse(JSON.stringify(currentJobs)));
}

export function listen<T>(event: string, handler: (ev: { event: string; payload: T }) => void): UnlistenFn {
  let set = listeners.get(event);
  if (!set) {
    set = new Set();
    listeners.set(event, set);
  }
  set.add(handler as Handler);
  return () => {
    const s = listeners.get(event);
    if (s) {
      s.delete(handler as Handler);
    }
  };
}

// ---- Эмуляция таймера для живого обновления в моке ----
let timerStarted = false;
function ensureTimer(): void {
  if (timerStarted || typeof window === 'undefined') return;
  timerStarted = true;

  let tick = 0;
  setInterval(() => {
    tick += 1;
    const ptyJob = currentJobs.find((j) => j.name === 'pty-bridge');
    if (ptyJob && ptyJob.state === 'running') {
      const now = new Date();
      const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
      if (tick % 2 === 0) {
        ptyJob.lastLine = `Компилирует src/pty/bridge.rs (такт ${tick})`;
        ptyJob.logLines = [
          ...(ptyJob.logLines || []).slice(-4),
          { time: timeStr, text: `Обновление модуля bridge.rs (итерация ${tick})` },
        ];
        emitJobs();
      }
    }
  }, 7000);
}

// Вызываем инициализацию таймера
if (typeof window !== 'undefined') {
  setTimeout(ensureTimer, 1000);
}

// ---- Реализация команд ----
export async function list_projects(): Promise<Project[]> {
  return [...currentProjects];
}

export async function save_projects(projects: Project[]): Promise<void> {
  currentProjects = [...projects];
}

export async function list_jobs(): Promise<Job[]> {
  ensureTimer();
  return JSON.parse(JSON.stringify(currentJobs));
}

export async function job_screen(name: string, lines = 20): Promise<string> {
  const job = currentJobs.find((j) => j.name === name);
  if (!job) return `[job ${name} not found]`;
  const logs = job.logLines || [];
  return logs.slice(-lines).map((l) => `${l.time} ${l.text}`).join('\n') || job.lastLine;
}

export async function job_send(name: string, text: string): Promise<void> {
  console.log('[mock job_send]', name, text);
  const job = currentJobs.find((j) => j.name === name);
  if (job) {
    if (job.state === 'asking') {
      job.state = 'running';
      job.question = undefined;
      job.lastLine = `Принял ответ: «${text}». Продолжаю вёрстку табов`;
      if (job.steps[1]) job.steps[1].done = true;
      const now = new Date();
      const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
      job.logLines = [
        ...(job.logLines || []),
        { time: timeStr, text: `Ответ получен: ${text}` },
      ];
      emitJobs();
    }
  }
}

export async function job_kill(name: string): Promise<void> {
  console.log('[mock job_kill]', name);
  const job = currentJobs.find((j) => j.name === name);
  if (job) {
    job.state = 'killed';
    job.lastLine = 'Остановлено пользователем';
    emitJobs();
  }
}

export async function job_new(repo: string, name: string): Promise<string> {
  return `${repo}/${name}`;
}

export async function job_run(
  agent: 'codex' | 'gemini' | 'claude',
  worktree: string,
  taskFile: string,
  name?: string
): Promise<Job> {
  const newJob: Job = {
    name: name || worktree,
    agent,
    worktree,
    task: taskFile,
    startedAt: 'только что',
    state: 'running',
    lastLine: 'Инициализация задачи...',
    steps: [{ text: 'Запуск', done: true }],
  };
  currentJobs.push(newJob);
  emitJobs();
  return newJob;
}

export async function write_task(
  worktree: string,
  slug: string,
  _markdown: string
): Promise<string> {
  return `${worktree}/docs/TASK-${slug}.md`;
}

export async function limits(): Promise<Limit[]> {
  return [...currentLimits];
}

export async function setup_status(): Promise<Setup> {
  return JSON.parse(JSON.stringify(MOCK_SETUP));
}

export async function write_instructions(): Promise<string> {
  console.log('[mock write_instructions]');
  return '~/.orchestra/INSTRUCTIONS.md';
}

export async function conductor_start(_projectPath: string): Promise<{ sessionId?: string }> {
  return { sessionId: 'mock-session-001' };
}

export async function conductor_send(projectPath: string, text: string): Promise<void> {
  console.log('[mock conductor_send]', projectPath, text);
  // Эмулируем ответ дирижёра через stream-json
  setTimeout(() => {
    emit('conductor://event', {
      projectPath,
      event: {
        type: 'assistant',
        message: {
          id: `msg_${Date.now()}`,
          content: [
            {
              type: 'text',
              text: `Принял указание: «${text}». Проверяю ворктри и запускаю следующий такт в партитуре.`,
            },
          ],
        },
      },
    });
  }, 600);
}

export async function conductor_stop(_projectPath: string): Promise<void> {}

const CHAT_STORAGE_PREFIX = 'takt-mock-chat:';

/** В браузере история живёт в localStorage, чтобы переживать перезагрузку страницы. */
export async function chat_history(projectPath: string): Promise<HistoryMessage[]> {
  try {
    const stored = localStorage.getItem(CHAT_STORAGE_PREFIX + projectPath);
    if (stored) return JSON.parse(stored) as HistoryMessage[];
  } catch {}
  return [];
}

export async function chat_record(projectPath: string, message: HistoryMessage): Promise<void> {
  const messages = await chat_history(projectPath);
  const index = messages.findIndex((m) => m.id === message.id);
  if (index >= 0) messages[index] = message;
  else messages.push(message);
  try {
    localStorage.setItem(CHAT_STORAGE_PREFIX + projectPath, JSON.stringify(messages));
  } catch {}
}

const CONDUCTOR_STORAGE_KEY = 'takt-mock-conductor';

function readMockConductor(): ConductorAgent {
  try {
    const stored = localStorage.getItem(CONDUCTOR_STORAGE_KEY);
    if (stored === 'claude' || stored === 'codex' || stored === 'agy') return stored;
  } catch {}
  return 'claude';
}

/** В браузере: Claude и Codex найдены, Antigravity не установлен — чтобы видеть отключённую карточку. */
export async function conductor_config(): Promise<ConductorConfig> {
  return {
    agent: readMockConductor(),
    available: [
      { id: 'claude', found: true, loggedIn: true },
      { id: 'codex', found: true, loggedIn: true },
      { id: 'agy', found: false },
    ],
  };
}

export async function set_conductor(agent: ConductorAgent): Promise<void> {
  console.log('[mock set_conductor]', agent);
  try {
    localStorage.setItem(CONDUCTOR_STORAGE_KEY, agent);
  } catch {}
}

export async function pty_open(_cwd: string, _cols: number, _rows: number): Promise<string> {
  return 'mock-pty-001';
}

export async function pty_write(id: string, data: string): Promise<void> {
  // Эхо в мок-терминале
  emit('pty://data', { id, data });
}

export async function pty_resize(_id: string, _cols: number, _rows: number): Promise<void> {}

export async function pty_close(_id: string): Promise<void> {}

export async function open_in_terminal(cwd: string, action?: string): Promise<void> {
  console.log('[mock open_in_terminal]', cwd, action);
}

// ---- Модели ----
const mockSelection: ModelSelection = { claude: 'opus' };

const MOCK_MODELS: ModelsInfo = {
  path: '~/.orchestra/models.json',
  providers: [
    {
      id: 'claude',
      found: true,
      source: 'claude --help',
      models: [
        { id: 'fable', label: 'fable', popular: true, note: 'Fable 5.1' },
        { id: 'opus', label: 'opus', popular: true, note: 'Opus 5.5' },
        { id: 'sonnet', label: 'sonnet', popular: true, note: 'Sonnet 5.5' },
        { id: 'claude-fable-5-1', label: 'claude-fable-5-1', popular: false },
        { id: 'claude-opus-5-5', label: 'claude-opus-5-5', popular: false },
        { id: 'claude-sonnet-5-5', label: 'claude-sonnet-5-5', popular: false },
        { id: 'claude-haiku-5-5', label: 'claude-haiku-5-5', popular: false },
      ],
    },
    {
      id: 'codex',
      found: true,
      source: '~/.codex/models_cache.json',
      models: [
        { id: 'gpt-6-sol', label: 'gpt-6-sol', popular: true, note: 'low · medium · high · xhigh' },
        { id: 'gpt-6-sol-mini', label: 'gpt-6-sol-mini', popular: false, note: 'low · medium · high' },
        { id: 'gpt-5.5-codex', label: 'gpt-5.5-codex', popular: false, note: 'medium · high' },
      ],
    },
    {
      id: 'agy',
      found: true,
      source: 'agy models',
      models: [
        { id: 'gemini-3.8-pro-high', label: 'Gemini 3.8 Pro (High)', popular: true },
        { id: 'gemini-3.8-flash-high', label: 'Gemini 3.8 Flash (High)', popular: true },
        { id: 'gemini-3.8-flash', label: 'Gemini 3.8 Flash', popular: false },
      ],
    },
    {
      id: 'ollama',
      found: true,
      source: 'ollama list',
      models: [
        { id: 'qwen2.5-coder:7b', label: 'qwen2.5-coder:7b', popular: true, note: '4.7 GB' },
        { id: 'llama3.1:8b', label: 'llama3.1:8b', popular: false, note: '4.9 GB' },
      ],
    },
  ],
};

export async function list_models(): Promise<ModelsInfo> {
  await new Promise((resolve) => setTimeout(resolve, 400));
  const info = JSON.parse(JSON.stringify(MOCK_MODELS)) as ModelsInfo;
  for (const provider of info.providers) {
    provider.selected = mockSelection[provider.id];
  }
  return info;
}

export async function save_model(
  provider: ModelProviderId,
  model: string | null
): Promise<ModelSelection> {
  if (model) mockSelection[provider] = model;
  else delete mockSelection[provider];
  return { ...mockSelection };
}

// ---- Предпросмотр результата ----
const TAKT = '~/orchestra/takt';

function minutesAgo(minutes: number): string {
  return new Date(Date.now() - minutes * 60_000).toISOString();
}

/** Мок-пути → URL, которые отдаёт `pnpm dev` (папка docs/ лежит в корне Vite). */
const MOCK_FILE_URLS: Record<string, string> = {
  [`${TAKT}/terminal-pane/docs/screenshots/chat.png`]: '/docs/screenshots/chat.png',
  [`${TAKT}/main/docs/screenshots/chat.png`]: '/docs/screenshots/onboarding-welcome.png',
  [`${TAKT}/terminal-pane/docs/screenshots/settings.png`]: '/docs/screenshots/settings.png',
};

const MOCK_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 520 220" font-family="Montserrat, sans-serif" font-size="14">
  <rect width="520" height="220" fill="#f4f5f0"/>
  <g fill="#ffffff" stroke="#0b0d0c" stroke-width="1.5">
    <rect x="20" y="80" width="130" height="60" rx="10"/>
    <rect x="195" y="30" width="130" height="60" rx="10"/>
    <rect x="195" y="130" width="130" height="60" rx="10"/>
    <rect x="370" y="80" width="130" height="60" rx="10"/>
  </g>
  <g stroke="#3f5a00" stroke-width="2" fill="none">
    <path d="M150 110 L195 60"/><path d="M150 110 L195 160"/>
    <path d="M325 60 L370 110"/><path d="M325 160 L370 110"/>
  </g>
  <g fill="#0b0d0c" text-anchor="middle">
    <text x="85" y="115">React</text>
    <text x="260" y="65">ipc.ts</text>
    <text x="260" y="165">mock.ts</text>
    <text x="435" y="115">Tauri core</text>
  </g>
</svg>`;

const MOCK_TASK_MD = `# TASK: Терминал во вкладке «Код»

## Цель
Встроить **xterm** в раскладку чата: слева список ворктри, справа PTY-сессия \`zsh -l\`.

## Объём
- \`src/components/Terminal.tsx\` — обёртка над \`@xterm/xterm\` + \`addon-fit\`
- события \`pty://data\` и \`pty://exit\`
- ресайз по \`ResizeObserver\`

| Команда | Что делает |
|---|---|
| \`pty_open\` | открывает сессию |
| \`pty_write\` | пишет ввод |

## Приёмка
- [x] терминал печатает вывод
- [ ] ресайз без мерцания

<script>alert('xss')</script>
<img src=x onerror="alert('xss')">
`;

const MOCK_HTML = `<!doctype html><html><body style="font-family:system-ui;padding:24px;background:#fff;color:#111">
<h1>Отчёт CodeRabbit</h1><p>Major: <b>0</b>, minor: <b>2</b>.</p>
<ul><li>src/store.ts:120 — лишний ререндер</li><li>src/App.tsx:40 — неиспользуемый импорт</li></ul>
</body></html>`;

// Минимальный валидный PDF с одной строкой текста.
const MOCK_PDF = `%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 400 200]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj
4 0 obj<</Length 52>>stream
BT /F1 20 Tf 40 100 Td (Takt preview: PDF) Tj ET
endstream endobj
5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
trailer<</Root 1 0 R>>
%%EOF`;

const MOCK_TEXTS: Record<string, string> = {
  [`${TAKT}/terminal-pane/docs/TASK-ui.md`]: MOCK_TASK_MD,
  [`${TAKT}/pty-bridge/docs/architecture.svg`]: MOCK_SVG,
  [`${TAKT}/pty-bridge/coderabbit-report.html`]: MOCK_HTML,
};

const MOCK_DIFF = `diff --git a/src/App.tsx b/src/App.tsx
index 3f2c1a0..9b7e4d2 100644
--- a/src/App.tsx
+++ b/src/App.tsx
@@ -1,9 +1,10 @@
 import { useEffect } from 'react';
 import { Lead } from './components/Lead';
+import { Preview } from './components/Preview';
 import { Sidebar } from './components/Sidebar';
 import { UpdateBanner } from './components/UpdateBanner';
 import { About } from './screens/About';
 import { Chat } from './screens/Chat';
-import { Code } from './screens/Code';
+import { Code } from './screens/Code';
 import { Models } from './screens/Models';
 import { Orchestra } from './screens/Orchestra';
@@ -40,24 +41,26 @@ function App() {
   return (
     <div className="page">
       <Lead />
       <UpdateBanner />
${Array.from({ length: 14 }, (_, i) => `       {/* строка контекста ${i + 1} */}`).join('\n')}
       {screen === 'chat' && (
-        <div className="win" id="chat">
+        <div className="win with-preview" id="chat">
           <Sidebar />
           {tab === 'chat' && <Chat />}
           {tab === 'code' && <Code />}
           {tab === 'orchestra' && <Orchestra />}
+          <Preview />
         </div>
       )}
     </div>
   );
 }
`;

let previewPdfUrl: string | null = null;
function pdfUrl(): string {
  if (!previewPdfUrl) {
    previewPdfUrl = URL.createObjectURL(new Blob([MOCK_PDF], { type: 'application/pdf' }));
  }
  return previewPdfUrl;
}

let currentPreview: PreviewSource[] | null = null;
function previewList(): PreviewSource[] {
  currentPreview ??= [
    { kind: 'url', url: 'http://localhost:1420/', title: 'Vite · :1420', mtime: minutesAgo(1) },
    { kind: 'image', path: `${TAKT}/terminal-pane/docs/screenshots/chat.png`, title: 'chat.png', mtime: minutesAgo(4) },
    { kind: 'image', path: `${TAKT}/main/docs/screenshots/chat.png`, title: 'chat.png · main', mtime: minutesAgo(180) },
    { kind: 'doc', path: `${TAKT}/terminal-pane/docs/TASK-ui.md`, title: 'TASK-ui.md', mtime: minutesAgo(12) },
    { kind: 'doc', path: `${TAKT}/pty-bridge/docs/architecture.svg`, title: 'architecture.svg', mtime: minutesAgo(25) },
    { kind: 'doc', path: `${TAKT}/pty-bridge/coderabbit-report.html`, title: 'coderabbit-report.html', mtime: minutesAgo(40) },
    { kind: 'doc', path: `${TAKT}/pty-bridge/docs/spec.pdf`, title: 'spec.pdf', mtime: minutesAgo(65) },
    { kind: 'diff', path: `${TAKT}/pty-bridge/src/App.tsx`, title: 'src/App.tsx', mtime: minutesAgo(2) },
  ];
  return currentPreview;
}

let previewTimerStarted = false;
function ensurePreviewTimer(projectPath: string): void {
  if (previewTimerStarted || typeof window === 'undefined') return;
  previewTimerStarted = true;
  let n = 0;
  setInterval(() => {
    n += 1;
    const list = previewList();
    list.unshift({
      kind: 'image',
      path: `${TAKT}/terminal-pane/docs/screenshots/settings${n === 1 ? '' : `-${n}`}.png`,
      title: n === 1 ? 'settings.png' : `settings-${n}.png`,
      mtime: new Date().toISOString(),
    });
    emit('preview://changed', { projectPath, sources: JSON.parse(JSON.stringify(list)) });
  }, 20_000);
}

export async function preview_sources(projectPath: string): Promise<PreviewSource[]> {
  ensurePreviewTimer(projectPath);
  return JSON.parse(JSON.stringify(previewList()));
}

export async function preview_diff(_path: string, _worktree: string): Promise<string> {
  return MOCK_DIFF;
}

export async function preview_read(path: string): Promise<string> {
  const text = MOCK_TEXTS[path];
  if (text === undefined) throw new Error(`Файл не найден: ${path}`);
  return text;
}

export function preview_file_src(path: string): string {
  if (path.endsWith('.pdf')) return pdfUrl();
  return MOCK_FILE_URLS[path.replace(/settings-\d+\.png$/, 'settings.png')] ?? path;
}
