# Desenvolvimento, testes e publicação

## Pré-requisitos

- Git e Node.js 22.12+ ou 24, com npm. O CI usa Node 22; o ambiente Windows registrado usa Node 24.14.0.
- Rust stable, com `rustfmt` e `clippy`; a versão registrada no desenvolvimento é 1.95.0.
- Windows: compilador MSVC/Build Tools do Visual Studio com desenvolvimento C++ e Microsoft WebView2.
- Linux: dependências Tauri instaladas conforme o comando abaixo, usado pelo CI Ubuntu 22.04.

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Não há sidecar Python nesta reconstrução. Veja [CI](../.github/workflows/ci.yml), [Cargo.toml](../Cargo.toml) e [package.json](../package.json) para as definições efetivas.

## Preparar e executar

```sh
git clone https://github.com/Magonitte/reverb.git
cd reverb
npm ci
rustup component add rustfmt clippy
npm run test:prepare
```

`test:prepare` baixa ferramentas reais em `.test-tools/`. Internet é necessária para instalar dependências/ferramentas e para recursos de catálogo/download. Preserve `package-lock.json` e `Cargo.lock`; use `npm ci` para reproduzir a árvore registrada.

| Comando                     | Finalidade                                                   |
| --------------------------- | ------------------------------------------------------------ |
| `npm run dev:mock`          | Interface simulada em localhost:1421                         |
| `npm run tauri -- dev`      | App nativo com frontend de desenvolvimento em localhost:1420 |
| `npm run build`             | Frontend de produção em `dist/`                              |
| `npm run tauri -- build`    | Aplicativo/pacotes em `target/release/bundle/`               |
| `cargo build -p reverb-cli` | CLI em `target/debug/`                                       |

`npm run dev` sozinho inicia o frontend; para IPC real use o app Tauri. O mock usa `.env.mock`, handlers em `src/lib/ipc/mock/` e cenários de teste. Ele não comprova acesso à rede ou execução nativa.

### Dados isolados

Por padrão, o app de desenvolvimento e a CLI podem usar o diretório normal de dados. Para experiências, isole o banco e a saída. O app debug reconhece `REVERB_DATA_DIR` e `REVERB_TOOLS_DIR`; esses overrides não são aceitos pelo executável de produção.

Exemplo PowerShell, a partir da raiz:

```powershell
$env:REVERB_DATA_DIR = Join-Path $env:TEMP 'reverb-dev-data'
$env:REVERB_TOOLS_DIR = Join-Path (Get-Location) '.test-tools'
npm run tauri -- dev
```

## Validar uma entrega

Prepare Chromium uma vez com `npx playwright install chromium`. No Windows, instale também `tauri-driver` com `cargo install tauri-driver --locked` para o teste nativo. O script baixa o msedgedriver correspondente ao WebView2.

Execute os quatro portões, com fontes estáveis:

```sh
npm run verify
npm run e2e
npm run verify:net
npm run e2e:app
```

| Portão       | O que verifica                                                                             |
| ------------ | ------------------------------------------------------------------------------------------ |
| `verify`     | TypeScript, ESLint, i18n, IPC, Vitest, frontend, Rust fmt/clippy/test, bindings e segredos |
| `e2e`        | Interface mock, acessibilidade e screenshots no Playwright                                 |
| `verify:net` | Testes Rust marcados como rede e eventuais cenários Playwright de rede                     |
| `e2e:app`    | Build, deep links e WebdriverIO no Tauri Windows com downloads reais e dados temporários   |

`e2e:app` atualmente exige Windows. O CI Linux compila e valida o código, mas isso não equivale à suíte nativa Windows. Não rode o navegador mock ao mesmo tempo que comandos que regeneram bindings ou fazem builds: recargas do Vite podem invalidar o cenário em andamento.

Para máquinas com pouca memória e para reduzir interferência entre consultas reais, esta sessão usa:

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:RUST_LOG = 'info'
$env:RUST_TEST_THREADS = '1' # especialmente para verify:net
```

No Linux, use `export NOME=valor` para as mesmas variáveis. `RUST_LOG=info` evita que uma configuração externa `warn` suprima os eventos esperados pelo teste de logs. Um `GITHUB_TOKEN` local opcional pode reduzir limites de API nos downloads de ferramentas; não o grave no repositório.

A quantidade e o resultado da última rodada ficam em [PROGRESS](../PROGRESS.md). Testes de credenciais opcionais devem registrar claramente N/A, sem serem apresentados como acesso real validado. Quando screenshots mudarem intencionalmente, inspecione imagem atual e diff antes de atualizar somente as bases afetadas; não reduza as asserções para obter um portão verde.

## Tipos, IPC e traduções

Os modelos Rust usam `ts-rs`. Após alterar um contrato, rode `npm run bindings` e versione os arquivos gerados em `src/bindings/`. Não os edite manualmente. `check:bindings` verifica consistência.

Novos comandos devem estar no Rust, no registro Tauri, nos wrappers da API e no mock. `npm run check:ipc` confere esses conjuntos. Traduções precisam existir em `src/locales/pt-BR/translation.json` e `src/locales/en/translation.json`; execute `npm run check:i18n`.

## CLI

A CLI é destinada a desenvolvimento/diagnóstico e não é prometida como executável separado nos instaladores. Consulte as opções efetivas com:

```sh
cargo run -p reverb-cli -- --help
cargo run -p reverb-cli -- --data-dir ./data-dev doctor
cargo run -p reverb-cli -- --data-dir ./data-dev settings get
cargo run -p reverb-cli -- --data-dir ./data-dev --tools-dir ./.test-tools analyze "https://www.youtube.com/watch?v=jNQXAC9IVRw" --json
```

Também há comandos `tools`, `search`, `download`, `jobs`, `queue` e `tags`. A CLI usa o mesmo core; sem `--data-dir`, pode operar no banco do aplicativo instalado. Não use o banco real para testes de alteração.

## Publicar uma versão

1. Atualize comportamento, documentação, CHANGELOG e PROGRESS.
2. Use `node scripts/bump-version.mjs <x.y.z>` e confira versões/locks.
3. Execute todos os portões relevantes nas fontes finais. Para uma fase completa, os quatro são obrigatórios.
4. Commit, envie a branch e abra PR com problema resolvido, comportamento final, validação e limitações.
5. Crie/envie a tag `v<x.y.z>` para disparar [Release](../.github/workflows/release.yml). A tag precisa corresponder ao `package.json`.
6. Espere os jobs Windows e Linux passarem; confira os instaladores, assinaturas e `latest.json` publicados.
7. Teste atualização da versão anterior e preservação dos dados antes de registrar a instalação como concluída.

O workflow usa os segredos de assinatura `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` no GitHub. Não versione chaves privadas. A chave pública e o endpoint do atualizador ficam em `src-tauri/tauri.conf.json`. O helper `scripts/release.mjs` exige árvore limpa e possui modo `--push`; ele não substitui o protocolo completo de uma fase, nem a conferência dos artefatos.

## Arquivos gerados e espaço

`target/` pode ocupar dezenas de GB por símbolos de depuração e variantes de build. Confira espaço antes de compilar. Artefatos gerados podem ser reconstruídos; `cargo clean` limpa o target e torna o próximo build mais lento. Não limpe pasta de dados, músicas ou fontes para liberar espaço. Relatórios, caches, `dist/`, dependências e `.test-tools/` ficam fora do Git.
