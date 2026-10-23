import React from 'react';
import { LOCALES, useLocale, useT } from '../i18n';
import { useStore } from '../store';

/** Компактный переключатель языка рядом с темой: по клику идёт по кругу uk → en → ru. */
export const LocaleToggle: React.FC = () => {
  const t = useT();
  const locale = useLocale();
  const setLocale = useStore((state) => state.setLocale);

  const next = LOCALES[(LOCALES.indexOf(locale) + 1) % LOCALES.length];

  return (
    <button
      type="button"
      className="theme locale"
      onClick={() => setLocale(next)}
      aria-label={t('locale.toggle')}
      title={t(`locale.${next}`)}
    >
      {locale.toUpperCase()}
    </button>
  );
};
