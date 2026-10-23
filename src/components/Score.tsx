import React from 'react';
import { useStore } from '../store';
import type { Job } from '../lib/ipc';

export const Score: React.FC = () => {
  const jobs = useStore((state) => state.jobs);

  const totalSteps = 5;

  const getAgentColorVar = (agent: Job['agent']) => {
    switch (agent) {
      case 'codex':
        return 'var(--codex)';
      case 'gemini':
        return 'var(--agy)';
      case 'claude':
        return 'var(--acc)';
      default:
        return 'var(--acc)';
    }
  };

  const getAgentName = (agent: Job['agent']) => {
    switch (agent) {
      case 'codex':
        return 'Codex';
      case 'gemini':
        return 'Antigravity';
      case 'claude':
        return 'Claude';
      default:
        return agent;
    }
  };

  return (
    <div className="score" aria-label="Партитура задачи">
      <header>
        <h4>Партитура</h4>
        <span>5 шагов · начали 4 минуты назад</span>
      </header>

      {/* Строка дирижёра Claude */}
      <div className="stave me-row" style={{ '--c': 'var(--acc)' } as React.CSSProperties}>
        <div className="nm">
          <i />
          Claude
        </div>
        <div className="bars">
          <i className="done" />
          <i className="done" />
          <i />
          <i />
          <i />
        </div>
        <div className="st">
          <b>план готов</b>, жду исполнителей
        </div>
      </div>

      {/* Строки исполнителей */}
      {jobs.map((job) => {
        const colorVar = getAgentColorVar(job.agent);
        const agentName = getAgentName(job.agent);
        const steps = job.steps || [];
        const doneCount = steps.filter((s) => s.done).length;
        const currentIdx = steps.findIndex((s) => !s.done);
        const stepNum = currentIdx >= 0 ? currentIdx + 1 : doneCount;

        let statusText: React.ReactNode = (
          <>
            шаг {stepNum} из {steps.length || totalSteps} · <b>{job.lastLine}</b>
          </>
        );

        if (job.state === 'asking') {
          statusText = (
            <>
              шаг {stepNum} из {steps.length || totalSteps} · <b>ждёт ответа</b>
            </>
          );
        }

        return (
          <div
            key={job.name}
            className="stave"
            style={{ '--c': colorVar } as React.CSSProperties}
          >
            <div className="nm">
              <i />
              {agentName}
            </div>
            <div className="bars">
              {steps.map((st, idx) => {
                let cls = '';
                if (st.done) {
                  cls = 'done';
                } else if (idx === currentIdx) {
                  if (job.state === 'asking') {
                    cls = 'ask';
                  } else if (job.state === 'running') {
                    cls = 'now';
                  }
                }
                return <i key={idx} className={cls} />;
              })}
            </div>
            <div className="st">{statusText}</div>
          </div>
        );
      })}

      <div className="legend">
        <span>
          <i />
          сделано
        </span>
        <span className="l2">
          <i />
          в работе
        </span>
        <span className="l3">
          <i />
          вопрос к вам
        </span>
        <span className="l4">
          <i />
          впереди
        </span>
      </div>
    </div>
  );
};
