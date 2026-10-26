import React, { useState } from 'react';
import { useT } from '../i18n';
import type { Job } from '../lib/ipc';
import { AgentLogo } from './BrandLogos';
import { jobStateLabel } from '../lib/labels';
import { sourcesForJob, useStore } from '../store';

interface SatelliteProps {
  job: Job;
}

export const Satellite: React.FC<SatelliteProps> = ({ job }) => {
  const t = useT();
  const setTab = useStore((state) => state.setTab);
  const setActiveWorktree = useStore((state) => state.setActiveWorktree);
  const sendJobAnswer = useStore((state) => state.sendJobAnswer);
  const previewSources = useStore((state) => state.previewSources);
  const openPreview = useStore((state) => state.openPreview);
  const jobSources = sourcesForJob(job, previewSources);

  const [customAnswer, setCustomAnswer] = useState('');

  const isCodex = job.agent === 'codex';
  const isAgy = job.agent === 'gemini';
  const satCls = isCodex ? 'sat codex' : isAgy ? 'sat agy' : 'sat claude';
  const monoText = isCodex ? 'Cx' : isAgy ? 'Ag' : 'Cl';
  const agentName = isCodex ? 'Codex' : isAgy ? 'Antigravity' : 'Claude';
  const modelText = job.model || (isCodex ? 'gpt-5-codex' : isAgy ? 'Gemini 2.5 Pro' : 'Claude');

  const isAsking = job.state === 'asking' || Boolean(job.question);
  const stateCls = isAsking ? 'state wait' : 'state';
  const stateText = isAsking ? t('sat.waiting') : jobStateLabel(job.state);

  const steps = job.steps || [];
  const firstIncompleteIdx = steps.findIndex((s) => !s.done);
  const nextStep = firstIncompleteIdx >= 0 ? steps[firstIncompleteIdx].text : t('sat.finish');

  const logLines = job.logLines || [{ time: '14:02', text: job.lastLine || t('sat.running') }];

  const laterKeyword = t('sat.laterKeyword').toLowerCase();

  const handleChipClick = (chip: string) => {
    sendJobAnswer(job.name, chip);
  };

  const handleCustomSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!customAnswer.trim()) return;
    sendJobAnswer(job.name, customAnswer.trim());
    setCustomAnswer('');
  };

  const handleOpenTerminal = () => {
    setActiveWorktree(job.worktree);
    setTab('code');
  };

  return (
    <article className={satCls}>
      <div className="hd">
        <div className="mono" title={agentName} aria-label={monoText}>
          <AgentLogo agent={job.agent} size={16} />
        </div>
        <div className="t">
          <b>{agentName}</b>
          <span>{t('sat.meta', { worktree: job.worktree, model: modelText })}</span>
        </div>
        <span className={stateCls}>
          <i />
          {stateText}
        </span>
      </div>

      <div className="log">
        {logLines.slice(-3).map((item, idx) => (
          <div key={idx}>
            <time>{item.time}</time>
            <span>{item.text}</span>
          </div>
        ))}
      </div>

      {isAsking && (
        <div className="ask">
          <p>{job.question || job.lastLine}</p>
          <div className="chips">
            {(job.chips || []).map((chip, idx) => {
              const isQ = chip.toLowerCase().includes(laterKeyword);
              return (
                <button
                  key={idx}
                  className={isQ ? 'q' : undefined}
                  onClick={() => handleChipClick(chip)}
                >
                  {chip}
                </button>
              );
            })}
          </div>
          <form className="sat-ask-input-wrap" onSubmit={handleCustomSubmit}>
            <input
              type="text"
              className="sat-ask-input"
              placeholder={t('sat.customPlaceholder')}
              value={customAnswer}
              onChange={(e) => setCustomAnswer(e.target.value)}
            />
            <button type="submit" className="sat-ask-send">
              {t('sat.answer')}
            </button>
          </form>
        </div>
      )}

      <div className="ft">
        <div className="step">
          {isAsking ? (
            <>
              {t('sat.pause', { step: firstIncompleteIdx >= 0 ? firstIncompleteIdx + 1 : 2 })} ·{' '}
              <b>{nextStep}</b>
            </>
          ) : (
            <>
              {t('sat.next')} <b>{nextStep}</b>
            </>
          )}
        </div>
        <button type="button" onClick={() => console.log('diff clicked')}>
          {t('sat.diff')}
        </button>
        {jobSources.length > 0 && (
          <button type="button" className="sat-result" onClick={() => openPreview(jobSources[0])}>
            {t('preview.title')}
          </button>
        )}
        <button type="button" onClick={handleOpenTerminal}>
          {t('sat.terminal')}
        </button>
      </div>
    </article>
  );
};
