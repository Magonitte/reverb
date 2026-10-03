import { describe, expect, it } from "vitest";
import { FX2_MUSIC } from "@/lib/ipc/mock/fixtures";
import { mockMetadataFor } from "@/lib/ipc/mock/metadata";
import { buildOverride, fieldText, isEdited } from "@/lib/metadataEdit";

const result = mockMetadataFor(FX2_MUSIC);

describe("buildOverride", () => {
  it("sem edição não há override", () => {
    expect(buildOverride(result, {})).toBeNull();
    expect(buildOverride(null, { title: "x" })).toBeNull();
    expect(isEdited(result, {})).toBe(false);
  });

  it("campo igual ao do resultado não conta como edição", () => {
    expect(buildOverride(result, { artist: "Rick Astley", year: "1987" })).toBeNull();
    expect(buildOverride(result, { artist: "  Rick Astley  " })).toBeNull();
  });

  it("só os campos alterados vão, com número como número", () => {
    const override = buildOverride(result, {
      artist: "Outro",
      year: "1988",
      trackNo: "3",
      album: "Whenever You Need Somebody",
    });
    expect(override).toEqual({ artist: "Outro", year: 1988, trackNo: 3 });
    expect(isEdited(result, { artist: "Outro" })).toBe(true);
  });

  it("texto vazio e número inválido ficam de fora", () => {
    expect(buildOverride(result, { title: "   ", genre: "" })).toBeNull();
    expect(buildOverride(result, { year: "abc", trackNo: "-2" })).toBeNull();
    expect(buildOverride(result, { year: "19.5" })).toBeNull();
  });

  it("fieldText devolve texto de input", () => {
    expect(fieldText(result.fields, "title")).toBe("Never Gonna Give You Up");
    expect(fieldText(result.fields, "year")).toBe("1987");
    expect(fieldText({ ...result.fields, genre: null }, "genre")).toBe("");
  });
});
