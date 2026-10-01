# F00 — Fundação do monorepo

**Objetivo:** estrutura do projeto criada, tudo compila, ferramentas de qualidade e o comando
`npm run verify` funcionando, app Tauri abre e tem modo headless de autoteste.

**Pré-requisitos:** nenhum. **Arquitetura:** §2, §3, §4, §17 (flags headless), §19.

## Tarefas

1. **Pré-checagem do ambiente** — registrar no PROGRESS.md as saídas de: `node -v`, `npm -v`,
   `rustc -V`, `cargo -V`, `git --version`, `gh --version`. Confirmar que o "Microsoft C++
   Build Tools"/MSVC existe (`cargo build` de um projeto vazio linka) e que o WebView2 está
   instalado (padrão no Windows 11).
2. **Git**: `git init`, branch `main`. `.gitignore` com: `node_modules/`, `dist/`, `target/`,
   `.test-tools/`, `test-results/`, `playwright-report/`, `*.log`, `.env*`, `*.key`,
   `*.key.pub.bak`, `*.pem`, `cookies*.txt`, `api_config.json`, `/data/`, `src-tauri/gen/schemas/`
   (é regenerado), `.DS_Store`, `Thumbs.db`. **Não** ignore `src/bindings/` (é commitado).
   Commit inicial contendo `plano/`, `referencias/`, `LEIA-ME_PRIMEIRO.md`, `PROGRESS.md`.
3. **Frontend** (manual, sem templates interativos): `package.json` (`"name":"reverb"`,
   `"version":"0.1.0"`, `"private":true`, `"type":"module"`), instalar dependências de §2
   (React, react-dom, react-router, zustand, i18next, react-i18next, lucide-react, motion,
   @tanstack/react-virtual, @tauri-apps/api@^2) e de desenvolvimento (vite, @vitejs/plugin-react,
   typescript, @types/react, @types/react-dom, tailwindcss, @tailwindcss/vite, vitest, jsdom,
   @testing-library/react, @testing-library/user-event, @testing-library/jest-dom, eslint,
   @eslint/js, typescript-eslint, eslint-plugin-react-hooks, eslint-plugin-i18next, globals,
   prettier, @playwright/test, @axe-core/playwright, @tauri-apps/cli@^2).
   `vite.config.ts`: porta 1420 fixa (`strictPort: true`), `clearScreen: false`,
   `envPrefix: ["VITE_", "TAURI_ENV_"]`, alias `@` → `src`, plugin Tailwind.
   Script `dev:mock` = `vite --port 1421 --mode mock` com `.env.mock` contendo `VITE_REVERB_MOCK=1`
   (o `.env.mock` **é** versionado — exceção explícita no `.gitignore`: `!.env.mock`).
4. **Camada IPC** `src/lib/ipc/`: `isTauri()` (verifica `window.__TAURI_INTERNALS__`),
   `api.ts` com `call<T>(cmd, args)` que usa `invoke` no Tauri ou o backend mock quando
   `import.meta.env.VITE_REVERB_MOCK === "1"` **ou** não estiver no Tauri; `mock/index.ts`
   com o comando `app_info` respondendo dados falsos plausíveis. Em testes, usar `mockIPC`.
5. **App mínimo**: `App.tsx` mostra o nome do app (via i18n) e `version`/`platform` vindos de
   `app_info`. i18n inicial com pt-BR e en (chaves `app.name`, `app.loading`).
6. **Cargo workspace**: `Cargo.toml` raiz (`[workspace] members = ["crates/*", "src-tauri"]`,
   `resolver = "2"`, `[workspace.package] edition = "2021"`, `[workspace.dependencies]` com as
   versões compartilhadas). `.cargo/config.toml` com `TS_RS_EXPORT_DIR` (§3).
7. **`crates/reverb-core`** (lib): módulos `paths` (resolução de diretórios §4, recebendo
   `exe_dir` e `app_data_dir` como parâmetros — testável), `error` (`CoreError` com `thiserror`,
   serializável como `{kind, message, i18nKey?}`), `events` (`trait EventSink: Send + Sync
   { fn emit(&self, event: &str, payload: serde_json::Value); }` + `MemorySink` para testes).
   `AppInfo` com `#[derive(Serialize, TS)]`.
8. **`crates/reverb-cli`** (bin `reverb-cli`, clap): `--version`, `--data-dir <p>`,
   subcomando `doctor` que imprime JSON (`{"version","os","arch","dataDir","portable"}`).
9. **`crates/fake-tool`** (bin `fake-tool`): imprime `FAKE_TOOL_VERSION` (env) ou `1.0.0` com
   `--version`; `--sleep <s>` dorme; `--spawn-child-sleep <s>` cria um filho de si mesmo
   dormindo, imprime `CHILD_PID=<pid>` no stdout e também dorme (para testar morte de árvore); `--stdout-lines <arquivo>` ecoa um arquivo linha a
   linha com 50 ms de intervalo (simula yt-dlp); `--exit <code>` sai com código; `--stderr <texto>`;
   `--http-ping <porta>` sobe um servidor HTTP mínimo em `127.0.0.1:<porta>` que responde
   `GET /ping` com `{"version":"fake"}` (simula o servidor de PO token — usado na F02).
