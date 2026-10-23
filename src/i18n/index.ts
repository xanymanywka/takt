import { useSyncExternalStore } from 'react';
import en from './en.json';
import ru from './ru.json';
import uk from './uk.json';

export type Locale = 'uk' | 'en' | 'ru';

export const LOCALES: Locale[] = ['uk', 'en', 'ru'];

/** Теги для Intl: даты и числа форматируются по выбранному языку. */
export const INTL_TAGS: Record<Locale, string> = {
  uk: 'uk-UA',
  en: 'en-GB',
  ru: 'ru-RU',
};

const STORAGE_KEY = 'takt-locale';

type Dictionary = Record<string, string>;

const dictionaries: Record<Locale, Dictionary> = {
  uk: uk as Dictionary,
  en: en as Dictionary,
  ru: ru as Dictionary,
};

function isLocale(value: unknown): value is Locale {
  return value === 'uk' || value === 'en' || value === 'ru';
}

/** Определение языка: сохранённый выбор → системная локаль → английский. */
function detectLocale(): Locale {
  if (typeof window === 'undefined') return 'en';
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLocale(saved)) return saved;
  } catch {}
  const tag = (navigator.language || '').toLowerCase();
  if (tag.startsWith('uk')) return 'uk';
  if (tag.startsWith('ru') || tag.startsWith('be')) return 'ru';
  return 'en';
}

let current: Locale = detectLocale();
const listeners = new Set<() => void>();
const pluralRules = new Map<Locale, Intl.PluralRules>();

if (typeof document !== 'undefined') {
  document.documentElement.lang = current;
}

export function getLocale(): Locale {
  return current;
}

export function setLocale(locale: Locale): void {
  if (locale === current) return;
  current = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {}
  if (typeof document !== 'undefined') {
    document.documentElement.lang = locale;
  }
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Текущий язык как подписка React: компонент перерисуется при смене. */
export function useLocale(): Locale {
  return useSyncExternalStore(subscribe, getLocale, getLocale);
}

export type TParams = Record<string, string | number>;

function pluralForm(locale: Locale, count: number): string {
  let rules = pluralRules.get(locale);
  if (!rules) {
    rules = new Intl.PluralRules(INTL_TAGS[locale]);
    pluralRules.set(locale, rules);
  }
  return rules.select(count);
}

/**
 * Перевод по плоскому ключу. `{name}` в строке заменяется на params.name.
 * Если передан `count` и есть ключи `key.one` / `key.few` / `key.many` / `key.other`,
 * форма выбирается через Intl.PluralRules.
 */
export function t(key: string, params?: TParams): string {
  const dict = dictionaries[current];
  let template: string | undefined;

  if (params && typeof params.count === 'number') {
    const form = pluralForm(current, params.count);
    template = dict[`${key}.${form}`] ?? dict[`${key}.other`];
  }
  template ??= dict[key] ?? dictionaries.en[key];

  if (template === undefined) {
    if (import.meta.env.DEV) console.warn(`i18n: нет ключа «${key}» для ${current}`);
    return key;
  }
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match
  );
}

/** Хук: подписывает компонент на смену языка и отдаёт функцию перевода. */
export function useT(): typeof t {
  useLocale();
  return t;
}
