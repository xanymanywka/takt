import React from 'react';
import { Tabs } from '../components/Tabs';
import { Terminal } from '../components/Terminal';
import { useT } from '../i18n';
import { useStore } from '../store';

export const Code: React.FC = () => {
  const t = useT();
  const activeProject = useStore((state) => state.activeProject);
  const activeWorktree = useStore((state) => state.activeWorktree);
  const setActiveWorktree = useStore((state) => state.setActiveWorktree);
  const jobs = useStore((state) => state.jobs);

  const projectName = activeProject?.name || 'Takt';

  // Собираем список ворктри
  const worktrees = Array.from(new Set([...jobs.map((j) => j.worktree), 'main']));

  return (
    <main className="main">
      <div className="top">
        <Tabs />
        <div className="where">
          <b>{projectName}</b> · {t('code.worktree')} <b>{activeWorktree}</b>
        </div>
      </div>

      <div className="code-view">
        <aside className="wt-list">
          <h4>{t('code.worktrees')}</h4>
          {worktrees.map((wt) => {
            const matchingJob = jobs.find((j) => j.worktree === wt);
            const isCurrent = wt === activeWorktree;
            const dotColor =
              matchingJob?.agent === 'codex'
                ? 'var(--codex)'
                : matchingJob?.agent === 'gemini'
                ? 'var(--agy)'
                : 'var(--acc)';

            return (
              <button
                key={wt}
                type="button"
                className="wt-item"
                aria-current={isCurrent ? 'true' : undefined}
                onClick={() => setActiveWorktree(wt)}
              >
                <span className="wt-dot" style={{ background: dotColor }} />
                <span>{wt}</span>
              </button>
            );
          })}
        </aside>

        <section className="term-container">
          <div className="term-header">
            <span>
              {t('code.terminal')}: <b>{activeWorktree}</b>
            </span>
            <span>{t('code.pty')}</span>
          </div>
          <Terminal worktree={activeWorktree} />
        </section>
      </div>
    </main>
  );
};
