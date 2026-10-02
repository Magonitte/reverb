import type { SettingsPatch } from "@/bindings/SettingsPatch";
import type { SettingsView } from "@/bindings/SettingsView";
import { mockBus } from "./bus";
import { mockToolsStatus, mockRuntimeChoices } from "./tools";

const SECRET_KEYS = [
  "acoustidKey",
  "spotifyClientId",
  "spotifyClientSecret",
  "discogsToken",
  "jamendoClientId",
] as const;

const PROFILES = ["original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac"];

export const DEFAULT_SETTINGS: SettingsView = {
  outputDir: "",
  fileTemplate: "{albumartist}/{album}/{track:02} - {title}",
  autoOrganize: true,
  defaultProfile: "original",
  parallelism: 2,
  speedLimitMbps: 0,
  queueLimit: 500,
  maxAttempts: 3,
  fetchMetadata: true,
  extractTitleFromVideo: true,
  preferOfficialAudio: true,
  confidenceAutoApply: 0.85,
  confidenceReview: 0.6,
  offlineMode: false,
  fetchArtwork: true,
  writeFolderCover: true,
  fetchLyrics: true,
  writeLrcFile: true,
  normalizeVolume: true,
  sponsorblockRemove: false,
  sponsorblockCategories: ["music_offtopic"],
  splitChapters: "ask",
  trimSilence: false,
  playlistPacingSeconds: 3,
  watchLibrary: true,
  theme: "dark",
  language: "pt-BR",
  transparency: "auto",
  launchAtStartup: false,
  startMinimized: true,
  minimizeToTray: true,
  closeToTray: true,
  completionNotifications: true,
  clipboardWatch: false,
  globalShortcut: "",
  weeklySelfTest: true,
  // Most browser scenarios represent an already configured installation.
  onboardingCompleted: true,
  cookiesSource: "none",
  cookiesFile: "",
  ytdlpChannel: "stable",
  jsRuntime: "auto",
  autoUpdateTools: true,
  potProvider: "auto",
  qualityTargetKbps: 0,
  autoUpgrade: false,
  artistCheckIntervalHours: 24,
  autoCheckAppUpdates: true,
  verifyLosslessOnImport: true,
  secretsStatus: { acoustid: false, spotify: false, discogs: false, jamendo: false },
};

type SecretValues = Partial<Record<(typeof SECRET_KEYS)[number], string>>;

let current: SettingsView = structuredClone(DEFAULT_SETTINGS);
let secrets: SecretValues = {};

function fail(field: string, message: string): never {
  throw { kind: "invalid", message, i18nKey: `errors.settings.${field}` };
}

function range(field: string, value: number, min: number, max: number) {
  if (typeof value !== "number" || value < min || value > max) {
    fail(field, `${field} deve estar entre ${min} e ${max} (recebido ${value})`);
  }
}

/** Validações básicas, espelhando o backend Rust (arquitetura §6). */
function validate(next: SettingsView) {
  if (next.outputDir !== "" && !/^([a-zA-Z]:[\\/]|\/)/.test(next.outputDir)) {
    fail("outputDir", "a pasta de destino deve ser um caminho absoluto (ou vazia para o padrão)");
  }
  if (!PROFILES.includes(next.defaultProfile)) {
    fail("defaultProfile", `perfil desconhecido: ${next.defaultProfile}`);
  }
  range("parallelism", next.parallelism, 1, 4);
  range("speedLimitMbps", next.speedLimitMbps, 0, 1000);
  range("queueLimit", next.queueLimit, 10, 5000);
  range("maxAttempts", next.maxAttempts, 1, 10);
  range("confidenceAutoApply", next.confidenceAutoApply, 0.5, 1);
  range("confidenceReview", next.confidenceReview, 0.3, 0.95);
  if (next.confidenceAutoApply <= next.confidenceReview) {
    fail("confidenceAutoApply", "confidenceAutoApply deve ser maior que confidenceReview");
  }
  range("playlistPacingSeconds", next.playlistPacingSeconds, 0, 60);
  range("artistCheckIntervalHours", next.artistCheckIntervalHours, 6, 168);
  if (![0, 160, 256].includes(next.qualityTargetKbps)) {
    fail("qualityTargetKbps", "qualityTargetKbps deve ser 0, 160 ou 256");
  }
}

function statusOf(values: SecretValues): SettingsView["secretsStatus"] {
  return {
    acoustid: !!values.acoustidKey,
    spotify: !!values.spotifyClientId && !!values.spotifyClientSecret,
    discogs: !!values.discogsToken,
    jamendo: !!values.jamendoClientId,
  };
}

export function mockSettingsGet(): SettingsView {
  return structuredClone(current);
}

export function mockSettingsUpdate(patch: SettingsPatch): SettingsView {
  const next: SettingsView = structuredClone(current);
  const nextSecrets: SecretValues = { ...secrets };
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined || value === null) continue;
    if ((SECRET_KEYS as readonly string[]).includes(key)) {
      nextSecrets[key as keyof SecretValues] = value as string;
    } else if (key in next && key !== "secretsStatus") {
      (next as unknown as Record<string, unknown>)[key] = value;
    } else {
      fail("unknown", `chave de configuração desconhecida: ${key}`);
    }
  }
  validate(next);
  if (
    next.onboardingCompleted &&
    !current.onboardingCompleted &&
    (!["ytdlp", "ffmpeg"].every((tool) =>
      mockToolsStatus().some((status) => status.tool === tool && status.installed),
    ) ||
      (next.jsRuntime === "auto"
        ? !mockRuntimeChoices().length
        : !mockRuntimeChoices().includes(next.jsRuntime)))
  ) {
    throw {
      kind: "invalid",
      message: "Required tools are missing",
      i18nKey: "onboarding.toolsRequired",
    };
  }
  next.secretsStatus = statusOf(nextSecrets);
  current = next;
  secrets = nextSecrets;
  mockBus.emit("settings://changed", structuredClone(current));
  return structuredClone(current);
}

export function mockSettingsReset(): SettingsView {
  current = { ...structuredClone(DEFAULT_SETTINGS), secretsStatus: statusOf(secrets) };
  mockBus.emit("settings://changed", structuredClone(current));
  return structuredClone(current);
}

/** Volta o backend falso ao estado inicial (testes). */
export function resetMockSettings() {
  current = structuredClone(DEFAULT_SETTINGS);
  secrets = {};
}
