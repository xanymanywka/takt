import React from 'react';
import { ArrowRightIcon, CheckIcon, CircleTodoIcon, DotIcon } from './Icons';
import { useT } from '../i18n';
import { useStore, type ScreenType } from '../store';

export type SetupStep = 'access' | 'models' | 'settings' | 'about';

interface SetupNavProps {
  current: SetupStep;
}

type ItemState = 'done' | 'now' | 'todo';

/**
 * Левая колонка экранов настройки. Состояния реальные: «Доступы» и «Модели» считаются
 * готовыми, когда данные загружены; «Инструкции» — когда есть ~/.orchestra/INSTRUCTIONS.md.
 */
export const SetupNav: React.FC<SetupNavProps> = ({ current }) => {
  const t = useT();
  const setScreen = useStore((state) => state.setScreen);
  const setup = useStore((state) => state.setup);
  const projects = useStore((state) => state.projects);

  const tools = setup?.tools ?? [];
  const toolsReady = tools.length > 0 && tools.every((tool) => tool.found && tool.loggedIn !== false);
  const sshReady = (setup?.ssh?.length ?? 0) > 0;
  const instructionsReady = Boolean(setup?.instructionsExists);
  const projectsReady = projects.length > 0;

  const icon = (state: ItemState) => (
    <span className={`k ${state === 'now' ? 'now' : state === 'todo' ? 'todo' : ''}`}>
      {state === 'done' && <CheckIcon />}
      {state === 'now' && <DotIcon />}
      {state === 'todo' && <CircleTodoIcon />}
    </span>
  );

  const item = (label: string, state: ItemState, screen: ScreenType, isCurrent: boolean) => (
    <li aria-current={isCurrent ? 'true' : undefined} className={state === 'todo' ? 'off' : undefined}>
      <button
        type="button"
        className="setnav-link"
        aria-current={isCurrent ? 'page' : undefined}
        onClick={() => setScreen(screen)}
      >
        {icon(isCurrent ? 'now' : state)}
        {label}
      </button>
    </li>
  );

  return (
    <div className="proj">
      <h3>{t('start.nav.title')}</h3>
      <ul className="setnav">
        {item(t('start.nav.workers'), toolsReady ? 'done' : 'todo', 'start', current === 'access')}
        {item(t('start.nav.projects'), projectsReady ? 'done' : 'todo', 'start', false)}
        {item(t('start.nav.access'), sshReady ? 'done' : 'todo', 'start', current === 'access')}
        {item(t('start.nav.instructions'), instructionsReady ? 'done' : 'todo', 'start', false)}
        {item(t('start.nav.models'), 'done', 'models', current === 'models')}
        {item(t('start.nav.settings'), 'done', 'settings', current === 'settings')}
        <li className="about-nav" aria-current={current === 'about' ? 'true' : undefined}>
          <button
            type="button"
            className="setnav-link"
            aria-current={current === 'about' ? 'page' : undefined}
            onClick={() => setScreen('about')}
          >
            <span className="k">
              <ArrowRightIcon />
            </span>
            {t('start.nav.about')}
          </button>
        </li>
      </ul>
    </div>
  );
};
