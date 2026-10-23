import React from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';

/** Переключатель Чат · Код · Оркестр, общий для трёх экранов. */
export const Tabs: React.FC = () => {
  const t = useT();
  const tab = useStore((state) => state.tab);
  const setTab = useStore((state) => state.setTab);

  return (
    <div className="seg" role="tablist" aria-label={t('tabs.label')}>
      <button role="tab" aria-pressed={tab === 'chat'} onClick={() => setTab('chat')}>
        {t('tabs.chat')}
      </button>
      <button role="tab" aria-pressed={tab === 'code'} onClick={() => setTab('code')}>
        {t('tabs.code')}
      </button>
      <button
        role="tab"
        aria-pressed={tab === 'orchestra'}
        onClick={() => setTab('orchestra')}
      >
        {t('tabs.orchestra')}
      </button>
    </div>
  );
};
