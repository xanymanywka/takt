import { create } from 'zustand';
import {
  conductor_send,
  isTauriEnv,
  job_kill,
  job_send,
  limits as apiLimits,
  list_jobs,
  list_projects,
  listen,
  setup_status,
  type Job,
  type Limit,
  type Project,
  type Setup,
} from './lib/ipc';
import {
  checkForUpdate,
  discardUpdate,
  installUpdate as runUpdateInstall,
  type UpdateInfo,
} from './lib/updater';

export type ScreenType = 'chat' | 'start';
export type TabType = 'chat' | 'code' | 'orchestra';
export type ThemeType = 'light' | 'dark';
export type UpdateCheckStatus = 'idle' | 'checking' | 'current' | 'available' | 'error';

const UPDATE_CHECK_INTERVAL = 24 * 60 * 60 * 1000;
const UPDATE_CHECKS_STORAGE_KEY = 'takt-update-checks';

let dismissedUpdateVersion: string | null = null;

export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant' | 'tool';
  text: string;
  time?: string;
  who?: string;
}

interface OrchestraState {
  screen: ScreenType;
  tab: TabType;
  theme: ThemeType;
  projects: Project[];
  activeProject: Project | null;
  activeWorktree: string;
  jobs: Job[];
  limits: Limit[];
  messages: ChatMessage[];
  setup: Setup | null;
  error: string | null;
  update: UpdateInfo | null;
  updateChecksEnabled: boolean;
  updateProgress: number | null;
  updateError: string | null;
  updateCheckStatus: UpdateCheckStatus;
  updateCheckMessage: string | null;

  setScreen: (screen: ScreenType) => void;
  setTab: (tab: TabType) => void;
  setTheme: (theme: ThemeType) => void;
  toggleTheme: () => void;
  setActiveProject: (project: Project) => void;
  setActiveWorktree: (worktree: string) => void;
  sendUserMessage: (text: string) => Promise<void>;
  sendJobAnswer: (jobName: string, answer: string) => Promise<void>;
  killJob: (jobName: string) => Promise<void>;
  checkUpdate: (manual?: boolean) => Promise<void>;
  installUpdate: () => Promise<void>;
  dismissUpdate: () => void;
  setUpdateChecks: (enabled: boolean) => void;
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

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const INITIAL_MESSAGES: ChatMessage[] = [
  {
    id: 'init_user_1',
    role: 'user',
    text: 'Добавь мост к терминалам: Codex и agy должны идти через PTY, а не через голый stdout, иначе теряем цвет и ввод.',
    time: 'только что',
  },
  {
    id: 'init_assistant_1',
    role: 'assistant',
    who: 'Claude',
    text: 'Понял. Это две независимые части, поэтому разведу их по разным ворктри: Codex делает PTY-мост на Rust-стороне, Antigravity рисует панель терминала в интерфейсе. Себе оставляю план, ревью и мёрж.',
  },
];

export const useStore = create<OrchestraState>((set, get) => ({
  screen: 'chat',
  tab: 'chat',
  theme: getInitialTheme(),
  projects: [],
  activeProject: null,
  activeWorktree: 'pty-bridge',
  jobs: [],
  limits: [],
  messages: isTauriEnv() ? [] : INITIAL_MESSAGES,
  setup: null,
  error: null,
  update: null,
  updateChecksEnabled: getInitialUpdateChecks(),
  updateProgress: null,
  updateError: null,
  updateCheckStatus: 'idle',
  updateCheckMessage: null,

  setScreen: (screen: ScreenType) => set({ screen }),
  setTab: (tab: TabType) => set({ tab }),
  setActiveWorktree: (activeWorktree: string) => set({ activeWorktree }),

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

  setActiveProject: (activeProject: Project) => set({ activeProject }),

  sendUserMessage: async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;

    const userMsg: ChatMessage = {
      id: `usr_${Date.now()}`,
      role: 'user',
      text: trimmed,
      time: 'только что',
    };

    set((state) => ({
      messages: [...state.messages, userMsg],
      error: null,
    }));

    const project = get().activeProject;
    const projectPath = project?.path || '';

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
        updateCheckMessage: manual ? `Доступна версия ${existingUpdate.version}` : null,
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
          updateCheckMessage: manual ? `Доступна версия ${update.version}` : null,
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
        updateCheckMessage: manual ? 'Установлена актуальная версия' : null,
      });
    } catch (err: unknown) {
      const message = errorMessage(err);
      console.info(`Обновления: ${message}`);
      set({
        updateCheckStatus: 'error',
        updateCheckMessage: manual ? `Не удалось проверить обновления: ${message}` : null,
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
        updateError: `Не удалось установить обновление: ${errorMessage(err)}`,
      });
    }
  },

  dismissUpdate: () => {
    dismissedUpdateVersion = get().update?.version ?? null;
    set({ update: null, updateProgress: null, updateError: null });
    void discardUpdate().catch((err: unknown) => {
      console.info(`Обновления: ${errorMessage(err)}`);
    });
  },

  setUpdateChecks: (updateChecksEnabled: boolean) => {
    try {
      localStorage.setItem(UPDATE_CHECKS_STORAGE_KEY, String(updateChecksEnabled));
    } catch {}
    set({ updateChecksEnabled, updateCheckMessage: null });
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
      const [projs, jbs, lmts, stp] = await Promise.all([
        list_projects(),
        list_jobs(),
        apiLimits(),
        setup_status(),
      ]);

      const active = projs.length > 0 ? projs[0] : null;
      set({
        projects: projs,
        activeProject: active,
        jobs: jbs,
        limits: lmts,
        setup: stp,
      });
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

        if (payload.type === 'assistant' && payload.message?.content) {
          const msgId = payload.message.id || `msg_${Date.now()}`;
          const textBlocks = payload.message.content
            .filter((b): b is { type: 'text'; text: string } => b.type === 'text')
            .map((b) => b.text)
            .join('\n\n');

          if (textBlocks) {
            set((state) => {
              const existingIdx = state.messages.findIndex((m) => m.id === msgId);
              if (existingIdx >= 0) {
                const updated = [...state.messages];
                updated[existingIdx] = {
                  ...updated[existingIdx],
                  text: textBlocks,
                };
                return { messages: updated };
              } else {
                return {
                  messages: [
                    ...state.messages,
                    {
                      id: msgId,
                      role: 'assistant',
                      who: 'Claude',
                      text: textBlocks,
                    },
                  ],
                };
              }
            });
          }

          for (const block of payload.message.content) {
            if (block.type === 'tool_use') {
              let label = `вызывает ${block.name}`;
              if (block.name === 'read_file' && block.input?.path) {
                label = `читает ${String(block.input.path)}`;
              } else if (block.name === 'run_command' && block.input?.CommandLine) {
                label = `запускает ${String(block.input.CommandLine)}`;
              }
              set((state) => ({
                messages: [
                  ...state.messages,
                  {
                    id: `tool_${Date.now()}_${Math.random()}`,
                    role: 'tool',
                    text: label,
                  },
                ],
              }));
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
