#!/usr/bin/env node
// Рендерит src-tauri/dmg/background.html в background.png (2x) через установленный Google Chrome.
// Запуск: node scripts/dmg-background.mjs
import { execFileSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(new URL('..', import.meta.url).pathname);
const html = resolve(root, 'src-tauri/dmg/background.html');
const png = resolve(root, 'src-tauri/dmg/background.png');
const chrome = [
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/Applications/Chromium.app/Contents/MacOS/Chromium',
].find((p) => existsSync(p));

if (!chrome) {
  console.error('dmg-background: нужен Google Chrome или Chromium в /Applications');
  process.exit(1);
}

execFileSync(
  chrome,
  [
    '--headless=new',
    '--disable-gpu',
    '--hide-scrollbars',
    '--allow-file-access-from-files',
    '--force-device-scale-factor=2',
    '--window-size=660,400',
    `--screenshot=${png}`,
    `file://${html}`,
  ],
  { stdio: 'inherit' }
);

// Finder показывает PNG 1:1 по пикселям; метка 144 dpi говорит ему, что это 2x-картинка для 660x400.
execFileSync('sips', ['-s', 'dpiWidth', '144', '-s', 'dpiHeight', '144', png], { stdio: 'ignore' });

const size = statSync(png).size;
console.log(`dmg-background: ${png} (${Math.round(size / 1024)} KiB)`);
