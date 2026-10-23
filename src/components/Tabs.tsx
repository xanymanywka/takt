import React from 'react';
import { useT } from '../i18n';
import { useStore, type TabType } from '../store';

const TABS: Array<{ id: TabType; key: string }> = [
  { id: 'chat', key: 'tabs.chat' },
  { id: 'code', key: 'tabs.code' },
  { id: 'orchestra', key: 'tabs.orchestra' },
];

/** Переключатель Чат · Код · Оркестр, общий для трёх экранов. */
export const Tabs: React.FC = () => {
  const t = useT();
  const tab = useStore((state) => state.tab);
  const setTab = useStore((state) => state.setTab);

  return (
    <div className="seg" role="tablist" aria-label={t('tabs.label')}>
      {TABS.map(({ id, key }) => (
        <button
          key={id}
          type="button"
          role="tab"
          aria-selected={tab === id}
          tabIndex={tab === id ? 0 : -1}
          onClick={() => setTab(id)}
        >
          {t(key)}
        </button>
      ))}
    </div>
  );
};
