import { ESLint } from "eslint";
import { describe, expect, it } from "vitest";

const eslint = new ESLint();
const FILE = "src/components/exemplo.tsx";

async function ruleIds(code: string): Promise<string[]> {
  const [result] = await eslint.lintText(code, { filePath: FILE });
  return result.messages.map((m) => m.ruleId ?? "");
}

describe("lint i18n (T7)", () => {
  it("pega texto literal em JSX", async () => {
    expect(await ruleIds("export const Comp = () => <div>Olá</div>;")).toContain(
      "i18next/no-literal-string",
    );
  });

  it("pega texto literal em aria-label", async () => {
    expect(await ruleIds('export const Comp = () => <button aria-label="Fechar" />;')).toContain(
      "i18next/no-literal-string",
    );
  });

  it("aceita texto vindo de t()", async () => {
    const code = 'export const Comp = ({ t }: { t: (k: string) => string }) => <div>{t("x")}</div>;';
    expect(await ruleIds(code)).not.toContain("i18next/no-literal-string");
  });
});
