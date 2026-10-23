import React from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';
import { LocaleToggle } from './LocaleToggle';
import { ThemeToggle } from './ThemeToggle';

export const Lead: React.FC = () => {
  const t = useT();
  const screen = useStore((state) => state.screen);
  const setScreen = useStore((state) => state.setScreen);
  const activeProject = useStore((state) => state.activeProject);

  return (
    <header className="lead">
      <div>
        <h1>
          {t('lead.title.before')}
          <em>{t('lead.title.em')}</em>
          {t('lead.title.after')}
        </h1>
        {activeProject && <p>{activeProject.name}</p>}
      </div>
      <div className="ctl">
        <div className="switch" role="tablist" aria-label={t('lead.screens')}>
          <button
            role="tab"
            aria-pressed={screen === 'chat'}
            onClick={() => setScreen('chat')}
          >
            {t('lead.chat')}
          </button>
          <button
            role="tab"
            aria-pressed={screen === 'start'}
            onClick={() => setScreen('start')}
          >
            {t('lead.start')}
          </button>
        </div>
        <LocaleToggle />
        <ThemeToggle />
      </div>
    </header>
  );
};
