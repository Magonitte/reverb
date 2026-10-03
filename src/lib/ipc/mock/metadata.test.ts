import { beforeEach, describe, expect, it } from "vitest";
import { mockCall, resetMockQueue, resetMockSettings } from "@/lib/ipc/mock";
import { FX1_VIDEO, FX2_MUSIC, FX3_CLIP } from "./fixtures";
import {
  mockFindOfficialVersion,
  mockMetadataFor,
  mockMetadataPreview,
  mockMetadataSearch,
} from "./metadata";
import { mockEnqueue, mockSimulationStep } from "./queue";

beforeEach(() => {
  resetMockQueue();
  resetMockSettings();
});

describe("mock da identificação (F08)", () => {
  it("FX3 (clipe) tem FX2 como versão oficial; FX1 e FX2 não", () => {
    expect(mockFindOfficialVersion(FX3_CLIP)).toMatchObject({
      videoId: "lYBUbBu4W08",
      url: "https://music.youtube.com/watch?v=lYBUbBu4W08",
      album: "Whenever You Need Somebody",
      via: "text",
    });
    expect(mockFindOfficialVersion(FX3_CLIP, "GBARL9300135")?.via).toBe("isrc");
    expect(mockFindOfficialVersion(FX1_VIDEO)).toBeNull();
    expect(mockFindOfficialVersion(FX2_MUSIC)).toBeNull();
  });

  it("FX1 não é música: sem candidatos nem confiança", () => {
    const result = mockMetadataFor(FX1_VIDEO);
    expect(result.contentType).toBe("other");
    expect(result.bucket).toBe("none");
    expect(result.candidates).toEqual([]);
    expect(result.fields.artist).toBe("jawed");
  });

  it("FX2 é oficial: fonte youtube_music e confiança 1", () => {
    const result = mockMetadataFor(FX2_MUSIC);
    expect(result).toMatchObject({ source: "youtube_music", confidence: 1, bucket: "auto" });
    expect(result.fields).toMatchObject({ album: "Whenever You Need Somebody", year: 1987 });
  });

  it("FX3 com a oficial usa os dados de FX2; sem ela, os do clipe + iTunes", () => {
    const withOfficial = mockMetadataFor(FX3_CLIP, { useOfficial: true });
    expect(withOfficial.source).toBe("youtube_music");
    expect(withOfficial.official?.videoId).toBe("lYBUbBu4W08");
    const without = mockMetadataFor(FX3_CLIP, { useOfficial: false });
    expect(without.source).toBe("itunes");
    expect(without.official?.videoId).toBe("lYBUbBu4W08");
    expect(without.fields.title).toBe("Never Gonna Give You Up");
    expect(without.fields.artist).toBe("Rick Astley");
  });

  it("a edição do usuário vence tudo", () => {
    const result = mockMetadataFor(FX3_CLIP, { override: { artist: "Eu", year: 2001 } });
    expect(result).toMatchObject({ source: "user", confidence: 1, bucket: "auto" });
    expect(result.fields).toMatchObject({ artist: "Eu", year: 2001 });
    expect(result.official).toEqual(mockFindOfficialVersion(FX3_CLIP));
  });

  it("o comando de pré-visualização respeita preferOfficialAudio e a escolha do usuário", () => {
    const url = FX3_CLIP.webpageUrl!;
    expect(mockMetadataPreview({ url, video: FX3_CLIP }).source).toBe("youtube_music");
    expect(mockMetadataPreview({ url, video: FX3_CLIP, useOfficial: false }).source).toBe("itunes");
    // Sem o vídeo, o mock analisa a URL.
    expect(mockMetadataPreview({ url }).fields.title).toBe("Never Gonna Give You Up");
  });

  it("pré-visualizar uma coleção é recusado", () => {
    expect(() =>
      mockMetadataPreview({ url: "https://www.youtube.com/playlist?list=PLabc" }),
    ).toThrowError(expect.objectContaining({ kind: "unavailable" }));
  });

  it("busca manual devolve candidatos só para consultas conhecidas", () => {
    expect(mockMetadataSearch("rick astley never gonna give you up")).toHaveLength(1);
    expect(mockMetadataSearch("   ")).toEqual([]);
    expect(mockMetadataSearch("desconhecido")).toEqual([]);
  });

  it("os comandos estão no mock", async () => {
    expect(await mockCall("find_official_version", { video: FX3_CLIP })).toMatchObject({
      videoId: "lYBUbBu4W08",
    });
    expect(
      await mockCall("metadata_preview", { request: { url: FX2_MUSIC.webpageUrl } }),
    ).toMatchObject({ source: "youtube_music" });
    expect(await mockCall("metadata_search", { query: "rick" })).toHaveLength(1);
  });

  it("a simulação grava o resultado, a confiança e o artista no job concluído", () => {
    const job = mockEnqueue({
      url: FX3_CLIP.webpageUrl!,
      sourceId: FX3_CLIP.id,
      metadataOverride: null,
      priority: false,
      allowDuplicate: false,
    });
    for (let i = 0; i < 20 && job.status !== "done"; i += 1) mockSimulationStep();
    expect(job.status).toBe("done");
    expect(job.artist).toBe("Rick Astley");
    expect(job.title).toBe("Never Gonna Give You Up");
    expect(job.confidence).toBeGreaterThan(0.9);
    expect(job.metadataResult?.bucket).toBe("auto");
  });

  it("job com override leva a edição para o resultado", () => {
    const job = mockEnqueue({
      url: FX3_CLIP.webpageUrl!,
      sourceId: FX3_CLIP.id,
      metadataOverride: { artist: "Eu" },
      priority: false,
      allowDuplicate: false,
    });
    for (let i = 0; i < 20 && job.status !== "done"; i += 1) mockSimulationStep();
    expect(job.metadataOverride).toEqual({ artist: "Eu" });
    expect(job.artist).toBe("Eu");
    expect(job.metadataResult?.source).toBe("user");
    expect(job.confidence).toBe(1);
  });
});
