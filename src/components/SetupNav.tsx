import React from 'react';
import { ArrowRightIcon, CheckIcon, CircleTodoIcon, DotIcon } from './Icons';
import { useT } from '../i18n';
import { useStore } from '../store';

export type SetupStep = 'access' | 'models';

interface SetupNavProps {
  current: SetupStep;
}

/** Левая колонка онбординга: пройденные, текущий и будущие шаги + «О программе». */
export const SetupNav: React.FC<SetupNavProps> = ({ current }) => {
  const t = useT();
  const setScreen = useStore((state) => state.setScreen);

  const stepItem = (label: string, state: 'done' | 'now' | 'todo') => (
    <li aria-current={state === 'now' ? 'true' : undefined} className={state === 'todo' ? 'off' : undefined}>
      <span className={`k ${state === 'now' ? 'now' : state === 'todo' ? 'todo' : ''}`}>
        {state === 'done' && <CheckIcon />}
        {state === 'now' && <DotIcon />}
        {state === 'todo' && <CircleTodoIcon />}
      </span>
      {label}
    </li>
  );

  const linkItem = (label: string, screen: SetupStep, state: 'done' | 'now' | 'todo') => (
    <li aria-current={state === 'now' ? 'true' : undefined} className={state === 'todo' ? 'off' : undefined}>
      <button
        type="button"
        className="setnav-link"
        aria-current={state === 'now' ? 'page' : undefined}
        onClick={() => setScreen(screen === 'access' ? 'start' : 'models')}
      >
        <span className={`k ${state === 'now' ? 'now' : state === 'todo' ? 'todo' : ''}`}>
          {state === 'done' && <CheckIcon />}
          {state === 'now' && <DotIcon />}
          {state === 'todo' && <CircleTodoIcon />}
        </span>
        {label}
      </button>
    </li>
  );

  const accessState = current === 'access' ? 'now' : 'done';
  const modelsState = current === 'models' ? 'now' : 'todo';

  return (
    <div className="proj">
      <h3>{t('start.nav.title')}</h3>
      <ul className="setnav">
        {stepItem(t('start.nav.conductor'), 'done')}
        {stepItem(t('start.nav.workers'), 'done')}
        {stepItem(t('start.nav.import'), 'done')}
        {stepItem(t('start.nav.github'), 'done')}
        {linkItem(t('start.nav.access'), 'access', accessState)}
        {stepItem(t('start.nav.instructions'), 'todo')}
        {linkItem(t('start.nav.models'), 'models', modelsState)}
        {stepItem(t('start.nav.skills'), 'todo')}
        <li className="about-nav">
          <button type="button" className="setnav-link" onClick={() => setScreen('about')}>
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
