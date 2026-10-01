/** Resposta de `updater_check` (o tipo vive no app Tauri, não no core). */
export interface AppUpdateInfo {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}
