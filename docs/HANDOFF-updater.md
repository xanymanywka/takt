# Ключ подписи обновлений

Статус: выполнено 23.10.2026.

- Пара ключей сгенерирована: `pnpm tauri signer generate -w ~/.tauri/takt.key` (без пароля).
- Публичный ключ вставлен в `plugins.updater.pubkey` файла `src-tauri/tauri.conf.json`.
- Приватный ключ лежит только в `~/.tauri/takt.key` (права 0600) и в секрете репозитория
  `TAURI_SIGNING_PRIVATE_KEY`. В репозиторий он не попадает.
- Секрет `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` не создан: ключ без пароля, пустая переменная
  в `release.yml` этому не мешает.

Локальная подписанная сборка:

```sh
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/takt.key"   # путь к файлу или само содержимое
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
pnpm tauri build
```

Без этих переменных `pnpm tauri build` собирает `.app` и `.dmg`, но завершается
кодом 1 на шаге подписи updater-артефакта (`Takt.app.tar.gz.sig`). Это не ломает
установку вручную, только автообновление.

Релиз: пуш тега `v<версия>` запускает `.github/workflows/release.yml`, который кладёт в
GitHub Releases universal `.dmg`, `.app.tar.gz`, `.sig` и `latest.json`.

Если ключ утерян: сгенерировать новый, заменить pubkey в конфиге, обновить секрет и выпустить
версию; установленные старые сборки её не примут (подпись не совпадёт), их нужно переустановить
из `.dmg` вручную.
