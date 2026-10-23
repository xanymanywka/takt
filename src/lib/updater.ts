import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { isTauriEnv } from './ipc';
import { t } from '../i18n';

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
}

let pendingUpdate: Update | null = null;

export async function discardUpdate(): Promise<void> {
  const update = pendingUpdate;
  pendingUpdate = null;
  if (update) await update.close();
}

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauriEnv()) return null;

  const update = await check();
  pendingUpdate = update;

  if (!update) return null;

  return {
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date,
    body: update.body,
  };
}

export async function installUpdate(onProgress: (progress: number) => void): Promise<void> {
  if (!pendingUpdate) {
    throw new Error(t('update.notFound'));
  }

  let downloaded = 0;
  let contentLength = 0;

  await pendingUpdate.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      downloaded = 0;
      contentLength = event.data.contentLength ?? 0;
      onProgress(0);
      return;
    }

    if (event.event === 'Progress') {
      downloaded += event.data.chunkLength;
      if (contentLength > 0) {
        onProgress(Math.min(99, Math.round((downloaded / contentLength) * 100)));
      }
      return;
    }

    onProgress(100);
  });

  pendingUpdate = null;
  await relaunch();
}
