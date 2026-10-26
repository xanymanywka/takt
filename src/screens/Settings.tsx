import React from 'react';
import { ArrowRightIcon, MoonIcon, SunIcon } from '../components/Icons';
import { ConductorChoice } from '../components/ConductorChoice';
import { SetupSide } from '../components/SetupSide';
import { LOCALES, useLocale, useT, type Locale } from '../i18n';
import { useStore, type ScreenType, type ThemeType } from '../store';

export const Settings: React.FC = () => {
  const t = useT();
  const locale = useLocale();
  const theme = useStore((state) => state.theme);
  const setTheme = useStore((state) => state.setTheme);
  const setLocale = useStore((state) => state.setLocale);
  const setScreen = useStore((state) => state.setScreen);
  const restartOnboarding = useStore((state) => state.restartOnboarding);
  const updateChecksEnabled = useStore((state) => state.updateChecksEnabled);
  const updateCheckStatus = useStore((state) => state.updateCheckStatus);
  const updateCheckMessage = useStore((state) => state.updateCheckMessage);
  const checkUpdate = useStore((state) => state.checkUpdate);
  const setUpdateChecks = useStore((state) => state.setUpdateChecks);
  const setup = useStore((state) => state.setup);

  const themes: Array<{ key: ThemeType; label: string; icon: React.ReactNode }> = [
    { key: 'light', label: t('settings.theme.light'), icon: <SunIcon size={14} /> },
    { key: 'dark', label: t('settings.theme.dark'), icon: <MoonIcon size={14} /> },
  ];

  const link = (label: string, hint: string, screen: ScreenType) => (
    <button type="button" className="row link" onClick={() => setScreen(screen)}>
      <div className="nm">
        {label}
        <span>{hint}</span>
      </div>
      <span className="about-arrow" aria-hidden="true">
        <ArrowRightIcon />
      </span>
    </button>
  );

  return (
    <div className="win" id="settings">
      <SetupSide current="settings" />
      <main className="main">
        <section className="start settings">
          <h2>{t('settings.title')}</h2>
          <p className="sub">{t('settings.sub')}</p>

          <div className="settings-grid">
            <div className="grp">
              <h3>
                {t('settings.language.title')} <span>{t('settings.language.sub')}</span>
              </h3>
              <div className="choice-row" role="radiogroup" aria-label={t('settings.language.title')}>
                {LOCALES.map((code: Locale) => (
                  <button
                    key={code}
                    type="button"
                    role="radio"
                    aria-checked={locale === code}
                    className={`choice-btn ${locale === code ? 'is-active' : ''}`}
                    onClick={() => setLocale(code)}
                  >
                    {t(`locale.${code}`)}
                  </button>
                ))}
              </div>
            </div>

            <div className="grp">
              <h3>
                {t('settings.theme.title')} <span>{t('settings.theme.sub')}</span>
              </h3>
              <div className="choice-row" role="radiogroup" aria-label={t('settings.theme.title')}>
                {themes.map((item) => (
                  <button
                    key={item.key}
                    type="button"
                    role="radio"
                    aria-checked={theme === item.key}
                    className={`choice-btn ${theme === item.key ? 'is-active' : ''}`}
                    onClick={() => setTheme(item.key)}
                  >
                    {item.icon}
                    {item.label}
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="grp update-settings">
            <h3>
              {t('start.updates.title')} <span>{t('start.updates.channel')}</span>
            </h3>
            <div className="update-settings__row">
              <label className="update-settings__toggle">
                <input
                  type="checkbox"
                  checked={updateChecksEnabled}
                  onChange={(event) => setUpdateChecks(event.target.checked)}
                />
                <span className="update-settings__track" aria-hidden="true">
                  <span />
                </span>
                <span>{t('start.updates.onLaunch')}</span>
              </label>
              <button
                type="button"
                className="update-settings__check"
                disabled={updateCheckStatus === 'checking'}
                onClick={() => void checkUpdate(true)}
              >
                {updateCheckStatus === 'checking' ? t('start.updates.checking') : t('start.updates.checkNow')}
              </button>
            </div>
            {updateCheckMessage ? (
              <p
                className={`update-settings__status ${updateCheckStatus === 'error' ? 'is-error' : ''}`}
                role="status"
                aria-live="polite"
              >
                {updateCheckMessage}
              </p>
            ) : null}
          </div>

          <div className="grp">
            <h3>{t('settings.agents.title')}</h3>
            <div className="conductor-pick">
              <div className="conductor-pick__head">
                {t('conductor.title')}
                <span>{t('conductor.appliesNext')}</span>
              </div>
              <ConductorChoice compact />
            </div>
            <div className="rows">
              {link(
                t('start.nav.access'),
                t('settings.agents.accessHint', {
                  ssh: setup?.ssh?.length ?? 0,
                  tools: setup?.tools?.length ?? 0,
                }),
                'start'
              )}
              {link(t('start.nav.models'), t('settings.agents.modelsHint'), 'models')}
            </div>
          </div>

          <div className="grp">
            <h3>{t('settings.app.title')}</h3>
            <div className="rows">
              {link(t('start.nav.about'), t('settings.app.aboutHint'), 'about')}
              <button type="button" className="row link danger" onClick={restartOnboarding}>
                <div className="nm">
                  {t('settings.app.rerun')}
                  <span>{t('settings.app.rerunHint')}</span>
                </div>
                <span className="about-arrow" aria-hidden="true">
                  <ArrowRightIcon />
                </span>
              </button>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
};
