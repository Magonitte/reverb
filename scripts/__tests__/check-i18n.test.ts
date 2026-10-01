import { describe, expect, it } from "vitest";
import { compareLocales } from "../check-i18n.mjs";

describe("check-i18n (T8)", () => {
  it("aceita dicionários com as mesmas chaves", () => {
    expect(compareLocales({ a: { b: "x" } }, { a: { b: "y" } })).toEqual([]);
  });

  it("falha quando falta uma chave", () => {
    const problems = compareLocales({ a: "x", b: "y" }, { a: "x" });
    expect(problems).toHaveLength(1);
    expect(problems[0]).toContain('"b"');
  });

  it("falha com valor vazio", () => {
    expect(compareLocales({ a: "x" }, { a: " " }).length).toBeGreaterThan(0);
  });
});
