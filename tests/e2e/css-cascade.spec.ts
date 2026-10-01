import { expect, test } from "@playwright/test";
import { open } from "./helpers";

// T6 — guarda da cascata CSS (lição do projeto antigo): estilos globais fora de @layer base
// vencem as utilidades do Tailwind e "quebram" classes.
test("T6: utilidades do Tailwind vencem os estilos globais", async ({ page }) => {
  await open(page, "/");

  const result = await page.evaluate(() => {
    const probe = document.createElement("div");
    probe.className = "pl-10 text-accent";
    document.body.append(probe);
    const reference = document.createElement("div");
    reference.style.color = "var(--accent)";
    document.body.append(reference);
    const computed = getComputedStyle(probe);
    const out = {
      paddingLeft: computed.paddingLeft,
      color: computed.color,
      accent: getComputedStyle(reference).color,
    };
    probe.remove();
    reference.remove();
    return out;
  });

  expect(result.paddingLeft).toBe("40px");
  expect(result.color).toBe(result.accent);
  expect(result.accent).not.toBe("");
});

test("T6: o acento vem do token e muda com o tema", async ({ page }) => {
  await open(page, "/");
  const accentOf = () =>
    page.evaluate(() => {
      const probe = document.createElement("div");
      probe.className = "text-accent";
      document.body.append(probe);
      const color = getComputedStyle(probe).color;
      probe.remove();
      return color;
    });

  const dark = await accentOf();
  expect(dark).toBe("rgb(245, 158, 75)"); // #f59e4b
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "light";
  });
  expect(await accentOf()).not.toBe(dark);
});

test("T6: o CSS global fica em @layer base (nenhuma regra solta em tags)", async ({ page }) => {
  await open(page, "/");
  const unlayered = await page.evaluate(() => {
    const offenders: string[] = [];
    const walk = (rules: CSSRuleList, inLayer: boolean) => {
      for (const rule of Array.from(rules)) {
        if (rule instanceof CSSLayerBlockRule) walk(rule.cssRules, true);
        else if (rule instanceof CSSStyleRule && !inLayer) {
          if (/^(html|body|\*|#root|h[1-6]|button|a|p|input)(\b|$)/.test(rule.selectorText)) {
            offenders.push(rule.selectorText);
          }
        } else if (rule instanceof CSSMediaRule) walk(rule.cssRules, inLayer);
      }
    };
    for (const sheet of Array.from(document.styleSheets)) {
      try {
        walk(sheet.cssRules, false);
      } catch {
        // folhas de outra origem não são legíveis
      }
    }
    return offenders;
  });
  expect(unlayered).toEqual([]);
});
