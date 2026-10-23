import React from 'react';
import { Composer } from '../components/Composer';
import { Score } from '../components/Score';
import { Satellite } from '../components/Satellite';
import { Tabs } from '../components/Tabs';
import { useT } from '../i18n';
import { useStore } from '../store';

export const Chat: React.FC = () => {
  const t = useT();
  const activeProject = useStore((state) => state.activeProject);
  const messages = useStore((state) => state.messages);
  const jobs = useStore((state) => state.jobs);

  const projectName = activeProject?.name || 'Takt';
  const workerCount = jobs.length;

  // Если в проекте нет сообщений и нет задач
  const isEmpty = messages.length === 0 && jobs.length === 0;

  // Индекс последнего ответа дирижёра
  let lastAssistantIndex = -1;
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i].role === 'assistant') {
      lastAssistantIndex = i;
      break;
    }
  }

  return (
    <main className="main">
      <div className="top">
        <Tabs />
        <div className="where">
          <b>{projectName}</b> · {t('chat.branch', { branch: 'design-v3' })}{' '}
          <span className="pill">{t('chat.workers', { count: workerCount })}</span>
        </div>
      </div>

      <section className="chat">
        {isEmpty ? (
          <div className="empty-state">
            <h3>{t('chat.empty.title')}</h3>
            <p>{t('chat.empty.text')}</p>
          </div>
        ) : (
          <>
            {messages.map((msg, index) => {
              if (msg.role === 'user') {
                return (
                  <div key={msg.id} className="me">
                    {msg.text}
                    {msg.time && <small>{msg.time}</small>}
                  </div>
                );
              }

              if (msg.role === 'tool') {
                return (
                  <div key={msg.id} className="tool-line">
                    <span />
                    {msg.text}
                  </div>
                );
              }

              const isLastAssistant = msg.role === 'assistant' && index === lastAssistantIndex;

              return (
                <React.Fragment key={msg.id}>
                  <div className="voice">
                    <div className="mono">{t('agent.claudeInitial')}</div>
                    <div>
                      <div className="who">
                        <b>Claude</b> {t('chat.conducts')}
                      </div>
                      <p>{msg.text}</p>
                      {isLastAssistant && <Score />}
                    </div>
                  </div>

                  {isLastAssistant && jobs.length > 0 && (
                    <div className="sats">
                      {jobs.map((job) => (
                        <Satellite key={job.name} job={job} />
                      ))}
                    </div>
                  )}
                </React.Fragment>
              );
            })}
          </>
        )}
      </section>

      <Composer />
    </main>
  );
};
