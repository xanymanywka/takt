import type { JobState } from './ipc';

const JOB_STATE_LABELS: Record<JobState, string> = {
  running: 'пишет',
  asking: 'ждёт вас',
  done: 'готово',
  limit: 'лимит',
  killed: 'остановлено',
  failed: 'ошибка',
  exited: 'вышел',
};

export function jobStateLabel(state: JobState): string {
  return JOB_STATE_LABELS[state] ?? state;
}

export function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

const ruPlural = new Intl.PluralRules('ru');

export function pluralRu(n: number, one: string, few: string, many: string): string {
  const form = ruPlural.select(n);
  return form === 'one' ? one : form === 'few' ? few : many;
}
