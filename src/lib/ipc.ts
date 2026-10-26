import { invoke as tauriInvoke, isTauri } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';
import * as mock from './mock';

// ---- Контракт SPEC.md §7 ----
export type AgentKind = 'codex' | 'gemini' | 'claude';
export type JobState = 'running' | 'asking' | 'limit' | 'done' | 'killed' | 'failed' | 'exited';

export interface Project {
  name: string;
  path: string;
  group?: string;
  color?: string;
}

export interface Step {
  text: string;
  done: boolean;
}

export interface Job {
  name: string;
  agent: AgentKind;
  worktree: string;
  task: string;
  startedAt: string;
  state: JobState;
  handedTo?: string;
  takeoverOf?: string;
  steps: Step[];
  lastLine: string;
  model?: string;
  question?: string;
  chips?: string[];
  logLines?: { time: string; text: string }[];
}

export interface Limit {
  provider: 'claude' | 'codex' | 'antigravity';
  sessionLeft?: number;
  weeklyLeft?: number;
  resetsIn?: string;
  raw: string;
}

export interface Tool {
  id: 'claude' | 'codex' | 'agy' | 'gh' | 'codexbar' | 'coderabbit' | 'tmux' | 'git' | 'agents';
  found: boolean;
  path?: string;
  version?: string;
  loggedIn?: boolean;
  account?: string;
  warn?: boolean;
  statusText?: string;
  agents?: { claude: boolean; codex: boolean; agy: boolean };
}

export interface SshHost {
  host: string;
  hostname?: string;
  user?: string;
  port?: number;
  identityFile?: string;
  status?: string;
  warn?: boolean;
  checked?: string;
  usedBy?: string;
  rule?: string;
  agents?: { claude: boolean; codex: boolean; agy: boolean };
}

export interface McpServer {
  name: string;
  source: 'codex' | 'agy';
  command?: string;
  url?: string;
}

export interface Setup {
  instructionsExists: boolean;
  projectsExists: boolean;
  tools: Tool[];
  ssh: SshHost[];
  mcp: McpServer[];
}

// ---- Модели (SPEC.md §10) ----
export type ModelProviderId = 'claude' | 'codex' | 'agy' | 'ollama';

export interface ModelInfo {
  id: string;
  label: string;
  popular: boolean;
  note?: string;
}

export interface ModelProvider {
  id: ModelProviderId;
  found: boolean;
  source: string;
  models: ModelInfo[];
  selected?: string;
  error?: string;
}

export interface ModelsInfo {
  path: string;
  providers: ModelProvider[];
}

export type ModelSelection = Record<string, string>;

export type UnlistenFn = () => void;

export function isTauriEnv(): boolean {
  if (typeof window === 'undefined') return false;
  const win = window as unknown as { __TAURI_INTERNALS__?: unknown; __TAURI__?: unknown };
  return Boolean(win.__TAURI_INTERNALS__ || win.__TAURI__ || isTauri());
}

// ---- Команды (invoke) ----
export async function list_projects(): Promise<Project[]> {
  if (isTauriEnv()) return tauriInvoke<Project[]>('list_projects');
  return mock.list_projects();
}

export async function save_projects(projects: Project[]): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('save_projects', { projects });
  return mock.save_projects(projects);
}

export async function list_jobs(): Promise<Job[]> {
  if (isTauriEnv()) return tauriInvoke<Job[]>('list_jobs');
  return mock.list_jobs();
}

export async function job_screen(name: string, lines?: number): Promise<string> {
  if (isTauriEnv()) return tauriInvoke<string>('job_screen', { name, lines });
  return mock.job_screen(name, lines);
}

export async function job_send(name: string, text: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('job_send', { name, text });
  return mock.job_send(name, text);
}

export async function job_kill(name: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('job_kill', { name });
  return mock.job_kill(name);
}

export async function job_new(repo: string, name: string): Promise<string> {
  if (isTauriEnv()) return tauriInvoke<string>('job_new', { repo, name });
  return mock.job_new(repo, name);
}

export async function job_run(
  agent: AgentKind,
  worktree: string,
  taskFile: string,
  name?: string
): Promise<Job> {
  if (isTauriEnv()) return tauriInvoke<Job>('job_run', { agent, worktree, taskFile, name });
  return mock.job_run(agent, worktree, taskFile, name);
}

export async function write_task(
  worktree: string,
  slug: string,
  markdown: string
): Promise<string> {
  if (isTauriEnv()) return tauriInvoke<string>('write_task', { worktree, slug, markdown });
  return mock.write_task(worktree, slug, markdown);
}

export async function limits(): Promise<Limit[]> {
  if (isTauriEnv()) return tauriInvoke<Limit[]>('limits');
  return mock.limits();
}

export async function setup_status(): Promise<Setup> {
  if (isTauriEnv()) return tauriInvoke<Setup>('setup_status');
  return mock.setup_status();
}

export async function write_instructions(): Promise<string> {
  if (isTauriEnv()) return tauriInvoke<string>('write_instructions');
  return mock.write_instructions();
}

export async function conductor_start(projectPath: string): Promise<{ sessionId?: string }> {
  if (isTauriEnv()) return tauriInvoke<{ sessionId?: string }>('conductor_start', { projectPath });
  return mock.conductor_start(projectPath);
}

export async function conductor_send(projectPath: string, text: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('conductor_send', { projectPath, text });
  return mock.conductor_send(projectPath, text);
}

export async function conductor_stop(projectPath: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('conductor_stop', { projectPath });
  return mock.conductor_stop(projectPath);
}

export async function pty_open(cwd: string, cols: number, rows: number): Promise<string> {
  if (isTauriEnv()) return tauriInvoke<string>('pty_open', { cwd, cols, rows });
  return mock.pty_open(cwd, cols, rows);
}

export async function pty_write(id: string, data: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('pty_write', { id, data });
  return mock.pty_write(id, data);
}

export async function pty_resize(id: string, cols: number, rows: number): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('pty_resize', { id, cols, rows });
  return mock.pty_resize(id, cols, rows);
}

export async function pty_close(id: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('pty_close', { id });
  return mock.pty_close(id);
}

/** `action` — ключ из белого списка ядра (`install:tmux`, `login:codex`), не произвольная команда. */
export async function open_in_terminal(cwd: string, action?: string): Promise<void> {
  if (isTauriEnv()) return tauriInvoke<void>('open_in_terminal', { cwd, action });
  return mock.open_in_terminal(cwd, action);
}

export async function list_models(): Promise<ModelsInfo> {
  if (isTauriEnv()) return tauriInvoke<ModelsInfo>('list_models');
  return mock.list_models();
}

export async function save_model(
  provider: ModelProviderId,
  model: string | null
): Promise<ModelSelection> {
  if (isTauriEnv()) return tauriInvoke<ModelSelection>('save_model', { provider, model });
  return mock.save_model(provider, model);
}

// ---- События (listen) ----
export async function listen<T>(
  event: string,
  handler: (event: { event: string; payload: T }) => void
): Promise<UnlistenFn> {
  if (isTauriEnv()) {
    return tauriListen<T>(event, handler);
  }
  return mock.listen<T>(event, handler);
}
