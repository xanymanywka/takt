import React, { useEffect } from 'react';
import { useT } from '../i18n';
import { CONDUCTOR_AGENTS, type ConductorAgent, type ConductorOption } from '../lib/ipc';
import { useStore } from '../store';
import { AgentLogo } from './BrandLogos';
import { CheckIcon } from './Icons';

/** Полные названия CLI на карточках; короткие имена для подписей — в store.conductorName. */
const TITLES: Record<ConductorAgent, string> = {
  claude: 'Claude Code',
  codex: 'Codex CLI',
  agy: 'Antigravity',
};

/** Карточка доступна, если CLI найден и вход не отвергнут явно. */
export function conductorReady(option: ConductorOption | undefined): boolean {
  return Boolean(option && option.found && option.loggedIn !== false);
}

interface Props {
  /** В Настройках карточки ниже и без длинной подсказки под недоступными. */
  compact?: boolean;
}

/** Выбор дирижёра: три карточки, одна активная. Общий для мастера и Настроек. */
export const ConductorChoice: React.FC<Props> = ({ compact = false }) => {
  const t = useT();
  const conductorAgent = useStore((state) => state.conductorAgent);
  const options = useStore((state) => state.conductorOptions);
  const setConductor = useStore((state) => state.setConductor);
  const refreshConductor = useStore((state) => state.refreshConductor);

  useEffect(() => {
    void refreshConductor();
  }, [refreshConductor]);

  const byId = (id: ConductorAgent) => options.find((option) => option.id === id);

  return (
    <div className={`conductor-cards ${compact ? 'compact' : ''}`} role="radiogroup" aria-label={t('conductor.title')}>
      {CONDUCTOR_AGENTS.map((id) => {
        const option = byId(id);
        const checking = options.length === 0;
        const ready = conductorReady(option);
        const active = conductorAgent === id;
        const status = checking
          ? t('conductor.checking')
          : !option?.found
            ? t('conductor.notFound')
            : option.loggedIn === false
              ? t('conductor.loggedOut')
              : option.loggedIn === undefined
                ? t('conductor.found')
                : t('conductor.loggedIn');
        const cls = checking ? 'off' : ready ? (option?.loggedIn === undefined ? 'warn' : 'good') : 'off';
        return (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={!ready && !checking}
            className={`conductor-card ${active ? 'is-active' : ''} ${id}`}
            onClick={() => void setConductor(id)}
          >
            <span className="conductor-card__logo" aria-hidden="true">
              <AgentLogo agent={id} size={compact ? 18 : 24} />
            </span>
            <span className="conductor-card__body">
              <b>{TITLES[id]}</b>
              <span className={`conductor-card__status ${cls}`}>{status}</span>
              {!compact && !ready && !checking && (
                <span className="conductor-card__hint">{t('conductor.unavailableHint')}</span>
              )}
            </span>
            {active && (
              <span className="conductor-card__check" aria-hidden="true">
                <CheckIcon />
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};
