import { useStore } from '../store';

export function UpdateBanner() {
  const update = useStore((state) => state.update);
  const updateProgress = useStore((state) => state.updateProgress);
  const updateError = useStore((state) => state.updateError);
  const installUpdate = useStore((state) => state.installUpdate);
  const dismissUpdate = useStore((state) => state.dismissUpdate);

  if (!update) return null;

  const isInstalling = updateProgress !== null;

  return (
    <section className="update-banner" aria-label="Доступно обновление">
      <div className="update-banner__content">
        <strong>Доступна версия {update.version}</strong>
        {isInstalling ? (
          <span className="update-banner__progress" role="status" aria-live="polite">
            Скачивание {Math.round(updateProgress)}%
          </span>
        ) : null}
        <div className="update-banner__actions">
          <button
            type="button"
            className="update-banner__install"
            disabled={isInstalling}
            onClick={installUpdate}
          >
            Обновить
          </button>
          <button
            type="button"
            className="update-banner__later"
            disabled={isInstalling}
            onClick={dismissUpdate}
          >
            Позже
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
