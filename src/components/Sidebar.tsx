import React from 'react';
import { useStore } from '../store';

export const Sidebar: React.FC = () => {
  const projects = useStore((state) => state.projects);
  const activeProject = useStore((state) => state.activeProject);
  const setActiveProject = useStore((state) => state.setActiveProject);
  const jobs = useStore((state) => state.jobs);
  const limits = useStore((state) => state.limits);

  // Проверяем, есть ли запущенные задачи
  const hasRunningJobs = jobs.some((j) => j.state === 'running' || j.state === 'asking');

  const getLimitVal = (provider: 'claude' | 'codex' | 'antigravity'): number => {
    const lim = limits.find((l) => l.provider === provider);
    return lim?.sessionLeft ?? (provider === 'claude' ? 62 : provider === 'codex' ? 41 : 88);
  };

  const claudeVal = getLimitVal('claude');
  const codexVal = getLimitVal('codex');
  const agyVal = getLimitVal('antigravity');

  return (
    <aside className="side">
      <div className="lights">
        <i />
        <i />
        <i />
      </div>

      <div className="proj">
        <h3>Проекты</h3>
        <ul>
          {projects.map((proj) => {
            const isCurrent = activeProject?.name === proj.name;
            const isLive = isCurrent && hasRunningJobs;
            return (
              <li
                key={proj.name}
                aria-current={isCurrent ? 'true' : undefined}
                onClick={() => setActiveProject(proj)}
              >
                <span
                  className="dot"
                  style={{ background: proj.color || '#9b86d6' }}
                />
                <span className="n">{proj.name}</span>
                {isLive && <span className="live" title="идёт задача" />}
              </li>
            );
          })}
        </ul>
      </div>

      <div className="limits">
        <h3>Осталось на сегодня</h3>
        <div className="lim">
          <span>Claude</span>
          <b>{claudeVal}%</b>
          <span className="t">
            <i style={{ width: `${claudeVal}%` }} />
          </span>
        </div>
        <div className="lim codex">
          <span>Codex</span>
          <b>{codexVal}%</b>
          <span className="t">
            <i style={{ width: `${codexVal}%` }} />
          </span>
        </div>
        <div className="lim agy">
          <span>Antigravity</span>
          <b>{agyVal}%</b>
          <span className="t">
            <i style={{ width: `${agyVal}%` }} />
          </span>
        </div>
      </div>
    </aside>
  );
};
