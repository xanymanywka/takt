import { useT } from '../i18n';
import { useStore } from '../store';

export function UpdateBanner() {
  const t = useT();
  const update = useStore((state) => state.update);
  const updateProgress = useStore((state) => state.updateProgress);
  const updateError = useStore((state) => state.updateError);
  const installUpdate = useStore((state) => state.installUpdate);
  const dismissUpdate = useStore((state) => state.dismissUpdate);

  if (!update) return null;

  const isInstalling = updateProgress !== null;

  return (
    <section className="update-banner" aria-label={t('update.aria')}>
      <div className="update-banner__content">
        <strong>{t('update.available', { version: update.version })}</strong>
        {isInstalling ? (
          <span className="update-banner__progress" role="status" aria-live="polite">
            {t('update.downloading', { percent: Math.round(updateProgress) })}
          </span>
        ) : null}
        <div className="update-banner__actions">
          <button
            type="button"
            className="update-banner__install"
            disabled={isInstalling}
            onClick={installUpdate}
          >
            {t('update.install')}
          </button>
          <button
            type="button"
            className="update-banner__later"
            disabled={isInstalling}
            onClick={dismissUpdate}
          >
            {t('update.later')}
          </button>
        </div>
      </div>
      {updateError ? (
        <p className="update-banner__error" role="alert">
          {updateError}
        </p>
      ) : null}
    </section>
  );
}
