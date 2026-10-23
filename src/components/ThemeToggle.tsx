import React from 'react';
import { useT } from '../i18n';
import { useStore } from '../store';
import { MoonIcon, SunIcon } from './Icons';

export const ThemeToggle: React.FC = () => {
  const t = useT();
  const toggleTheme = useStore((state) => state.toggleTheme);

  return (
    <button
      className="theme"
      id="theme"
      onClick={toggleTheme}
      aria-label={t('theme.toggle')}
      title={t('theme.title')}
    >
      <SunIcon className="sun" />
      <MoonIcon className="moon" />
    </button>
  );
};
