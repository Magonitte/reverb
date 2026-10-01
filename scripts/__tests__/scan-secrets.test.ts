import { describe, expect, it } from "vitest";
import { scanContent } from "../scan-secrets.mjs";

describe("scan-secrets (T9)", () => {
  it("detecta uma chave privada falsa", () => {
    const hits = scanContent("linha ok\n-----BEGIN RSA PRIVATE KEY-----\nabc");
    expect(hits).toEqual([{ line: 2, pattern: "chave privada" }]);
  });

  it("detecta client_secret e access_token", () => {
    expect(scanContent('client_secret = "abcdefgh12345"')).toHaveLength(1);
    expect(scanContent('{"access_token": "abcdefghijklmnopqrstuvwxyz"}')).toHaveLength(1);
  });

  it("não acusa texto comum", () => {
    expect(scanContent("const x = 1;\n// nada secreto aqui")).toEqual([]);
  });
});
