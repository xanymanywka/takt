import React from 'react';
import { useT } from '../i18n';
import type { Job } from '../lib/ipc';
import { conductorName, sourcesForJob, useStore } from '../store';
import { PanelIcon } from './Icons';
import { AgentLogo } from './BrandLogos';
import type { ConductorAgent } from '../lib/ipc';

const CONDUCTOR_COLOR: Record<ConductorAgent, string> = {
  claude: 'var(--acc)',
  codex: 'var(--codex)',
  agy: 'var(--agy)',
};

export const Score: React.FC = () => {
  const t = useT();
  const jobs = useStore((state) => state.jobs);
  const conductorAgent = useStore((state) => state.conductorAgent);
  const previewSources = useStore((state) => state.previewSources);
  const openPreview = useStore((state) => state.openPreview);

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
    <div className="score" aria-label={t('score.aria')}>
      <header>
        <h4>{t('score.title')}</h4>
        <span>{t('score.meta', { steps: totalSteps, minutes: 4 })}</span>
      </header>

      {/* Строка дирижёра — выбранный агент */}
      <div className="stave me-row" style={{ '--c': CONDUCTOR_COLOR[conductorAgent] } as React.CSSProperties}>
        <div className="nm">
          <span className="lg">
            <AgentLogo agent={conductorAgent} size={13} />
          </span>
          {conductorName(conductorAgent)}
        </div>
        <div className="bars">
          <i className="done" />
          <i className="done" />
          <i />
          <i />
          <i />
        </div>
        <div className="st">
          <b>{t('score.planReady')}</b>, {t('score.waitingWorkers')}
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
        const stepOf = t('score.stepOf', { step: stepNum, total: steps.length || totalSteps });

        const jobSources = sourcesForJob(job, previewSources);

        const statusText: React.ReactNode = (
          <>
            {stepOf} · <b>{job.state === 'asking' ? t('score.awaitingAnswer') : job.lastLine}</b>
          </>
        );

        return (
          <div
            key={job.name}
            className="stave"
            style={{ '--c': colorVar } as React.CSSProperties}
          >
            <div className="nm">
              <span className="lg">
                <AgentLogo agent={job.agent} size={13} />
              </span>
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
            {jobSources.length > 0 && (
              <button
                type="button"
                className="stave-result"
                onClick={() => openPreview(jobSources[0])}
                title={t('preview.openFor', { name: job.name })}
                aria-label={t('preview.openFor', { name: job.name })}
              >
                <PanelIcon size={13} />
              </button>
            )}
          </div>
        );
      })}

      <div className="legend">
        <span>
          <i />
          {t('score.legend.done')}
        </span>
        <span className="l2">
          <i />
          {t('score.legend.now')}
        </span>
        <span className="l3">
          <i />
          {t('score.legend.ask')}
        </span>
        <span className="l4">
          <i />
          {t('score.legend.ahead')}
        </span>
      </div>
    </div>
  );
};
