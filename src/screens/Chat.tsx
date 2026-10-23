import React from 'react';
import { Composer } from '../components/Composer';
import { Score } from '../components/Score';
import { Satellite } from '../components/Satellite';
import { useStore } from '../store';

export const Chat: React.FC = () => {
  const tab = useStore((state) => state.tab);
  const setTab = useStore((state) => state.setTab);
  const activeProject = useStore((state) => state.activeProject);
  const messages = useStore((state) => state.messages);
  const jobs = useStore((state) => state.jobs);

  const projectName = activeProject?.name || 'Такт';
  const workerCount = jobs.length || 2;

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
          <b>{projectName}</b> · ветка design-v3{' '}
          <span className="pill">{workerCount} исполнителя</span>
        </div>
      </div>

      <section className="chat">
        {isEmpty ? (
          <div className="empty-state">
            <h3>Дирижёр ждёт первой задачи</h3>
            <p>
              Опишите, что нужно сделать. Дирижёр создаст план, распределит задачи между исполнителями
              и покажет партитуру.
            </p>
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
                    <div className="mono">К</div>
                    <div>
                      <div className="who">
                        <b>Claude</b> дирижирует
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
