# F07 — Fluxo de download na UI (Início, barra de comando, Preview, Coleção, Atividade)

**Objetivo:** o usuário consegue, pela interface, colar um link ou pesquisar, ver o preview,
escolher perfil/opções, baixar faixas soltas e seleções de playlists, e acompanhar/gerenciar
a fila. Também é criada a infraestrutura de **E2E no app real**.

**Pré-requisito:** F06. **Design:** §3.1–3.4, §4, §5. **Arquitetura:** §7, §10, §14, §15.

## Tarefas

1. **Comandos Tauri** novos: `url_classify`, `analyze`, `search`, `pick_folder` (via
   `tauri-plugin-dialog` no Rust, assíncrono), `open_output_dir`, `clipboard_read_text`,
   `library_reveal` (nesta fase recebe um caminho de arquivo — abre a pasta do `output_path` com
   `tauri-plugin-opener` `reveal_item_in_dir`; na F10 passa a aceitar também id da biblioteca).
   Conferir que os comandos de fila da F04 estão todos registrados (o `check:ipc` ajuda).
2. **Barra de comando** (`components/command-bar/`): usada embutida no Início e no overlay
   Ctrl+K. Classifica a entrada em tempo real (espelho TS de `url_kind` só para dica visual; a
   decisão final é do backend `url_classify`). Enter: vídeo ⇒ Preview; coleção ⇒ Coleção;
   texto ⇒ resultados (abas YouTube Music | YouTube) com miniatura, título, canal/duração,
   botão "Baixar" direto (perfil padrão) e clique ⇒ Preview. Estados: carregando (skeleton),
   erro traduzido, vazio. "Colar e baixar" (ação rápida) lê a área de transferência via comando
   Rust `clipboard_read_text` (adicione-o; plugin `clipboard-manager`).
3. **Preview** (Sheet): conforme design §3.2, exceto itens marcados F08/F13 (versão oficial,
   pré-visualizar metadados, capítulos — deixar os espaços previstos ocultos). Selo de qualidade da
   fonte (`best_audio_abr` + codec). Chips de perfil com aviso de recodificação. Opções por job
   (`JobOptions`). Pasta de destino. Duplicata ⇒ diálogo (paridade) "Baixar novamente?".
   Botões **Baixar agora** (`priority: true`) e **Adicionar à fila**. Toast "Adicionado à fila".
4. **Coleção** (rota modal/página): design §3.3 sem o botão de sincronizar (F11). Lista
   virtualizada; seleção; filtro; marcação de já baixadas (`check_duplicates`); aviso de limite
   da fila; enfileira com `kind = playlist_item` e `playlist_ctx`.
5. **Atividade** (design §3.4): abas, contadores, controles, itens com estágio/progresso/
   velocidade/ETA/tentativas/erro traduzido (`errors.<kind>`), retry, remover, cancelar,
   arrastar para reordenar (dnd-kit, só pendentes ⇒ `job_move`), abrir pasta (concluídos),
   banner de autocura. Contador da sidebar.
6. **Início**: ações rápidas, "Em andamento" (3), "Recentes" (provisório: últimos jobs
   concluídos; na F10 passa a vir da biblioteca).
7. Mock estendido para todos esses comandos (análise de FX1–FX4, busca FX7, simulação de fila).
8. **E2E no app real** (`tests/e2e-app/`): WebdriverIO + `tauri-driver`.
   - Instalação: `cargo install tauri-driver --locked`; `msedgedriver` compatível com a versão do
     WebView2 instalada (use `cargo install --git https://github.com/chippers/msedgedriver-tool`
     e rode-o para baixar o driver certo, ou baixe manualmente da Microsoft pela versão do
     WebView2 em `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` → `pv`).
   - Script `npm run e2e:app` ⇒ `npx tauri build --debug --no-bundle` (**não** use `cargo build`
     puro: em debug sem a feature `custom-protocol` o app carrega o `devUrl` `localhost:1420` em
     vez do frontend embutido; o `tauri build --debug` embute o `dist/` e mantém
     `debug_assertions`) ⇒ inicia `tauri-driver` ⇒ roda specs com diretório de dados temporário
     (`REVERB_DATA_DIR`, honrado só em debug) e `REVERB_TOOLS_DIR=.test-tools` para não baixar
     ferramentas de novo, com `outputDir` temporário definido antes de abrir o app via
     `reverb-cli --data-dir <dir> settings set outputDir "\"<pasta-temp>\""` (valor em JSON).
     Também gravar `onboardingCompleted = true` (a partir da F12) para o app abrir direto no Início.
   - Se, após 5 ciclos de correção, o `tauri-driver` não funcionar nesta máquina: **parar e
     perguntar ao usuário** se aceita substituir os testes "app real" por um roteiro manual
     (lista de passos que ele executa e confirma). Registrar a decisão.

## Testes de verificação

| # | Teste | Ferramenta |
|---|-------|-----------|
| T1 | Store `jobs` aplica `job://updated`/`job://removed`/`queue://state` corretamente (ordem, contadores) | Vitest |
| T2 | Barra de comando: URL de vídeo ⇒ chama `analyze` e abre Preview; coleção ⇒ Coleção; texto ⇒ `search` e abas; Esc limpa; Enter envia | Vitest |
| T3 | Preview: renderiza dados de FX1/FX2 do mock, selo "Fonte: Opus 129 kbps", aviso ao escolher `mp3_320`, diálogo de duplicata, "Baixar agora" envia `priority: true` | Vitest |
| T4 | Coleção: selecionar 3 de 10, "Selecionar só as novas", filtro, botão mostra "(3)", enfileira 3 com `playlist_ctx.index` corretos | Vitest |
| T5 | Atividade: contadores, retry em falho, cancelar, remover, mensagem de erro traduzida para cada `ErrorKind`, banner de autocura | Vitest |
| T6 | E2E mock: fluxo colar URL ⇒ Preview ⇒ Baixar ⇒ progresso por estágios ⇒ aparece em Concluídos ⇒ abrir pasta chama `library_reveal` | Playwright |
| T7 | E2E mock: playlist FX4 ⇒ selecionar 3 ⇒ 3 jobs na Atividade; reordenar arrastando; Espaço pausa/retoma; Ctrl+K abre overlay | Playwright |
| T8 | E2E mock: busca ⇒ resultado ⇒ "Baixar" direto cria job | Playwright |
| T9 | Acessibilidade (axe) nas telas novas; screenshots atualizadas e inspecionadas | Playwright |
| T10 | **App real** (rede): abrir app ⇒ colar FX1 na barra ⇒ Preview mostra "Me at the zoo" ⇒ Adicionar à fila ⇒ aguardar status Concluído (≤ 120 s) ⇒ arquivo `.opus` existe na pasta configurada | WebdriverIO |
| T11 | **App real**: cancelar um download em andamento (com limite de velocidade baixo) muda para Cancelado | WebdriverIO |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` (T10–T11) ✔

## Armadilhas
- `REVERB_DATA_DIR`/`REVERB_TOOLS_DIR` só podem ser honrados em builds de debug
  (`cfg!(debug_assertions)`), nunca em release.
- O WebView2 do sistema atualiza sozinho; o `msedgedriver` precisa ter a mesma versão principal.
- Listas virtualizadas: em testes, garanta altura do contêiner (jsdom não calcula layout — use
  `@tanstack/react-virtual` com `initialRect` nos testes ou teste a lógica separadamente).
