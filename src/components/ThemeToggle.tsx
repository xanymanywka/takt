import React from 'react';
import { useStore } from '../store';
import { MoonIcon, SunIcon } from './Icons';

export const ThemeToggle: React.FC = () => {
  const toggleTheme = useStore((state) => state.toggleTheme);

  return (
    <button
      className="theme"
      id="theme"
      onClick={toggleTheme}
      aria-label="Переключить тему"
      title="Ночь / день"
    >
      <SunIcon className="sun" />
      <MoonIcon className="moon" />
    </button>
  );
};
