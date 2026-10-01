# 00 — Protocolo de Execução

Estas regras valem para **todas** as fases. Em caso de conflito entre uma fase e este
protocolo, **este protocolo vence**.

## 1. Ambiente conhecido (verificado em 2026-10-01)

| Item | Situação na máquina do usuário |
|------|-------------------------------|
| SO | Windows 11 Pro (usuário com espaço no nome: `C:\Users\Jean Carlos de Souza`) |
| Node.js / npm | v24.14.0 / 11.12.0 |
| Rust | 1.95.0 stable, alvo `x86_64-pc-windows-msvc` |
| Git | 2.54 |
| GitHub CLI | 2.101, autenticado na conta **Magonitte** |
| WSL | só `docker-desktop` (sem Ubuntu) → **Linux é validado pelo CI do GitHub Actions** |
| Android SDK / JDK 17 | **ausentes** (JDK instalado é 1.8) → necessários só na parte Android |
| Node no PATH | sim (útil como runtime JS alternativo do yt-dlp) |
| Deno no PATH | não |

Shells disponíveis: PowerShell 5.1 e Git Bash. Caminhos contêm espaços: **sempre** use aspas e,
em código, **sempre** passe argumentos de processo como lista (nunca concatene string de shell).

## 2. Ciclo de trabalho de cada fase

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Ler a fase inteira + seções referenciadas da arquitetura     │
│ 2. Registrar no PROGRESS.md: "Fase X — EM ANDAMENTO"            │
│ 3. Implementar as tarefas na ordem                              │
│ 4. Escrever os testes da seção "Testes de verificação"          │
│ 5. Rodar o PORTÃO da fase (comandos exatos da fase)             │
│ 6. Falhou? → CICLO DE CORREÇÃO (§3) → voltar ao passo 5         │
│ 7. Passou? → checklist de conclusão (§5) → commit + tag         │
│ 8. Registrar "Fase X — CONCLUÍDA" e só então ir para a próxima  │
└─────────────────────────────────────────────────────────────────┘
```

**Nunca avance de fase com qualquer teste do portão falhando.** Isso inclui testes de fases
anteriores: `npm run verify` e `npm run e2e` sempre rodam a suíte offline **inteira**; e quando
o portão inclui `npm run verify:net` ou `npm run e2e:app`, eles rodam **todos** os testes de
rede / de app real existentes (de todas as fases), não só os da fase atual.

## 3. Ciclo de correção (quando um teste falha)

1. Leia a saída **completa** do erro (não só a última linha).
2. Identifique a **causa raiz**. Escreva 1–3 linhas sobre ela no `PROGRESS.md` (seção da fase,
   subseção "Falhas e correções").
3. Corrija o **código de produção**. Só altere um teste se ele contradizer o plano — e então
   registre a justificativa citando a seção do plano.
4. Rode **o portão inteiro de novo**, não só o teste que falhou.
5. Repita.

### Proibido (invalida a fase)

- Pular, desabilitar ou marcar como `skip`/`only`/`#[ignore]` um teste para fazê-lo "passar".
  Exceção única: testes de rede usam `#[ignore = "network"]` / `test.describe.configure` com tag
  `@network` **por desenho**, e são obrigatoriamente executados nos portões que os listam.
- Afrouxar asserções, limites ou tolerâncias definidos no plano.
- Substituir por mock a própria coisa que um teste de integração deve exercitar.
- Usar `--no-verify`, apagar testes, comentar asserções, ou `unwrap_or_default` para esconder erro.
- Deixar `todo!()`, `unimplemented!()`, `// TODO` ou `throw new Error("not implemented")`
  dentro do escopo da fase concluída.
- Marcar a fase como concluída "com ressalvas".

### Snapshots e screenshots de base

Atualizar snapshots (`cargo insta accept`) ou screenshots de base do Playwright
(`--update-snapshots`) **só** é permitido quando a fase atual mudou **intencionalmente** aquela
saída/tela. Antes de aceitar: abrir e inspecionar cada diff/imagem nova, confirmar que a mudança
é a esperada pela tarefa e registrar no PROGRESS.md quais bases mudaram e por quê. Atualizar
bases para esconder uma regressão é proibido (equivale a afrouxar asserção).

