import { create } from 'zustand';
import {
  CONDUCTOR_AGENTS,
  chat_history,
  chat_record,
  conductor_config,
  conductor_send,
  isTauriEnv,
  job_kill,
  job_send,
  limits as apiLimits,
  list_jobs,
  list_projects,
  listen,
  preview_sources,
  set_conductor,
  setup_status,
  type ConductorAgent,
  type ConductorOption,
  type HistoryMessage,
  type Job,
  type Limit,
  type PreviewSource,
  type Project,
  type Setup,
} from './lib/ipc';
import {
  checkForUpdate,
  discardUpdate,
  installUpdate as runUpdateInstall,
  type UpdateInfo,
} from './lib/updater';
import { getLocale, setLocale as applyLocale, t, type Locale } from './i18n';

export type ScreenType = 'chat' | 'start' | 'about' | 'models' | 'settings' | 'onboarding';
export type TabType = 'chat' | 'code' | 'orchestra';
export type ThemeType = 'light' | 'dark';
export type UpdateCheckStatus = 'idle' | 'checking' | 'current' | 'available' | 'error';
export type PreviewDevice = 'phone' | 'tablet' | 'desktop';

const UPDATE_CHECK_INTERVAL = 24 * 60 * 60 * 1000;
const UPDATE_CHECKS_STORAGE_KEY = 'takt-update-checks';
const ONBOARDED_STORAGE_KEY = 'takt-onboarded';
const PREVIEW_STORAGE_PREFIX = 'takt-preview:';
const PREVIEW_MIN_WIDTH = 320;
const PREVIEW_MAX_SHARE = 0.6;
const PREVIEW_DEFAULT_SHARE = 0.4;

let dismissedUpdateVersion: string | null = null;

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'tool';
  text: string;
  time?: string;
  who?: string;
  /** Момент сообщения в RFC 3339 — по нему считается `time`. */
  at?: string;
}

