import React from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';
import { GearIcon } from './Icons';
import { LocaleToggle } from './LocaleToggle';
import { ThemeToggle } from './ThemeToggle';

export const Lead: React.FC = () => {
  const t = useT();
  const screen = useStore((state) => state.screen);
  const setScreen = useStore((state) => state.setScreen);
  const activeProject = useStore((state) => state.activeProject);
  const onboarding = screen === 'onboarding';

  return (
    <header className="lead">
      <div>
        <h1>
          {t('lead.title.before')}
          <em>{t('lead.title.em')}</em>
          {t('lead.title.after')}
        </h1>
        {activeProject && !onboarding && <p>{activeProject.name}</p>}
      </div>
      <div className="ctl">
        {!onboarding && (
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
        )}
        <LocaleToggle />
        <ThemeToggle />
        {!onboarding && (
          <button
            type="button"
            className="gear"
            aria-label={t('lead.settings')}
            title={t('lead.settings')}
            aria-current={screen === 'settings' ? 'page' : undefined}
            onClick={() => setScreen('settings')}
          >
            <GearIcon size={18} />
          </button>
        )}
      </div>
    </header>
  );
};
