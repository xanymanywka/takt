import React, { useCallback, useEffect, useState } from 'react';
import { ArrowRightIcon, CheckIcon } from '../components/Icons';
import { SetupSide } from '../components/SetupSide';
import { useT } from '../i18n';
import {
  list_models,
  save_model,
  type ModelInfo,
  type ModelProvider,
  type ModelProviderId,
  type ModelsInfo,
} from '../lib/ipc';
import { useStore } from '../store';

const PROVIDER_LABEL: Record<ModelProviderId, string> = {
  claude: 'Claude',
  codex: 'Codex',
  agy: 'Antigravity',
  ollama: 'Ollama',
};

const PROVIDER_CLASS: Record<ModelProviderId, string> = {
  claude: 'claude',
  codex: 'codex',
  agy: 'agy',
  ollama: 'ollama',
};

export const Models: React.FC = () => {
  const t = useT();
  const setScreen = useStore((state) => state.setScreen);

  const [info, setInfo] = useState<ModelsInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setInfo(await list_models());
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const choose = async (provider: ModelProvider, model: ModelInfo | null) => {
    const next = model && provider.selected === model.id ? null : model?.id ?? null;
    const key = `${provider.id}:${next ?? 'default'}`;
    setSaving(key);
    try {
      const selection = await save_model(provider.id, next);
      setInfo((prev) =>
        prev
          ? {
              ...prev,
              providers: prev.providers.map((p) => ({ ...p, selected: selection[p.id] })),
            }
          : prev
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(null);
    }
  };

  const chosenCount = info?.providers.filter((p) => p.selected).length ?? 0;

  return (
    <div className="win" id="models">
      <SetupSide current="models" />

      <main className="main">
        <section className="start models">
          <h2>{t('models.title')}</h2>
          <p className="sub">
            {t('models.sub1')} <i>{t('models.sub2')}</i> {t('models.sub3')}
          </p>

          {error && (
            <p className="models-error" role="alert">
              {t('models.error', { error })}
            </p>
          )}

          {loading && !info && (
            <p className="models-loading" role="status" aria-live="polite">
              {t('models.loading')}
            </p>
          )}

          {info?.providers.map((provider) => {
            const label = PROVIDER_LABEL[provider.id];
            const popular = provider.models.filter((m) => m.popular);
            const isOpen = Boolean(expanded[provider.id]);
            const selectedModel = provider.models.find((m) => m.id === provider.selected);
            const hiddenCount = provider.models.length - popular.length;

            return (
              <div className={`grp models-grp models-${PROVIDER_CLASS[provider.id]}`} key={provider.id}>
                <h3>
                  <span className={`models-dot ${PROVIDER_CLASS[provider.id]}`} aria-hidden="true" />
                  {label}{' '}
                  <span>
                    {provider.found
                      ? t('models.source', { source: provider.source })
                      : t('models.notFound')}
                  </span>
                </h3>

                {!provider.found && (
                  <p className="models-note">{t(`models.install.${provider.id}`)}</p>
                )}

                {provider.error && provider.found && (
                  <p className="models-note is-warn">{provider.error}</p>
                )}

                {provider.found && provider.models.length === 0 && !provider.error && (
                  <p className="models-note">{t('models.empty')}</p>
                )}

                {provider.models.length > 0 && (
                  <>
                    <div className="models-current">
                      {selectedModel ? (
                        <>
                          <span className="k">
                            <CheckIcon />
                          </span>
                          {t('models.current', { model: selectedModel.label })}
                        </>
                      ) : (
                        t('models.default')
                      )}
                    </div>

                    <div
                      className="lang-row models-chips"
                      role="radiogroup"
                      aria-label={t('models.popularFor', { provider: label })}
                    >
                      {popular.map((model) => {
                        const active = provider.selected === model.id;
                        return (
                          <button
                            key={model.id}
                            type="button"
                            role="radio"
                            aria-checked={active}
                            className={`lang-btn ${active ? 'is-active' : ''}`}
                            disabled={saving !== null}
                            aria-label={model.label}
                            title={model.note || model.id}
                            onClick={() => void choose(provider, model)}
                          >
                            {model.label}
                          </button>
                        );
                      })}
                      <button
                        type="button"
                        role="radio"
                        aria-checked={!provider.selected}
                        className={`lang-btn models-default ${!provider.selected ? 'is-active' : ''}`}
                        disabled={saving !== null}
                        onClick={() => void choose(provider, null)}
                      >
                        {t('models.useDefault')}
                      </button>
                    </div>

                    {hiddenCount > 0 && (
                      <button
                        type="button"
                        className="models-more"
                        aria-expanded={isOpen}
                        onClick={() => setExpanded((prev) => ({ ...prev, [provider.id]: !isOpen }))}
                      >
                        {isOpen
                          ? t('models.hideAll')
                          : t('models.showAll', { count: provider.models.length })}
                      </button>
                    )}

                    {isOpen && (
                      <div className="rows models-rows" role="listbox" aria-label={t('models.allFor', { provider: label })}>
                        {provider.models.map((model) => {
                          const active = provider.selected === model.id;
                          return (
                            <button
                              key={model.id}
                              type="button"
                              role="option"
                              aria-selected={active}
                              aria-label={model.note ? `${model.label}, ${model.note}` : model.label}
                              className={`row models-row ${active ? 'is-active' : ''}`}
                              disabled={saving !== null}
                              onClick={() => void choose(provider, model)}
                            >
                              <span className="ic">{active ? <CheckIcon /> : <span className="models-ring" />}</span>
                              <div className="nm">
                                {model.label}
                                {model.note && <span>{model.note}</span>}
                              </div>
                              <div className="ok">
                                {model.popular ? t('models.popular') : model.id !== model.label ? model.id : ''}
                              </div>
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </>
                )}

                {provider.id === 'ollama' && provider.found && (
                  <p className="models-note">{t('models.ollamaNote')}</p>
                )}
              </div>
            );
          })}

          <div className="foot">
            <div className="n">
              {info
                ? chosenCount === 0
                  ? t('models.savedNone', { path: info.path })
                  : t('models.saved', { path: info.path, count: chosenCount })
                : ''}
            </div>
            <div style={{ display: 'flex', gap: '10px' }}>
              <button
                type="button"
                className="cta"
                style={{
                  background: 'var(--raised)',
                  color: 'var(--fg)',
                  boxShadow: 'var(--shadow)',
                }}
                disabled={loading}
                onClick={() => void load()}
              >
                {loading ? t('models.refreshing') : t('models.refresh')}
              </button>
              <button type="button" className="cta" onClick={() => setScreen('chat')}>
                {t('models.toChat')}
                <span>
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