/** Подпись времени: сегодня — только часы и минуты, иначе ещё и дата. */
function formatTime(at: string | undefined, locale: Locale): string | undefined {
  if (!at) return undefined;
  const date = new Date(at);
  if (Number.isNaN(date.getTime())) return undefined;
  const sameDay = date.toDateString() === new Date().toDateString();
  const options: Intl.DateTimeFormatOptions = sameDay
    ? { hour: '2-digit', minute: '2-digit' }
    : { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' };
  return date.toLocaleString(locale, options);
}

function fromHistory(message: HistoryMessage, locale: Locale, conductor: ConductorAgent): ChatMessage {
  return {
    id: message.id,
    role: message.role,
    text: message.text,
    at: message.at,
    time: message.role === 'user' ? formatTime(message.at, locale) : undefined,
    who: message.role === 'assistant' ? conductorName(conductor) : undefined,
  };
}

function toHistory(message: ChatMessage): HistoryMessage {
  return { id: message.id, role: message.role, text: message.text, at: message.at };
}

/** Запись в историю не должна ломать ленту: ошибку только логируем. */
function record(projectPath: string, message: ChatMessage): void {
  if (!projectPath) return;
  chat_record(projectPath, toHistory(message)).catch((err: unknown) => {
    console.warn('[takt] chat_record', err);
  });
}

interface OrchestraState {
  screen: ScreenType;
  tab: TabType;
  theme: ThemeType;
  locale: Locale;
  projects: Project[];
  activeProject: Project | null;
  activeWorktree: string;
  jobs: Job[];
  limits: Limit[];
  messages: ChatMessage[];
  setup: Setup | null;
  /** Кто дирижирует; читается из ~/.orchestra/conductor.json, по умолчанию Claude. */
  conductorAgent: ConductorAgent;
  conductorOptions: ConductorOption[];
  error: string | null;
  update: UpdateInfo | null;
  updateChecksEnabled: boolean;
  updateProgress: number | null;
  updateError: string | null;
  updateCheckStatus: UpdateCheckStatus;
  updateCheckMessage: string | null;
  onboarded: boolean;
  setupRefreshing: boolean;
  previewOpen: boolean;
  previewWidth: number;
  previewSources: PreviewSource[];
  previewActive: PreviewSource | null;
  previewDevice: PreviewDevice;
  /** Сколько источников появилось, пока панель была свёрнута (бейдж на полоске). */
  previewNew: number;

  setScreen: (screen: ScreenType) => void;
  completeOnboarding: () => void;
  restartOnboarding: () => void;
  refreshSetup: () => Promise<void>;
  refreshConductor: () => Promise<void>;
  setConductor: (agent: ConductorAgent) => Promise<void>;
  setTab: (tab: TabType) => void;
  setTheme: (theme: ThemeType) => void;
  toggleTheme: () => void;
  setLocale: (locale: Locale) => void;
  setActiveProject: (project: Project) => void;
  /** Загружает ленту чата проекта из ~/.orchestra/chats. */
  loadHistory: (projectPath: string) => Promise<void>;
  setActiveWorktree: (worktree: string) => void;
  sendUserMessage: (text: string) => Promise<void>;
  sendJobAnswer: (jobName: string, answer: string) => Promise<void>;
  killJob: (jobName: string) => Promise<void>;
  checkUpdate: (manual?: boolean) => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  setUpdateChecks: (enabled: boolean) => void;
  togglePreview: () => void;
  setPreviewWidth: (width: number) => void;
  setPreviewDevice: (device: PreviewDevice) => void;
  openPreview: (source: PreviewSource) => void;
  loadPreviewSources: () => Promise<void>;
  init: () => Promise<() => void>;
}

function getInitialTheme(): ThemeType {
  if (typeof window === 'undefined') return 'light';
  try {
    const saved = localStorage.getItem('orch-theme');
    if (saved === 'light' || saved === 'dark') return saved;
  } catch (e) {}
  if (typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

function getInitialUpdateChecks(): boolean {
  if (typeof window === 'undefined') return true;
  try {
    return localStorage.getItem(UPDATE_CHECKS_STORAGE_KEY) !== 'false';
  } catch {
    return true;
  }
}

/** Мастер первого запуска показан один раз; флаг живёт в localStorage WebView. */
function getInitialOnboarded(): boolean {
  if (typeof window === 'undefined') return true;
  try {
    return localStorage.getItem(ONBOARDED_STORAGE_KEY) === 'true';
  } catch {
    return true;
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const CONDUCTOR_NAMES: Record<ConductorAgent, string> = {
  claude: 'Claude',
  codex: 'Codex',
  agy: 'Antigravity',
};

/** Короткое имя дирижёра для подписи в чате и партитуре. */
export function conductorName(agent: ConductorAgent): string {
  return CONDUCTOR_NAMES[agent];
}

/** Первый доступный по порядку claude → codex → agy; если никого нет — Claude. */
export function firstAvailableConductor(options: ConductorOption[]): ConductorAgent {
  for (const id of CONDUCTOR_AGENTS) {
    const option = options.find((item) => item.id === id);
    if (option && option.found && option.loggedIn !== false) return id;
  }
  return 'claude';
}

/** Демо-диалог вне Tauri; текст берётся из словаря текущего языка. */
function initialMessages(): ChatMessage[] {
  return [
    { id: 'init_user_1', role: 'user', text: t('demo.user'), time: t('chat.justNow') },
    { id: 'init_assistant_1', role: 'assistant', who: conductorName('claude'), text: t('demo.assistant') },
  ];
}

/** Ключ источника: путь файла или URL — одинаков между перезапусками. */
export function previewKey(source: PreviewSource): string {
  return `${source.kind}:${source.path ?? source.url ?? source.title}`;
}

function windowWidth(): number {
  return typeof window === 'undefined' ? 1280 : window.innerWidth;
}

export function clampPreviewWidth(width: number): number {
  const max = Math.max(PREVIEW_MIN_WIDTH, Math.round(windowWidth() * PREVIEW_MAX_SHARE));
  return Math.min(max, Math.max(PREVIEW_MIN_WIDTH, Math.round(width)));
}

function readSavedPreview(projectPath: string): string | null {
  try {
    return localStorage.getItem(PREVIEW_STORAGE_PREFIX + projectPath);
  } catch {
    return null;
  }
}

function savePreview(projectPath: string, source: PreviewSource): void {
  try {
    localStorage.setItem(PREVIEW_STORAGE_PREFIX + projectPath, previewKey(source));
  } catch {}
}

/** Источники из ворктри задачи: путь файла лежит внутри `job.worktree`. */
export function sourcesForJob(job: Job, sources: PreviewSource[]): PreviewSource[] {
  const wt = job.worktree.replace(/\/+$/, '');
  if (!wt) return [];
  return sources.filter((source) => {
    if (!source.path) return false;
    if (wt.includes('/')) return source.path.startsWith(`${wt}/`);
    return source.path.split('/').includes(wt);
  });
}

const initialOnboarded = getInitialOnboarded();

export const useStore = create<OrchestraState>((set, get) => ({
  screen: initialOnboarded ? 'chat' : 'onboarding',
  onboarded: initialOnboarded,
  setupRefreshing: false,
  tab: 'chat',
  theme: getInitialTheme(),
  locale: getLocale(),
  projects: [],
  activeProject: null,
  activeWorktree: 'pty-bridge',
  jobs: [],
  limits: [],
  messages: isTauriEnv() ? [] : initialMessages(),
  setup: null,
  conductorAgent: 'claude',
  conductorOptions: [],
  error: null,
  update: null,
  updateChecksEnabled: getInitialUpdateChecks(),
  updateProgress: null,
  updateError: null,
  updateCheckStatus: 'idle',
  updateCheckMessage: null,
  previewOpen: false,
  previewWidth: clampPreviewWidth(windowWidth() * PREVIEW_DEFAULT_SHARE),
  previewSources: [],
  previewActive: null,
  previewDevice: 'desktop',
  previewNew: 0,

  setScreen: (screen: ScreenType) => set({ screen }),
  setTab: (tab: TabType) => set({ tab }),

  completeOnboarding: () => {
    try {
      localStorage.setItem(ONBOARDED_STORAGE_KEY, 'true');
    } catch {}
    set({ onboarded: true, screen: 'chat', tab: 'chat' });
  },

  restartOnboarding: () => {
    try {
      localStorage.removeItem(ONBOARDED_STORAGE_KEY);
    } catch {}
    set({ onboarded: false, screen: 'onboarding' });
  },

  refreshSetup: async () => {
    if (get().setupRefreshing) return;
    set({ setupRefreshing: true });
    try {
      const [setup, projects] = await Promise.all([setup_status(), list_projects()]);
      const hadProject = get().activeProject !== null;
      set((state) => ({
        setup,
        projects,
        activeProject: state.activeProject ?? (projects.length > 0 ? projects[0] : null),
      }));
      const active = get().activeProject;
      if (!hadProject && active) void get().loadHistory(active.path);
    } catch (err: unknown) {
      set({ error: errorMessage(err) });
    } finally {
      set({ setupRefreshing: false });
    }
  },
  setActiveWorktree: (activeWorktree: string) => set({ activeWorktree }),

  refreshConductor: async () => {
    try {
      const config = await conductor_config();
      set({ conductorAgent: config.agent, conductorOptions: config.available });
    } catch (err: unknown) {
      set({ error: errorMessage(err) });
    }
  },

  setConductor: async (agent: ConductorAgent) => {
    const previous = get().conductorAgent;
    set({ conductorAgent: agent });
    try {
      await set_conductor(agent);
    } catch (err: unknown) {
      set({ conductorAgent: previous, error: errorMessage(err) });
    }
  },

  setTheme: (theme: ThemeType) => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = theme;
      try {
        localStorage.setItem('orch-theme', theme);
      } catch (e) {}
    }
    set({ theme });
  },

  toggleTheme: () => {
    const current = get().theme;
    const next: ThemeType = current === 'dark' ? 'light' : 'dark';
    get().setTheme(next);
  },

  setLocale: (locale: Locale) => {
    applyLocale(locale);
    set((state) => {
      // Демо-сообщения перевести вслед за языком, живые — не трогать.
      const isDemo = state.messages.some((m) => m.id === 'init_user_1');
      return { locale, messages: isDemo && !isTauriEnv() ? initialMessages() : state.messages };
    });
  },

  setActiveProject: (activeProject: Project) => {
    const changed = get().activeProject?.path !== activeProject.path;
    set({ activeProject, previewSources: [], previewActive: null, previewNew: 0 });
    if (changed) {
      set({ messages: [] });
      void get().loadHistory(activeProject.path);
    }
    void get().loadPreviewSources();
  },

  loadHistory: async (projectPath: string) => {
    try {
      const history = await chat_history(projectPath);
      const state = get();
      if (state.activeProject?.path !== projectPath) return;
      set({ messages: history.map((m) => fromHistory(m, state.locale, state.conductorAgent)) });
    } catch (err: unknown) {
      set({ error: errorMessage(err) });
    }
  },

  sendUserMessage: async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;

    const at = new Date().toISOString();
    const userMsg: ChatMessage = {
      id: `usr_${Date.now()}`,
      role: 'user',
      text: trimmed,
      at,
      time: formatTime(at, get().locale) ?? t('chat.justNow'),
    };

    set((state) => ({
      messages: [...state.messages, userMsg],
      error: null,
    }));

    const project = get().activeProject;
    const projectPath = project?.path || '';
    record(projectPath, userMsg);

    try {
      await conductor_send(projectPath, trimmed);
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  sendJobAnswer: async (jobName: string, answer: string) => {
    try {
      await job_send(jobName, answer);
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  killJob: async (jobName: string) => {
    try {
      await job_kill(jobName);
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  checkUpdate: async (manual = false) => {
    if (get().updateCheckStatus === 'checking') return;

    const existingUpdate = get().update;
    if (existingUpdate) {
      set({
        updateCheckStatus: 'available',
        updateCheckMessage: manual ? t('update.available', { version: existingUpdate.version }) : null,
      });
      return;
    }

    // Ручная проверка снимает отложенную версию: пользователь явно хочет её увидеть.
    if (manual) dismissedUpdateVersion = null;

    set({ updateCheckStatus: 'checking', updateCheckMessage: null });
    try {
      const update = await checkForUpdate();
      if (update && update.version !== dismissedUpdateVersion) {
        set({
          update,
          updateError: null,
          updateCheckStatus: 'available',
          updateCheckMessage: manual ? t('update.available', { version: update.version }) : null,
        });
        return;
      }

      if (update) {
        // Автопроверка: версия отложена кнопкой «Позже», до перезапуска не показываем.
        await discardUpdate();
        set({ update: null, updateCheckStatus: 'available', updateCheckMessage: null });
        return;
      }

      set({
        update: null,
        updateCheckStatus: 'current',
        updateCheckMessage: manual ? t('update.current') : null,
      });
    } catch (err: unknown) {
      const message = errorMessage(err);
      console.info(`${t('update.logPrefix')}: ${message}`);
      set({
        updateCheckStatus: 'error',
        updateCheckMessage: manual ? t('update.checkFailed', { message }) : null,
      });
    }
  },

  installUpdate: async () => {
    if (!get().update) return;

    set({ updateProgress: 0, updateError: null });
    try {
      await runUpdateInstall((updateProgress) => set({ updateProgress }));
    } catch (err: unknown) {
      set({
        updateProgress: null,
        updateError: t('update.installFailed', { message: errorMessage(err) }),
      });
    }
  },

  dismissUpdate: () => {
    dismissedUpdateVersion = get().update?.version ?? null;
    set({ update: null, updateProgress: null, updateError: null });
    void discardUpdate().catch((err: unknown) => {
      console.info(`${t('update.logPrefix')}: ${errorMessage(err)}`);
    });
  },

  setUpdateChecks: (updateChecksEnabled: boolean) => {
    try {
      localStorage.setItem(UPDATE_CHECKS_STORAGE_KEY, String(updateChecksEnabled));
    } catch {}
    set({ updateChecksEnabled, updateCheckMessage: null });
  },

  togglePreview: () =>
    set((state) => ({
      previewOpen: !state.previewOpen,
      previewNew: 0,
      previewActive: state.previewActive ?? state.previewSources[0] ?? null,
    })),

  setPreviewWidth: (width: number) => set({ previewWidth: clampPreviewWidth(width) }),

  setPreviewDevice: (previewDevice: PreviewDevice) => set({ previewDevice }),

  openPreview: (source: PreviewSource) => {
    const projectPath = get().activeProject?.path;
    if (projectPath) savePreview(projectPath, source);
    set({ previewActive: source, previewOpen: true, previewNew: 0 });
  },

  loadPreviewSources: async () => {
    const projectPath = get().activeProject?.path;
    if (!projectPath) return;
    try {
      const sources = await preview_sources(projectPath);
      if (get().activeProject?.path !== projectPath) return;
      const saved = readSavedPreview(projectPath);
      const restored = saved ? sources.find((s) => previewKey(s) === saved) : undefined;
      set((state) => ({
        previewSources: sources,
        previewActive: restored ?? state.previewActive ?? sources[0] ?? null,
      }));
    } catch (err: unknown) {
      set({ error: errorMessage(err) });
    }
  },

  init: async () => {
    // 1. Применяем тему
    const th = get().theme;
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = th;
    }

    if (get().updateChecksEnabled) {
      void get().checkUpdate();
    }

    const updateTimer = setInterval(() => {
      if (get().updateChecksEnabled) {
        void get().checkUpdate();
      }
    }, UPDATE_CHECK_INTERVAL);

    // 2. Загружаем данные
    try {
      const [projs, jbs, lmts, stp, conductor] = await Promise.all([
        list_projects(),
        list_jobs(),
        apiLimits(),
        setup_status(),
        conductor_config(),
      ]);

      const active = projs.length > 0 ? projs[0] : null;
      set({
        projects: projs,
        activeProject: active,
        jobs: jbs,
        limits: lmts,
        setup: stp,
        conductorAgent: conductor.agent,
        conductorOptions: conductor.available,
      });
      void get().loadPreviewSources();
    } catch (err: unknown) {
      console.error('Failed to load initial data:', err);
      set({ error: err instanceof Error ? err.message : String(err) });
    }

    // 3. Подписка на события
    const unlisteners: (() => void)[] = [];

    const unlistenJobs = await listen<Job[]>('jobs://changed', (event) => {
      set({ jobs: event.payload });
    });
    unlisteners.push(unlistenJobs);

    const unlistenLimits = await listen<Limit[]>('limits://changed', (event) => {
      set({ limits: event.payload });
    });
    unlisteners.push(unlistenLimits);

    const unlistenPreview = await listen<{ projectPath: string; sources: PreviewSource[] }>(
      'preview://changed',
      (e) => {
        const { projectPath } = e.payload;
        const seen = new Set<string>();
        const sources = e.payload.sources.filter((s) => {
          const key = previewKey(s);
          if (seen.has(key)) return false;
          seen.add(key);
          return true;
        });
        set((state) => {
          if (state.activeProject?.path !== projectPath) return {};
          const known = new Set(state.previewSources.map(previewKey));
          const added = sources.filter((s) => !known.has(previewKey(s))).length;
          const activeKey = state.previewActive ? previewKey(state.previewActive) : null;
          return {
            previewSources: sources,
            previewActive:
              (activeKey && sources.find((s) => previewKey(s) === activeKey)) ||
              state.previewActive ||
              sources[0] ||
              null,
            previewNew: state.previewOpen ? 0 : state.previewNew + added,
          };
        });
      }
    );
    unlisteners.push(unlistenPreview);

    const unlistenConductor = await listen<{ projectPath: string; event: unknown }>(
      'conductor://event',
      (e) => {
        const payload = e.payload?.event as
          | {
              type?: string;
              message?: {
                id?: string;
                content?: Array<
                  | { type: 'text'; text: string }
                  | { type: 'tool_use'; name: string; input: Record<string, unknown> }
                >;
              };
            }
          | undefined;

        if (!payload || !payload.type) return;
        const eventProject = e.payload.projectPath || '';
        // Событие чужого проекта в ленту не попадает, но в его историю записывается.
        const isActive = !eventProject || get().activeProject?.path === eventProject;
        const at = new Date().toISOString();

        if (payload.type === 'assistant' && payload.message?.content) {
          const msgId = payload.message.id || `msg_${Date.now()}`;
          const textBlocks = payload.message.content
            .filter((b): b is { type: 'text'; text: string } => b.type === 'text')
            .map((b) => b.text)
            .join('\n\n');

          if (textBlocks) {
            const assistantMsg: ChatMessage = {
              id: msgId,
              role: 'assistant',
              who: conductorName(get().conductorAgent),
              text: textBlocks,
              at,
            };
            record(eventProject, assistantMsg);
            if (isActive) {
              set((state) => {
                const existingIdx = state.messages.findIndex((m) => m.id === msgId);
                if (existingIdx >= 0) {
                  const updated = [...state.messages];
                  updated[existingIdx] = {
                    ...updated[existingIdx],
                    text: textBlocks,
                  };
                  return { messages: updated };
                }
                return { messages: [...state.messages, assistantMsg] };
              });
            }
          }

          for (const block of payload.message.content) {
            if (block.type === 'tool_use') {
              let label = t('tool.calls', { name: block.name });
              if (block.name === 'read_file' && block.input?.path) {
                label = t('tool.reads', { path: String(block.input.path) });
              } else if (block.name === 'run_command' && block.input?.CommandLine) {
                label = t('tool.runs', { command: String(block.input.CommandLine) });
              }
              const toolMsg: ChatMessage = {
                id: `tool_${Date.now()}_${Math.random()}`,
                role: 'tool',
                text: label,
                at,
              };
              record(eventProject, toolMsg);
              if (isActive) {
                set((state) => ({ messages: [...state.messages, toolMsg] }));
              }
            }
          }
        }
      }
    );
    unlisteners.push(unlistenConductor);

    return () => {
      clearInterval(updateTimer);
      unlisteners.forEach((u) => u());
    };
  },
}));