### Limite de tentativas e bloqueio

Se **a mesma falha** persistir após **5 ciclos completos** de correção, ou se a falha depender
de algo externo (sem internet, YouTube bloqueando, ação do usuário necessária, serviço fora do ar):

1. Marque a fase como `BLOQUEADA` no `PROGRESS.md` com: teste, saída do erro, o que já foi
   tentado, hipótese da causa.
2. **Pare e pergunte ao usuário.** Não avance, não contorne.

Falhas de rede intermitentes: um teste `@network` pode ser re-executado **até 3 vezes** antes de
contar como falha real. Registre as re-execuções.

## 4. Comandos-padrão de verificação

Criados na F00 e usados por todas as fases:

| Comando | O que roda | Quando |
|---------|-----------|--------|
| `npm run verify` | typecheck TS → ESLint → checagem i18n → Vitest → `cargo fmt --check` → `cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace` → checagem de bindings TS atualizados → varredura de segredos | **Todo portão** |
| `npm run e2e` | Playwright contra o frontend com backend mock (`VITE_REVERB_MOCK=1`) | Portões a partir da F00 |
| `npm run verify:net` | Testes Rust `--ignored` com `REVERB_NET_TESTS=1` + Playwright `@network` (se houver) | Portões que listam "rede" |
| `npm run test:prepare` | Instala ferramentas reais (yt-dlp, deno, ffmpeg) em `.test-tools/` para os testes | A partir da F02, antes de testes que usam ffmpeg/yt-dlp |
| `npm run e2e:app` | E2E no app Tauri real (tauri-driver) — ver F07 | Portões que listam "app real" |

O portão de uma fase = `npm run verify` + `npm run e2e` + os testes específicos listados na fase
(incluindo os de rede, se a fase pedir). Todos devem passar **na mesma rodada**.

## 5. Checklist de conclusão de fase

Antes de marcar `CONCLUÍDA`:

- [ ] Todas as tarefas da fase implementadas.
- [ ] Todos os testes da seção "Testes de verificação" existem e passam.
- [ ] Portão completo passou numa única rodada (cole o resumo no PROGRESS.md: nº de testes
      Rust, Vitest, Playwright, rede, e o tempo).
- [ ] `git grep -nE "todo!\(|unimplemented!\(|TODO|FIXME|not implemented"` sem ocorrências no
      escopo da fase (ocorrências em fases futuras explicitamente previstas são permitidas só se
      estiverem em `// F<nn>:` apontando a fase).
- [ ] Nenhum texto de UI fora dos arquivos de tradução (lint garante).
- [ ] Arquivos de tradução pt-BR e en com o mesmo conjunto de chaves (script garante).
- [ ] `PROGRESS.md` atualizado (decisões, desvios do plano e motivos).
- [ ] Commit `feat(FNN): <resumo>` e tag `fase-NN-ok`.
- [ ] Se a fase tiver "Pontos de parada humana" pendentes, eles foram resolvidos.

## 6. Pontos de parada humana

Algumas ações são externas, irreversíveis ou exigem o usuário. Nestes pontos **pare e peça
confirmação explícita** (não presuma a resposta):

- Criar repositório no GitHub, torná-lo público, fazer `git push` pela primeira vez (F06).
- Gerar/guardar a chave de assinatura do atualizador e cadastrar segredos no GitHub (F06).
- Publicar qualquer release (tags `v*`) (F06, F16, A3).
- Instalar aplicativos/SDKs no sistema do usuário (Android Studio, JDK, navegadores, etc.) ou
  instalar builds do próprio Reverb (instaladores NSIS de teste).
  **Não** precisam de confirmação as ferramentas de desenvolvimento instaladas por gerenciadores
  de pacote do projeto: dependências npm/cargo, navegadores do Playwright
  (`npx playwright install`), `cargo install` de `tauri-driver`, `msedgedriver-tool` e
  `cargo-audit`, e as ferramentas em `.test-tools/`.
