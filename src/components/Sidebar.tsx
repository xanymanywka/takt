import React from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';
import { AgentLogo } from './BrandLogos';
import { Support } from './Support';

type Provider = 'claude' | 'codex' | 'antigravity';

const PROVIDERS: Array<{ key: Provider; label: string; className: string }> = [
  { key: 'claude', label: 'Claude', className: 'lim' },
  { key: 'codex', label: 'Codex', className: 'lim codex' },
  { key: 'antigravity', label: 'Antigravity', className: 'lim agy' },
];

export const Sidebar: React.FC = () => {
  const t = useT();
  const projects = useStore((state) => state.projects);
  const activeProject = useStore((state) => state.activeProject);
  const setActiveProject = useStore((state) => state.setActiveProject);
  const jobs = useStore((state) => state.jobs);
  const limits = useStore((state) => state.limits);
  const screen = useStore((state) => state.screen);
  const setScreen = useStore((state) => state.setScreen);

  const hasRunningJobs = jobs.some((j) => j.state === 'running' || j.state === 'asking');

  // Нет данных о лимите — показываем «—», а не выдуманный процент.
  const limitFor = (provider: Provider): number | null =>
    limits.find((l) => l.provider === provider)?.sessionLeft ?? null;

  return (
    <aside className="side">
      <div className="lights">
        <i />
        <i />
        <i />
      </div>

      <div className="proj">
        <h3>{t('side.projects')}</h3>
        <ul>
          {projects.map((proj) => {
            const isCurrent = activeProject?.name === proj.name;
            const isLive = isCurrent && hasRunningJobs;
            return (
              <li key={proj.name} aria-current={isCurrent ? 'true' : undefined}>
                <button
                  type="button"
                  className="proj-btn"
                  aria-current={isCurrent ? 'true' : undefined}
                  onClick={() => setActiveProject(proj)}
                >
                  <span
                    className="dot"
                    style={{ background: proj.color || '#9b86d6' }}
                  />
                  <span className="n">{proj.name}</span>
                  {isLive && <span className="live" title={t('side.live')} />}
                </button>
              </li>
            );
          })}
        </ul>
      </div>

      <div className="limits">
        <h3>{t('side.limits')}</h3>
        {PROVIDERS.map(({ key, label, className }) => {
          const value = limitFor(key);
          return (
            <div className={className} key={key}>
              <span className="lim-name">
                <AgentLogo agent={key} size={12} />
                {label}
              </span>
              <b>{value === null ? '—' : `${value}%`}</b>
              <span className="t">
                <i style={{ width: `${value ?? 0}%` }} />
              </span>
            </div>
          );
        })}
      </div>

      <div className="side-foot">
        <Support compact onOpen={() => setScreen('about')} />
        <button
          type="button"
          className="side-about"
          aria-current={screen === 'about' ? 'page' : undefined}
          onClick={() => setScreen('about')}
        >
          {t('side.about')}
        </button>
      </div>
    </aside>
  );
};