10. **`src-tauri`**: `tauri init` não interativo ou arquivos escritos à mão, garantindo
    `tauri = { version = "2", features = ["tray-icon", "image-png"] }` e `tauri-build = "2"`.
    `tauri.conf.json`: `productName "Reverb"`, `identifier "com.reverb.desktop"`,
    `"version": "../package.json"`, `build.devUrl "http://localhost:1420"`,
    `beforeDevCommand "npm run dev"`, `beforeBuildCommand "npm run build"`,
    `frontendDist "../dist"`, `app.windows: []` (janela criada em `setup()`),
    `app.security.csp` restritiva (`default-src 'self'; img-src 'self' data: https:;
    style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost`) e
    `app.security.devCsp` igual, acrescentando `http://localhost:1420 ws://localhost:1420` em
    `connect-src` e `default-src` (HMR do Vite em `tauri dev`),
    ícones gerados com `npx tauri icon referencias/icones/icon.png` (o PNG tem 512×512; se o
    comando recusar ou avisar, gere um 1024×1024 ampliando-o com ffmpeg/`image` e registre — e
    avise o usuário no fim da fase que um ícone-fonte de 1024 px melhoraria a nitidez).
    `main.rs`: antes de construir a janela, tratar flags headless:
    `--headless-selftest <arquivo>` ⇒ escreve JSON `{"ok":true,"version":…,"dataDir":…}` e sai
    com código 0 sem criar janela. Este modo roda **antes** do `tauri::Builder` e resolve o
    diretório de dados pelo `reverb-core::paths` usando `dirs::data_dir()/com.reverb.desktop`
    (é o mesmo caminho que o `app_data_dir` do Tauri usa no Windows e no Linux). No `setup()`,
    compare `app.path().app_data_dir()` com o valor do core: se divergirem, registre `error!`
    no log e use **sempre** o valor do core (fonte única); o autoteste headless inclui o campo
    `dataDir` para conferência.
    Flags que precisam do runtime Tauri (atualizador, F06) rodam dentro do `setup()` e chamam
    `app.exit(code)` sem criar janela. Senão, criar a janela principal em `setup()`
    (1200×800, mín. 900×600, `decorations: false`, título "Reverb").
    Comando `app_info`. Capabilities conforme §15 (só as permissões de janela listadas).
11. **Qualidade**: `eslint.config.js` (flat; typescript-eslint, react-hooks, **i18next/no-literal-string
    em modo `jsx-text-only`** com `jsx-attributes.include` = `aria-label`, `title`, `placeholder`,
    `alt` — para impedir textos fixos também nesses atributos), `.prettierrc`, `rustfmt.toml` (padrão).
