import type {
  Job,
  Limit,
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

export async function pty_open(_cwd: string, _cols: number, _rows: number): Promise<string> {
  return 'mock-pty-001';
}

export async function pty_write(id: string, data: string): Promise<void> {
  // Эхо в мок-терминале
  emit('pty://data', { id, data });
}

export async function pty_resize(_id: string, _cols: number, _rows: number): Promise<void> {}

export async function pty_close(_id: string): Promise<void> {}

export async function open_in_terminal(cwd: string, command?: string): Promise<void> {
  console.log('[mock open_in_terminal]', cwd, command);
}
