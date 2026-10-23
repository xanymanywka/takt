import React, { useState } from 'react';
import type { Job } from '../lib/ipc';
import { useStore } from '../store';

interface SatelliteProps {
  job: Job;
}

export const Satellite: React.FC<SatelliteProps> = ({ job }) => {
  const setTab = useStore((state) => state.setTab);
  const setActiveWorktree = useStore((state) => state.setActiveWorktree);
  const sendJobAnswer = useStore((state) => state.sendJobAnswer);

  const [customAnswer, setCustomAnswer] = useState('');

  const isCodex = job.agent === 'codex';
  const isAgy = job.agent === 'gemini';
  const satCls = isCodex ? 'sat codex' : isAgy ? 'sat agy' : 'sat claude';
  const monoText = isCodex ? 'Cx' : isAgy ? 'Ag' : 'Cl';
  const agentName = isCodex ? 'Codex' : isAgy ? 'Antigravity' : 'Claude';
  const modelText = job.model || (isCodex ? 'gpt-5-codex' : 'Gemini 2.5 Pro');

  const isAsking = job.state === 'asking' || Boolean(job.question);
  const stateCls = isAsking ? 'state wait' : 'state';
  const stateText = isAsking ? 'ждёт вас' : job.state === 'done' ? 'готово' : 'пишет';

  const steps = job.steps || [];
  const firstIncompleteIdx = steps.findIndex((s) => !s.done);
  const nextStep = firstIncompleteIdx >= 0 ? steps[firstIncompleteIdx].text : 'завершение';

  const logLines = job.logLines || [
    { time: '14:02', text: job.lastLine || 'Выполняется задача' },
  ];

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
        <div className="mono">{monoText}</div>
        <div className="t">
          <b>{agentName}</b>
          <span>ворктри {job.worktree} · модель {modelText}</span>
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
              const isQ = chip.toLowerCase().includes('позже');
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
              placeholder="Свой ответ..."
              value={customAnswer}
              onChange={(e) => setCustomAnswer(e.target.value)}
            />
            <button type="submit" className="sat-ask-send">
              Ответить
            </button>
          </form>
        </div>
      )}

      <div className="ft">
        <div className="step">
          {isAsking ? (
            <>
              Пауза на шаге {firstIncompleteIdx >= 0 ? firstIncompleteIdx + 1 : 2} ·{' '}
              <b>{nextStep}</b>
            </>
          ) : (
            <>
              Дальше: <b>{nextStep}</b>
            </>
          )}
        </div>
        <button type="button" onClick={() => console.log('diff clicked')}>
          Дифф
        </button>
        <button type="button" onClick={handleOpenTerminal}>
          Терминал
        </button>
      </div>
    </article>
  );
};
