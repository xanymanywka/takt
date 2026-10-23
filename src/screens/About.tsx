import React, { useEffect, useState } from 'react';
import { ArrowRightIcon, GhIcon } from '../components/Icons';
import { Sidebar } from '../components/Sidebar';
import { Support } from '../components/Support';
import { useT } from '../i18n';
import { isTauriEnv } from '../lib/ipc';
import { useStore } from '../store';

const LINKS = {
  site: 'https://xanyman.space',
  telegram: 'https://t.me/xanymanywka',
  github: 'https://github.com/xanymanywka/takt',
  license: 'https://github.com/xanymanywka/takt/blob/main/LICENSE',
} as const;

/** Внутри Tauri ссылки открывает системный браузер через plugin-opener, снаружи — window.open. */
async function openExternal(url: string): Promise<void> {
  if (isTauriEnv()) {
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      return;
    } catch (err) {
      console.error(err);
    }
  }
  window.open(url, '_blank', 'noopener,noreferrer');
}

/** Версия: из Tauri (tauri.conf.json), в браузере — из package.json через vite define. */
async function readVersion(): Promise<string> {
  if (isTauriEnv()) {
    try {
      const { getVersion } = await import('@tauri-apps/api/app');
      return await getVersion();
    } catch (err) {
      console.error(err);
    }
  }
  return __APP_VERSION__;
}

export const About: React.FC = () => {
  const t = useT();
  const updateCheckStatus = useStore((state) => state.updateCheckStatus);
  const updateCheckMessage = useStore((state) => state.updateCheckMessage);
  const checkUpdate = useStore((state) => state.checkUpdate);
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void readVersion().then((v) => {
      if (!cancelled) setVersion(v);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const link = (key: keyof typeof LINKS, label: string, icon?: React.ReactNode) => (
    <button
      type="button"
      className="row about-link"
      onClick={() => void openExternal(LINKS[key])}
    >
      {icon && <span className="ic">{icon}</span>}
      <div className="nm">
        {label}
        <span>{LINKS[key].replace(/^https:\/\//, '')}</span>
      </div>
      <span className="about-arrow" aria-hidden="true">
        <ArrowRightIcon />
      </span>
    </button>
  );

  return (
    <div className="win" id="about">
      <Sidebar />
      <main className="main">
        <section className="start about">
          <h2>{t('about.title')}</h2>
          <p className="sub">{t('about.sub')}</p>

          <div className="about-card">
            <div className="about-mark" aria-hidden="true">
              T
            </div>
            <div className="about-id">
              <b>Takt</b>
              <span>
                {version ? t('about.version', { version }) : t('about.versionUnknown')}
              </span>
              <span>
                {t('about.author')}: {t('about.authorName')}
              </span>
            </div>
            <button
              type="button"
              className="update-settings__check"
              disabled={updateCheckStatus === 'checking'}
              onClick={() => void checkUpdate(true)}
            >
              {updateCheckStatus === 'checking' ? t('about.checking') : t('about.checkUpdates')}
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

          <p className="about-text">{t('about.description')}</p>

          <div className="grp">
            <h3>{t('about.links')}</h3>
            <div className="rows">
              {link('site', t('about.site'))}
              {link('telegram', t('about.telegram'))}
              {link('github', t('about.github'), <GhIcon />)}
            </div>
          </div>

          <div className="grp">
            <h3>
              {t('about.license')} <span>{t('about.licenseHint')}</span>
            </h3>
            <div className="rows">{link('license', t('about.licenseName'))}</div>
          </div>

          <Support />
        </section>
      </main>
    </div>
  );
};