12. **Scripts** (`package.json` + `scripts/*.mjs` em Node puro, multiplataforma):
    - `dev` = `vite`; `build` = `vite build`; `typecheck` = `tsc --noEmit`;
      `lint` = `eslint . --max-warnings 0`; `test` = `vitest run`;
    - `check:i18n` = `node scripts/check-i18n.mjs` (falha se pt-BR e en tiverem chaves diferentes
      ou valores vazios);
    - `scan:secrets` = `node scripts/scan-secrets.mjs` (varre arquivos versionados **e** novos não
      ignorados — `git ls-files --cached --others --exclude-standard` —
      por padrões: `-----BEGIN .*PRIVATE KEY-----`, `"access_token"\s*:\s*"[A-Za-z0-9_\-]{20,}`,
      `client_secret\s*[=:]\s*["'][^"']{8,}`, `TAURI_SIGNING_PRIVATE_KEY=`; ignora `plano/`);
    - `bindings` = `cargo test -p reverb-core export_bindings` (ts-rs gera em `src/bindings`);
    - `check:bindings` = copia `src/bindings` para uma pasta temporária, roda `bindings`, e falha
      se o conteúdo regenerado diferir da cópia (bindings desatualizados) — **não** depende do
      git, para funcionar antes do commit;
    - `verify` = `node scripts/verify.mjs` executando em sequência, parando no primeiro erro e
      imprimindo um resumo: typecheck, lint, check:i18n, test, **`npm run build`** (gera `dist/`
      — obrigatório **antes** de qualquer passo cargo, porque o `tauri::generate_context!` do
      crate `src-tauri` exige `frontendDist` existente em tempo de compilação),
      `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, check:bindings, scan:secrets (e, a partir da F05, `check:ipc`);
    - `e2e` = `playwright test --grep-invert @network`; `verify:net` = `node scripts/verify-net.mjs`
      (define `REVERB_NET_TESTS=1`, roda `cargo test --workspace -- --ignored` e
      `playwright test --grep @network` se existirem testes);
    - `test:prepare` = criado na F02 (nesta fase pode imprimir "nada a preparar").
13. **Vitest**: `vitest.config.ts` com jsdom, `setupFiles` (jest-dom + i18n de teste + limpeza do `mockIPC`).
14. **Playwright**: `playwright.config.ts` com `webServer: npm run dev:mock` (porta 1421),
    projetos `desktop-1366` (1366×768). Pasta `tests/e2e`.
15. **CI** `.github/workflows/ci.yml` (fica pronto agora; só roda quando houver remoto — F06):
    jobs `windows-latest` e `ubuntu-22.04`; Linux instala
    `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf`;
    `actions/setup-node` (Node 22+, cache npm), `dtolnay/rust-toolchain@stable`,
    `Swatinem/rust-cache`, `npm ci`, `npx playwright install --with-deps chromium` (Linux) /
    `npx playwright install chromium` (Windows), `npm run test:prepare`, `npm run verify`,
    `npm run e2e` (só Windows). Definir `env: GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` no job
    (o `test:prepare` consulta a API de releases do GitHub e o limite anônimo é baixo).

## Testes de verificação

| # | Teste | Como |
|---|-------|------|
| T1 | `paths` resolve modo normal e portátil | `cargo test -p reverb-core paths` (usar `tempfile`: com e sem `portable.txt`) |
| T2 | `CoreError` serializa no formato `{kind,message,i18nKey}` | teste unitário |
| T3 | `MemorySink` registra eventos | teste unitário |
| T4 | `fake-tool` funciona: `--version`, `--exit 3`, `--stderr` | teste de integração em `crates/fake-tool/tests` usando `env!("CARGO_BIN_EXE_fake-tool")` |
| T5 | CLI `doctor` imprime JSON válido com as chaves esperadas | `crates/reverb-cli/tests/doctor.rs` (`CARGO_BIN_EXE_reverb-cli`) |
| T6 | App React renderiza nome e versão vindos do `app_info` mockado | Vitest + `mockIPC` |
| T7 | Lint pega texto literal em JSX e em `aria-label` | teste Vitest (`scripts/__tests__/lint-i18n.test.ts`) que usa a API `ESLint` do pacote `eslint` com a config do projeto (`lintText`) sobre `<div>Olá</div>` e `<button aria-label="Fechar"/>` ⇒ espera erros da regra `i18next/no-literal-string`; e sobre `<div>{t("x")}</div>` ⇒ zero erros |
| T8 | `check-i18n` falha com chave faltando | Vitest testando a função exportada do script com objetos de exemplo |
| T9 | `scan-secrets` detecta um segredo falso | Vitest com conteúdo de exemplo em memória |
| T10 | E2E: página carrega, mostra "Reverb", sem erros no console | `tests/e2e/smoke.spec.ts` (falhar se `page.on('console')` registrar `error`) |
| T11 | App Tauri compila | `cargo build -p reverb` (nome do pacote em `src-tauri/Cargo.toml` = `reverb`); conferir também que, apagando `dist/`, o `npm run verify` recria e continua passando |
| T12 | Autoteste headless do executável real | `target/debug/reverb.exe --headless-selftest "<tmp>/reverb-selftest.json"` ⇒ código 0 e JSON com `ok: true` (automatizar em `scripts/verify-app-headless.mjs`, multiplataforma: `reverb.exe` no Windows, `reverb` no Linux, diretório temporário do SO) |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `cargo build -p reverb` ✔ · `node scripts/verify-app-headless.mjs` ✔

## Armadilhas
- `npx tauri init` pode gerar `tauri.conf.json` com janela padrão — remova (`app.windows: []`).
- No Windows, o binário de release com `windows_subsystem = "windows"` não tem console: o modo
  headless **escreve em arquivo**, não no stdout.
- `cargo add tauri` sem versão pode pegar alpha em alguns fluxos: sempre `cargo add tauri@2`.
- `cargo build -p reverb` em debug gera um exe que carrega o `devUrl` (servidor do Vite) em vez
  do frontend embutido — serve para o autoteste headless (não usa a WebView), mas para abrir o
  app de verdade sem `tauri dev` use `npx tauri build --debug --no-bundle`.
- O `eslint-plugin-i18next` deve ignorar arquivos de teste e `src/lib/ipc/mock/`.
- `referencias/` contém um `.ts` antigo e HTML de referência: excluir `plano/`, `referencias/`,
  `dist/`, `target/`, `src/bindings/` (gerado) do ESLint (`ignores`) e do Prettier; o
  `tsconfig.json` deve ter `include` explícito (`src`, `tests`, `scripts`, arquivos de config) —
  nunca `**/*`.
