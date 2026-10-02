import { mockSettingsGet } from "./settings";
import { mockCalls } from "./media";

export function mockTemplatePreview(template: string): string {
  const settings = mockSettingsGet();
  if (!template.trim()) throw { kind: "invalid", message: "Modelo vazio" };
  const values: Record<string, string> = {
    artist: "Rick Astley", albumartist: "Rick Astley", album: "Whenever You Need Somebody",
    title: "Never Gonna Give You Up", track: "1", disc: "1", year: "1987", genre: "Pop",
    channel: "Rick Astley", source_id: "lYBUbBu4W08", playlist: "Favorites", playlist_index: "1",
  };
  const expanded = template.replace(/\{([^{}]+)\}/g, (_, body: string) => {
    const [name, padding] = body.split(":");
    if (!(name in values) || (padding !== undefined &&
      (!/^[0-9]{1,2}$/.test(padding) || !["track", "disc", "playlist_index"].includes(name)))) {
      throw { kind: "invalid", message: "Variável ou formato inválido" };
    }
    return padding === undefined ? values[name] : values[name].padStart(Number(padding), "0");
  });
  if (/[{}]/.test(expanded)) throw { kind: "invalid", message: "Modelo inválido" };
  const model = settings.autoOrganize ? expanded : `${values.artist} - ${values.title}`;
  const parts = model.split(/[\\/]/).filter(Boolean).map((part) => {
    let safe = [...part.normalize("NFC")].map((char) => char.charCodeAt(0) < 32 ? "_" : char).join("")
      .replaceAll(":", " -").replace(/[<>"|?*]/g, "_").trim().replace(/[. ]+$/, "");
    if (/^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)/i.test(safe)) safe = `_${safe}`;
    return [...safe].slice(0, 120).join("") || "_";
  });
  const stem = parts.pop() ?? "_";
  return [...parts, `${[...stem].slice(0, 115).join("")}.opus`].join("/");
}

export function mockLibraryCover(id: number): string | null {
  mockCalls.push({ cmd: "library_cover", args: id });
  if (!Number.isInteger(id) || id <= 0) throw { kind: "file_missing", message: "Registro ausente" };
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256"><rect width="256" height="256" fill="#302819"/><circle cx="128" cy="128" r="90" fill="#cf9348"/><circle cx="128" cy="128" r="30" fill="#17141f"/></svg>';
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

export function mockLibraryOpenFile(path: string): void {
  if (!path) throw { kind: "file_missing", message: "Arquivo ausente" };
  mockCalls.push({ cmd: "library_open_file", args: path });
}
