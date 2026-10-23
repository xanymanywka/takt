import React from 'react';
import { useStore } from '../store';
import type { JobState } from '../lib/ipc';
import { capitalize, jobStateLabel } from '../lib/labels';

export const Orchestra: React.FC = () => {
  const tab = useStore((state) => state.tab);
  const setTab = useStore((state) => state.setTab);
  const activeProject = useStore((state) => state.activeProject);
  const jobs = useStore((state) => state.jobs);
  const killJob = useStore((state) => state.killJob);

  const projectName = activeProject?.name || 'Takt';

  const getAgentLabel = (agent: string) => {
    if (agent === 'codex') return 'Codex';
    if (agent === 'gemini') return 'Antigravity';
    if (agent === 'claude') return 'Claude';
    return agent;
  };

  const getStateLabel = (state: JobState) => capitalize(jobStateLabel(state));

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
          <b>{projectName}</b> · реестр задач
        </div>
      </div>

      <div className="orch-view">
        <div className="orch-header">
          <h2>Реестр задач</h2>
          <p>Все активные и завершённые задачи исполнителей</p>
        </div>

        <table className="orch-table">
          <thead>
            <tr>
              <th>Проект / Задача</th>
              <th>Исполнитель</th>
              <th>Состояние</th>
              <th>Шаги</th>
              <th>Начато</th>
              <th style={{ textAlign: 'right' }}>Действия</th>
            </tr>
          </thead>
          <tbody>
            {jobs.map((job) => {
              const steps = job.steps || [];
              const doneCount = steps.filter((s) => s.done).length;
              const isAsking = job.state === 'asking';

              return (
                <tr key={job.name}>
                  <td>
                    <b>{projectName}</b>
                    <span style={{ display: 'block', color: 'var(--muted)', fontSize: '13px' }}>
                      {job.name} ({job.worktree})
                    </span>
                  </td>
                  <td>
                    <span
                      style={{
                        color:
                          job.agent === 'codex'
                            ? 'var(--codex)'
                            : job.agent === 'gemini'
                            ? 'var(--agy)'
                            : 'var(--acc)',
                        fontWeight: 600,
                      }}
                    >
                      {getAgentLabel(job.agent)}
                    </span>
                  </td>
                  <td>
                    <span
                      style={{
                        color:
                          job.state === 'asking'
                            ? 'var(--warn)'
                            : job.state === 'done'
                            ? 'var(--ok)'
                            : 'var(--fg)',
                        fontWeight: 500,
                      }}
                    >
                      {getStateLabel(job.state)}
                    </span>
                  </td>
                  <td>
                    {doneCount} из {steps.length || 5}
                  </td>
                  <td>{job.startedAt}</td>
                  <td style={{ textAlign: 'right' }}>
                    <div style={{ display: 'inline-flex', gap: '8px' }}>
                      {isAsking && (
                        <button
                          type="button"
                          className="orch-btn ans"
                          onClick={() => setTab('chat')}
                        >
                          Ответить
                        </button>
                      )}
                      {job.state !== 'killed' && job.state !== 'done' && (
                        <button
                          type="button"
                          className="orch-btn kill"
                          onClick={() => killJob(job.name)}
                        >
                          Остановить
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </main>
  );
};
