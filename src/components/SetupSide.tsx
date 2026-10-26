import React from 'react';
import { SetupNav, type SetupStep } from './SetupNav';
import { Support } from './Support';

interface SetupSideProps {
  current: SetupStep;
  children?: React.ReactNode;
}

/** Левая колонка экранов настройки: навигация по разделам и плашка «Поддержать». */
export const SetupSide: React.FC<SetupSideProps> = ({ current, children }) => (
  <aside className="side">
    <SetupNav current={current} />
    {children}
    <div className="side-foot">
      <Support compact />
    </div>
  </aside>
);
