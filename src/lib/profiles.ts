/** Perfis de saída (arquitetura §7), espelho de `reverb-core::profiles`. */
export interface ProfileOption {
  id: string;
  /** Chave i18n do rótulo. */
  labelKey: string;
  reencodes: boolean;
}

export const PROFILE_OPTIONS: ProfileOption[] = [
  { id: "original", labelKey: "profiles.original", reencodes: false },
  { id: "mp3_v0", labelKey: "profiles.mp3_v0", reencodes: true },
  { id: "mp3_320", labelKey: "profiles.mp3_320", reencodes: true },
  { id: "aac_256", labelKey: "profiles.aac_256", reencodes: true },
  { id: "opus_96", labelKey: "profiles.opus_96", reencodes: true },
  { id: "flac", labelKey: "profiles.flac", reencodes: true },
];

export function profileReencodes(id: string): boolean {
  return PROFILE_OPTIONS.find((p) => p.id === id)?.reencodes ?? false;
}