- Qualquer verificação que a fase classifique como "checagem manual do usuário".
- Apagar qualquer arquivo fora de `Reverb_claude/` e das pastas temporárias de teste.

Ao pedir: diga exatamente o que será feito, por quê, e o que o usuário precisa fazer.

## 7. Regras de engenharia permanentes

1. **Versões:** Tauri **2.x** (NUNCA 3.x alpha — o crates.io já lista `3.0.0-alpha` como
   "latest"). Em `Cargo.toml` use `tauri = { version = "2", ... }` e plugins `"2"`; em npm,
   `@tauri-apps/*@^2`. Nunca use versões pre-release (`-alpha`, `-beta`, `-rc`, `-pre`) de
   nenhuma dependência. Use `cargo add <crate>` (escolhe a última estável) e `npm install`.
   Se uma API divergir do que o plano descreve, consulte a documentação **da versão instalada**
   (docs.rs / site oficial) e adapte; registre no PROGRESS.md.
2. **TypeScript:** use a versão estável mais nova **compatível** com `typescript-eslint` e com o
   plugin do Vite. Se TypeScript 7 causar conflito de peer dependency, fixe na maior versão
   compatível (6.x ou 5.9.x) e registre.
3. **Toda interação com o SO** (arquivos, processos, diálogos, abrir pasta, área de transferência,
   notificações, atualizador, autostart) acontece **em Rust**, exposta por comandos próprios.
   O frontend só usa `invoke`, `listen` e os controles de janela. Isso mantém as *capabilities*
   do Tauri mínimas (ver arquitetura §15).
4. **Processos externos:** argumentos sempre como lista; no Windows sempre com
   `CREATE_NO_WINDOW` (0x08000000); sempre em grupo/Job Object para matar a árvore inteira.
5. **Nunca** gravar segredos (chaves de API, cookies, chave de assinatura) em arquivos versionados
   nem em logs. A varredura de segredos do `verify` falha o portão.
6. **Textos de UI** só via i18n (`t("chave")`), pt-BR é o idioma padrão, en obrigatório.
7. **CSS:** estilos globais dentro de `@layer base` (lição do projeto antigo: CSS fora de layer
   vence as utilidades do Tailwind e "quebra" classes). Há teste específico na F05.
8. **Código antigo** em `D:\Documentos\4 - Pessoal\Reverb` é **somente leitura**.
8b. **Sala limpa:** funcionalidades inspiradas em outros projetos (estudos E1–E7 em
    `plano/anexos/estudos/`) são implementadas **somente** a partir dessas especificações.
    É **proibido** abrir, clonar, baixar ou consultar o código-fonte desses projetos (inclusive a
    pasta `G:\Estudo_Repo`), e copiar trechos de código de qualquer projeto de terceiros.
    Ferramentas externas (yt-dlp, Deno, FFmpeg, fpcalc, bgutil) são **usadas como programas**,
    baixadas pelo gerenciador de ferramentas — isso é permitido.
9. Commits pequenos e frequentes dentro da fase são bem-vindos; a tag só no fim.
10. Ao terminar cada sessão de trabalho (mesmo sem concluir a fase), atualize o PROGRESS.md com
    o ponto exato onde parou.
11. **Testes e experimentos nunca escrevem nas pastas reais do usuário** (Músicas, AppData do
    Reverb instalado, Downloads). Use diretórios temporários, `--data-dir`/`--tools-dir` no CLI e
    `REVERB_DATA_DIR`/`REVERB_TOOLS_DIR` no app de debug, sempre com `outputDir` temporário.
12. Mantenha o backend mock (`src/lib/ipc/mock/`) sempre em dia: todo comando/evento novo no
    Rust ganha implementação no mock na mesma fase (o `check:ipc` falha se faltar).

## 8. Formato do PROGRESS.md

Use o modelo já existente em `PROGRESS.md`. Uma seção por fase, com: status, datas, tarefas
concluídas, resultado do portão, falhas e correções, desvios do plano, pendências humanas.
