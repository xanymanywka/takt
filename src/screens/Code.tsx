import React from 'react';
import { Terminal } from '../components/Terminal';
import { useStore } from '../store';

export const Code: React.FC = () => {
  const tab = useStore((state) => state.tab);
  const setTab = useStore((state) => state.setTab);
  const activeProject = useStore((state) => state.activeProject);
  const activeWorktree = useStore((state) => state.activeWorktree);
  const setActiveWorktree = useStore((state) => state.setActiveWorktree);
  const jobs = useStore((state) => state.jobs);

  const projectName = activeProject?.name || 'Takt';

  // Собираем список ворктри
  const worktrees = Array.from(
    new Set([
      ...jobs.map((j) => j.worktree),
      'main',
    ])
  );

  return (
    <main className="main">
      <div className="top">
        <div className="seg" role="tablist" aria-label="Вкладки">
          <button
            role="tab"
            aria-pressed={tab === 'chat'}
            onClick={() => setTab('chat')}
          >
            Чат
          </button>
          <button
            role="tab"
            aria-pressed={tab === 'code'}
            onClick={() => setTab('code')}
          >
            Код
          </button>
          <button
            role="tab"
            aria-pressed={tab === 'orchestra'}
            onClick={() => setTab('orchestra')}
          >
            Оркестр
          </button>
        </div>
        <div className="where">
          <b>{projectName}</b> · ворктри <b>{activeWorktree}</b>
        </div>
      </div>

      <div className="code-view">
        <aside className="wt-list">
          <h4>Ворктри</h4>
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
              <div
                key={wt}
                className="wt-item"
                aria-current={isCurrent ? 'true' : undefined}
                onClick={() => setActiveWorktree(wt)}
              >
                <span className="wt-dot" style={{ background: dotColor }} />
                <span>{wt}</span>
              </div>
            );
          })}
        </aside>

        <section className="term-container">
          <div className="term-header">
            <span>
              Терминал: <b>{activeWorktree}</b>
            </span>
            <span>PTY-сессия</span>
          </div>
          <Terminal worktree={activeWorktree} />
        </section>
      </div>
    </main>
  );
};
