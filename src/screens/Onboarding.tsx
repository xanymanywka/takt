import React, { useEffect, useState } from 'react';
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  CheckIcon,
  CircleTodoIcon,
  CodeRabbitIcon,
  DotIcon,
  GhIcon,
  MoonIcon,
  ServerHostIcon,
  SunIcon,
  TerminalIcon,
} from '../components/Icons';
import { LOCALES, useLocale, useT, type Locale } from '../i18n';
import { open_in_terminal, write_instructions, type Tool } from '../lib/ipc';
import { useStore, type ThemeType } from '../store';

type StepId = 'welcome' | 'tools' | 'projects' | 'access' | 'instructions' | 'done';

const STEPS: StepId[] = ['welcome', 'tools', 'projects', 'access', 'instructions', 'done'];

/** Команда входа для каждого инструмента; пользователь выполняет её сам в терминале. */
const LOGIN_COMMAND: Partial<Record<Tool['id'], string>> = {
  claude: 'claude',
  codex: 'codex login',
  agy: 'agy login',
  gh: 'gh auth login',
  coderabbit: 'coderabbit auth login',
};

const INSTALL_COMMAND: Partial<Record<Tool['id'], string>> = {
  claude: 'npm i -g @anthropic-ai/claude-code',
  codex: 'npm i -g @openai/codex',
  gh: 'brew install gh',
  codexbar: 'brew install codexbar',
  coderabbit: 'curl -fsSL https://cli.coderabbit.ai/install.sh | sh',
  tmux: 'brew install tmux',
  git: 'xcode-select --install',
};

