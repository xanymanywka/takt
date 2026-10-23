import React from 'react';
import { useStore } from '../store';
import { ThemeToggle } from './ThemeToggle';

export const Lead: React.FC = () => {
  const screen = useStore((state) => state.screen);
  const setScreen = useStore((state) => state.setScreen);
  const activeProject = useStore((state) => state.activeProject);

  return (
    <header className="lead">
      <div>
        <h1>
          Такт. <em>Партитура</em>, а не дашборд.
        </h1>
        {activeProject && <p>{activeProject.name}</p>}
      </div>
      <div className="ctl">
        <div className="switch" role="tablist" aria-label="Экраны">
          <button
            role="tab"
            aria-pressed={screen === 'chat'}
            onClick={() => setScreen('chat')}
          >
            Чат с дирижёром
          </button>
          <button
            role="tab"
            aria-pressed={screen === 'start'}
            onClick={() => setScreen('start')}
          >
            Старт · Доступы
          </button>
        </div>
        <ThemeToggle />
      </div>
    </header>
  );
};
