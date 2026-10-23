// Проверка словарей: одинаковые наборы ключей и одинаковые {плейсхолдеры} во всех языках.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = join(dirname(fileURLToPath(import.meta.url)), '..', 'src', 'i18n');
const locales = ['uk', 'en', 'ru'];

const dicts = Object.fromEntries(
  locales.map((loc) => [loc, JSON.parse(readFileSync(join(dir, `${loc}.json`), 'utf8'))])
);

const placeholders = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');

let failed = false;
const report = (msg) => {
  failed = true;
  console.error(`i18n: ${msg}`);
};

const allKeys = new Set(locales.flatMap((loc) => Object.keys(dicts[loc])));

for (const key of allKeys) {
  const missing = locales.filter((loc) => !(key in dicts[loc]));
  if (missing.length) {
    report(`ключ «${key}» отсутствует в ${missing.join(', ')}`);
    continue;
  }
  for (const loc of locales) {
    const value = dicts[loc][key];
    if (typeof value !== 'string' || value.trim() === '') {
      report(`ключ «${key}» в ${loc} пустой или не строка`);
    }
  }
  const ref = placeholders(dicts.en[key]);
  for (const loc of locales) {
    if (placeholders(dicts[loc][key]) !== ref) {
      report(`ключ «${key}»: плейсхолдеры в ${loc} не совпадают с en`);
    }
  }
}

if (failed) process.exit(1);
console.log(`i18n: ${allKeys.size} ключей, ${locales.join('/')} согласованы`);