export const Onboarding: React.FC = () => {
  const t = useT();
  const locale = useLocale();
  const theme = useStore((state) => state.theme);
  const setTheme = useStore((state) => state.setTheme);
  const setLocale = useStore((state) => state.setLocale);
  const setup = useStore((state) => state.setup);
  const projects = useStore((state) => state.projects);
  const refreshSetup = useStore((state) => state.refreshSetup);
  const setupRefreshing = useStore((state) => state.setupRefreshing);
  const completeOnboarding = useStore((state) => state.completeOnboarding);

  const [index, setIndex] = useState(0);
  const [instructionsCreated, setInstructionsCreated] = useState(false);
  const step = STEPS[index];
  const total = STEPS.length;

  // Попав на шаг инструментов, перечитываем состояние: пользователь мог войти в терминале.
  useEffect(() => {
    if (step === 'tools' || step === 'instructions') void refreshSetup();
  }, [step, refreshSetup]);

  const tools = setup?.tools ?? [];
  const sshHosts = setup?.ssh ?? [];
  const mcpList = setup?.mcp ?? [];
  const missing = tools.filter((tool) => !tool.found);
  const loggedOut = tools.filter((tool) => tool.found && tool.loggedIn === false);
  const instructionsReady = instructionsCreated || Boolean(setup?.instructionsExists);

  const next = () => setIndex((i) => Math.min(i + 1, total - 1));
  const back = () => setIndex((i) => Math.max(i - 1, 0));

  // В ядро уходит только ключ действия; сама команда берётся из белого списка в setup.rs.
  const runInTerminal = async (action?: string) => {
    try {
      await open_in_terminal('~', action);
    } catch (err) {
      console.error(err);
    }
  };

  const createInstructions = async () => {
    try {
      await write_instructions();
      setInstructionsCreated(true);
      void refreshSetup();
    } catch (err) {
      console.error(err);
    }
  };

  const toolIcon = (id: string) => {
    if (id === 'gh') return <GhIcon />;
    if (id === 'coderabbit') return <CodeRabbitIcon />;
    return <TerminalIcon />;
  };

  const stepState = (i: number): 'done' | 'now' | 'todo' => (i < index ? 'done' : i === index ? 'now' : 'todo');

  const themes: Array<{ key: ThemeType; label: string; icon: React.ReactNode }> = [
    { key: 'light', label: t('settings.theme.light'), icon: <SunIcon size={14} /> },
    { key: 'dark', label: t('settings.theme.dark'), icon: <MoonIcon size={14} /> },
  ];

  const renderStep = () => {
    switch (step) {
      case 'welcome':
        return (
          <div className="wiz-step" key="welcome">
            <h2>{t('onb.welcome.title')}</h2>
            <p className="sub">
              {t('onb.welcome.sub1')} <i>{t('onb.welcome.sub2')}</i> {t('onb.welcome.sub3')}
            </p>
            <div className="wiz-hero">
              <div className="about-mark" aria-hidden="true">
                T
              </div>
              <p>{t('onb.welcome.text')}</p>
            </div>
            <div className="settings-grid">
              <div className="grp">
                <h3>
                  {t('settings.language.title')} <span>{t('settings.language.sub')}</span>
                </h3>
                <div className="choice-row" role="radiogroup" aria-label={t('settings.language.title')}>
                  {LOCALES.map((code: Locale) => (
                    <button
                      key={code}
                      type="button"
                      role="radio"
                      aria-checked={locale === code}
                      className={`choice-btn ${locale === code ? 'is-active' : ''}`}
                      onClick={() => setLocale(code)}
                    >
                      {t(`locale.${code}`)}
                    </button>
                  ))}
                </div>
              </div>
              <div className="grp">
                <h3>
                  {t('settings.theme.title')} <span>{t('settings.theme.sub')}</span>
                </h3>
                <div className="choice-row" role="radiogroup" aria-label={t('settings.theme.title')}>
                  {themes.map((item) => (
                    <button
                      key={item.key}
                      type="button"
                      role="radio"
                      aria-checked={theme === item.key}
                      className={`choice-btn ${theme === item.key ? 'is-active' : ''}`}
                      onClick={() => setTheme(item.key)}
                    >
                      {item.icon}
                      {item.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
        );

      case 'tools':
        return (
          <div className="wiz-step" key="tools">
            <h2>{t('onb.tools.title')}</h2>
            <p className="sub">
              {t('onb.tools.sub1')} <i>{t('onb.tools.sub2')}</i> {t('onb.tools.sub3')}
            </p>
            <div className="grp">
              <h3>
                {t('start.tools.title')}{' '}
                <span>
                  {missing.length === 0 && loggedOut.length === 0
                    ? t('onb.tools.allGood')
                    : t('onb.tools.todo', { missing: missing.length, loggedOut: loggedOut.length })}
                </span>
              </h3>
              <div className="rows">
                {tools.length === 0 && (
                  <div className="row">
                    <span className="ic">
                      <TerminalIcon />
                    </span>
                    <div className="nm">{setupRefreshing ? t('onb.tools.checking') : t('onb.tools.empty')}</div>
                  </div>
                )}
                {tools.map((tool) => {
                  const okText = !tool.found
                    ? t('onb.tools.notFound')
                    : tool.loggedIn === false
                      ? t('start.tools.loggedOut')
                      : tool.statusText || t('start.tools.loggedIn');
                  const cls = !tool.found ? 'off' : tool.loggedIn === false || tool.warn ? 'warn' : 'good';
                  const command = !tool.found ? INSTALL_COMMAND[tool.id] : tool.loggedIn === false ? LOGIN_COMMAND[tool.id] : undefined;
                  return (
                    <div key={tool.id} className="row">
                      <span className="ic">{toolIcon(tool.id)}</span>
                      <div className="nm">
                        {tool.id}
                        <span>{tool.account || tool.version || tool.path || ''}</span>
                      </div>
                      {command && <code className="cmd">{command}</code>}
                      <div className={`ok ${cls}`}>{okText}</div>
                      {command && (
                        <button type="button" className="act" onClick={() => void runInTerminal(`${tool.found ? 'login' : 'install'}:${tool.id}`)}>
                          {tool.found ? t('onb.tools.login') : t('onb.tools.install')}
                        </button>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
            <div className="choice-row">
              <button type="button" className="cta ghost" onClick={() => void runInTerminal()}>
                <TerminalIcon size={16} />
                <span>{t('start.foot.openTerminal')}</span>
              </button>
              <button
                type="button"
                className="cta ghost"
                disabled={setupRefreshing}
                onClick={() => void refreshSetup()}
              >
                <span>{setupRefreshing ? t('onb.tools.checking') : t('onb.tools.recheck')}</span>
              </button>
            </div>
          </div>
        );

      case 'projects':
        return (
          <div className="wiz-step" key="projects">
            <h2>{t('onb.projects.title')}</h2>
            <p className="sub">
              {t('onb.projects.sub1')} <i>{t('onb.projects.sub2')}</i> {t('onb.projects.sub3')}
            </p>
            <div className="grp">
              <h3>
                {t('side.projects')} <span>{t('onb.projects.count', { count: projects.length })}</span>
              </h3>
              <div className="rows">
                {projects.length === 0 && (
                  <div className="row">
                    <div className="nm">
                      {t('onb.projects.empty')}
                      <span>~/projects</span>
                    </div>
                  </div>
                )}
                {projects.map((project) => (
                  <div key={project.path} className="row">
                    <span className="ic">
                      <CheckIcon />
                    </span>
                    <div className="nm">
                      {project.name}
                      <span>{project.path}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            <p className="about-text">{t('onb.projects.note')}</p>
          </div>
        );

      case 'access':
        return (
          <div className="wiz-step" key="access">
            <h2>{t('start.access.title')}</h2>
            <p className="sub">
              {t('start.access.sub1')} <i>{t('start.access.sub2')}</i> {t('start.access.sub3')}
            </p>
            <div className="grp">
              <h3>
                {t('start.ssh.title')} <span>{t('start.ssh.count', { count: sshHosts.length })}</span>
              </h3>
              <div className="rows">
                {sshHosts.length === 0 && (
                  <div className="row">
                    <div className="nm">{t('onb.access.noSsh')}</div>
                  </div>
                )}
                {sshHosts.map((host) => (
                  <div key={host.host} className="row">
                    <span className="ic">
                      <ServerHostIcon />
                    </span>
                    <div className="nm">
                      {host.host}
                      <span>
                        {host.hostname || host.host}
                        {host.user ? ` · ${t('start.ssh.user', { user: host.user })}` : ''}
                      </span>
                    </div>
                    <div className={`ok ${host.warn ? 'warn' : 'good'}`}>{host.status || t('start.ssh.connecting')}</div>
                  </div>
                ))}
              </div>
            </div>
            <div className="grp">
              <h3>
                {t('onb.access.mcp')} <span>{t('onb.access.mcpCount', { count: mcpList.length })}</span>
              </h3>
              <div className="rows">
                {mcpList.length === 0 && (
                  <div className="row">
                    <div className="nm">{t('onb.access.noMcp')}</div>
                  </div>
                )}
                {mcpList.map((server) => (
                  <div key={`${server.source}-${server.name}`} className="row">
                    <div className="nm">
                      {server.name}
                      <span>{server.source}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        );

      case 'instructions':
        return (
          <div className="wiz-step" key="instructions">
            <h2>{t('onb.instr.title')}</h2>
            <p className="sub">
              {t('onb.instr.sub1')} <i>{t('onb.instr.sub2')}</i> {t('onb.instr.sub3')}
            </p>
            <div className="grp">
              <div className="rows">
                <div className="row">
                  <span className="ic">
                    <TerminalIcon />
                  </span>
                  <div className="nm">
                    ~/.orchestra/INSTRUCTIONS.md
                    <span>{instructionsReady ? t('onb.instr.existsHint') : t('onb.instr.missingHint')}</span>
                  </div>
                  <div className={`ok ${instructionsReady ? 'good' : 'warn'}`}>
                    {instructionsReady ? t('onb.instr.exists') : t('onb.instr.missing')}
                  </div>
                  {!instructionsReady && (
                    <button type="button" className="act" onClick={() => void createInstructions()}>
                      {t('onb.instr.create')}
                    </button>
                  )}
                </div>
                <div className="row">
                  <span className="ic">
                    <CheckIcon />
                  </span>
                  <div className="nm">
                    ~/.claude/CLAUDE.md
                    <span>{t('onb.instr.claudeMd')}</span>
                  </div>
                  <div className="ok good">{t('onb.instr.untouched')}</div>
                </div>
              </div>
            </div>
            <p className="about-text">{t('onb.instr.note')}</p>
          </div>
        );

      case 'done':
        return (
          <div className="wiz-step" key="done">
            <h2>{t('onb.done.title')}</h2>
            <p className="sub">
              {t('onb.done.sub1')} <i>{t('onb.done.sub2')}</i> {t('onb.done.sub3')}
            </p>
            <div className="grp">
              <div className="rows">
                <div className="row">
                  <span className="ic">{tools.length > 0 && loggedOut.length === 0 && missing.length === 0 ? <CheckIcon /> : <CircleTodoIcon />}</span>
                  <div className="nm">
                    {t('start.tools.title')}
                    <span>
                      {tools.length === 0
                        ? t('onb.tools.empty')
                        : t('onb.done.toolsHint', { total: tools.length, ready: tools.length - loggedOut.length - missing.length })}
                    </span>
                  </div>
                </div>
                <div className="row">
                  <span className="ic">{projects.length > 0 ? <CheckIcon /> : <CircleTodoIcon />}</span>
                  <div className="nm">
                    {t('side.projects')}
                    <span>{t('onb.projects.count', { count: projects.length })}</span>
                  </div>
                </div>
                <div className="row">
                  <span className="ic">{sshHosts.length > 0 ? <CheckIcon /> : <CircleTodoIcon />}</span>
                  <div className="nm">
                    {t('start.access.title')}
                    <span>
                      {t('start.ssh.count', { count: sshHosts.length })} · {t('onb.access.mcpCount', { count: mcpList.length })}
                    </span>
                  </div>
                </div>
                <div className="row">
                  <span className="ic">{instructionsReady ? <CheckIcon /> : <CircleTodoIcon />}</span>
                  <div className="nm">
                    {t('start.nav.instructions')}
                    <span>{instructionsReady ? t('onb.instr.exists') : t('onb.instr.missing')}</span>
                  </div>
                </div>
              </div>
            </div>
            <p className="about-text">{t('onb.done.note')}</p>
          </div>
        );
    }
  };

  return (
    <div className="win" id="onboarding">
      <aside className="side">
        <div className="proj">
          <h3>{t('onb.nav.title')}</h3>
          <ul className="setnav">
            {STEPS.map((id, i) => {
              const state = stepState(i);
              return (
                <li key={id} aria-current={state === 'now' ? 'true' : undefined} className={state === 'todo' ? 'off' : undefined}>
                  <button
                    type="button"
                    className="setnav-link"
                    aria-current={state === 'now' ? 'page' : undefined}
                    disabled={state === 'todo'}
                    onClick={() => setIndex(i)}
                  >
                    <span className={`k ${state === 'now' ? 'now' : state === 'todo' ? 'todo' : ''}`}>
                      {state === 'done' && <CheckIcon />}
                      {state === 'now' && <DotIcon />}
                      {state === 'todo' && <CircleTodoIcon />}
                    </span>
                    {t(`onb.step.${id}`)}
                  </button>
                </li>
              );
            })}
          </ul>
        </div>

        <div className="limits wiz-progress">
          <h3>{t('start.step', { step: index + 1, total })}</h3>
          <div className="lim">
            <span>{t('start.setup')}</span>
            <b>{Math.round(((index + 1) / total) * 100)}%</b>
            <span className="t">
              <i style={{ width: `${((index + 1) / total) * 100}%` }} />
            </span>
          </div>
        </div>
      </aside>

      <main className="main wiz">
        <section className="start">{renderStep()}</section>
        <div className="wiz-foot">
          <div className="n">{step === 'done' ? t('onb.foot.laterHint') : t('onb.foot.skipHint')}</div>
          <div className="acts">
            {index > 0 && (
              <button type="button" className="cta quiet" onClick={back}>
                <ArrowLeftIcon />
                <span>{t('onb.foot.back')}</span>
              </button>
            )}
            {step !== 'done' && (
              <button type="button" className="cta quiet" onClick={completeOnboarding}>
                {t('onb.foot.skip')}
              </button>
            )}
            {step === 'done' ? (
              <button type="button" className="cta" onClick={completeOnboarding}>
                {t('onb.foot.finish')}
                <span>
                  <ArrowRightIcon />
                </span>
              </button>
            ) : (
              <button type="button" className="cta" onClick={next}>
                {t('onb.foot.next')}
                <span>
                  <ArrowRightIcon />
                </span>
              </button>
            )}
          </div>
        </div>
      </main>
    </div>
  );
};
