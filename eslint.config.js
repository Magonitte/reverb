import js from "@eslint/js";
import globals from "globals";
import i18next from "eslint-plugin-i18next";
import reactHooks from "eslint-plugin-react-hooks";
import tseslint from "typescript-eslint";

const i18nRule = {
  "i18next/no-literal-string": [
    "error",
    {
      mode: "jsx-only",
      "jsx-attributes": { include: ["aria-label", "title", "placeholder", "alt"] },
    },
  ],
};

export default tseslint.config(
  {
    ignores: [
      "plano/**",
      "referencias/**",
      "dist/**",
      "target/**",
      "src/bindings/**",
      "src-tauri/**",
      "node_modules/**",
      "test-results/**",
      "playwright-report/**",
      ".test-tools/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: { globals: { ...globals.browser } },
    plugins: { "react-hooks": reactHooks, i18next },
    rules: { ...reactHooks.configs.recommended.rules, ...i18nRule },
  },
  {
    files: ["**/*.{js,mjs}", "scripts/**", "tests/**", "*.config.ts"],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/lib/ipc/mock/**", "tests/**", "scripts/**"],
    rules: { "i18next/no-literal-string": "off" },
  },
);
