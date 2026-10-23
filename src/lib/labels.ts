import { t } from '../i18n';
import type { JobState } from './ipc';

/** Подпись состояния задачи на текущем языке интерфейса. */
export function jobStateLabel(state: JobState): string {
  const key = `job.state.${state}`;
  const label = t(key);
  return label === key ? state : label;
}

export function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
