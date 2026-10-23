# Handoff: ключ подписи обновлений

Функциональность автообновления подключена, но публичного ключа
`~/.tauri/takt.key.pub` на момент реализации не было. Поэтому в
`src-tauri/tauri.conf.json` временно оставлено `"pubkey": ""`.

Владельцу перед первой подписанной сборкой нужно выполнить:

```sh
pnpm tauri signer generate -w ~/.tauri/takt.key
```

Затем вставить полное содержимое `~/.tauri/takt.key.pub` в поле
`plugins.updater.pubkey` файла `src-tauri/tauri.conf.json`. Приватный ключ
`~/.tauri/takt.key` и пароль хранить вне репозитория; в GitHub Actions передать их
только через секреты `TAURI_SIGNING_PRIVATE_KEY` и
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
