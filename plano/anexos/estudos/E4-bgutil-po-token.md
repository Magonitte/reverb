# E4 — Provedor de PO token (bgutil) como ferramenta externa

**Decisão:** usar o projeto `Brainicism/bgutil-ytdlp-pot-provider` **como programa externo
gerenciado** (igual ao yt-dlp), **não** reimplementar. Ele acompanha as mudanças do YouTube;
reimplementar transferiria essa manutenção para nós. Rodando como processo separado, sua
licença (GPL-3.0) não se estende ao Reverb.

## O que resolve
O YouTube exige "PO tokens" (prova de origem) em alguns clientes. Sem eles: `HTTP Error 403`,
formatos ausentes ou "Sign in to confirm you're not a bot", principalmente em IPs marcados ou em
uso intenso (playlists grandes). Premium não precisa do token de streaming.

## Peças (validado 2026-10-01, release `2.0.0`)
1. **Plugin do yt-dlp**: asset `bgutil-ytdlp-pot-provider.zip` do release. O zip é usado **como
   está** (não extrair) dentro de uma pasta passada com `--plugin-dirs <pasta>`. O yt-dlp
   standalone (2026.08.19) carregou o plugin assim (log: `Plugin directories: …zip\yt_dlp_plugins`).
2. **Provedor** (código do servidor, pasta `server/` do repositório na mesma tag):
   - obter o código da tag pelo `zipball_url` do release (API do GitHub) — sem precisar de git;
   - instalar dependências com o **Deno gerenciado**: na pasta `server/`,
     `deno install --allow-scripts=npm:canvas --frozen` (levou ~15 s; baixa binário nativo do
     `canvas`) — deixar um aviso de depreciação de `prebuild-install` ser ignorado;
   - **modo servidor HTTP** (recomendado): dentro de `server/node_modules`, executar
     `deno run --allow-env --allow-net --allow-ffi=. --allow-read=. ../src/main.ts --port <porta>`;
     liga só em `127.0.0.1`/`::1`; `GET http://127.0.0.1:<porta>/ping` ⇒
     `{"server_uptime":…, "version":"2.0.0"}`.
   - yt-dlp com `--plugin-dirs <pasta-do-zip> --extractor-args "youtubepot-bgutilhttp:base_url=http://127.0.0.1:<porta>"`.
3. **Medições** (vídeo de ~3,5 min, simulate):
   - sem provedor (cliente padrão): **2,4 s**;
   - modo **script** (`youtubepot-bgutilscript:server_home=<server>`; inicia o Deno a cada
     chamada): **13,1 s** — e grava cache em `%USERPROFILE%\.cache\bgutil-ytdlp-pot-provider`
     (fora da pasta do app) ⇒ **não usar** o modo script;
   - modo **HTTP**: **3,4–4,4 s** (o servidor reaproveita o estado).
   - Prova de funcionamento com cliente que exige token (`youtube:player_client=mweb`):
     log `Retrieved a gvs PO Token for mweb client` e download do formato 251.

## Comportamento a implementar
- Nova ferramenta gerenciada `bgutil` (F02): versão = tag do release; instala plugin + código
  do servidor + dependências (Deno); atualização junto com as demais ferramentas; teste de
  fumaça = subir o servidor numa porta livre, `GET /ping` responder com a versão, derrubar.
- Nova configuração `potProvider`: `"auto"` (padrão) | `"always"` | `"off"`.
  - `auto`: desligado no uso normal; **ligado pela autocura** (F04) quando ocorrer `bot_check`
    ou `HTTP Error 403`/`extractor` relacionado a formatos — e permanece ligado por 24 h
    (registrar em `kv`), depois volta a desligado.
  - `always`: sempre ligado (mais lento, mais robusto).
  - `off`: nunca.
- **Ciclo de vida do servidor** (`PotServer` no core): iniciado sob demanda numa porta livre
  aleatória em `127.0.0.1`; aguarda `/ping` (timeout 30 s); reutilizado por todos os jobs;
  encerrado após 10 min sem uso ou ao fechar o app (matar a árvore, como as outras ferramentas);
  se morrer, reiniciar uma vez; se falhar de novo ⇒ seguir sem token e avisar no diagnóstico.
- Quando ligado, os argumentos do yt-dlp ganham `--plugin-dirs` e o `--extractor-args` do
  `base_url`. **Não** forçar `player_client` — deixar o yt-dlp escolher (o token passa a estar
  disponível para os clientes que precisam).
- Diagnóstico (F12) mostra: versão do bgutil, estado do servidor, último uso.

## Testes de aceitação
- Unit: montagem de argumentos com/sem provedor; política `auto` (liga após `bot_check`, expira
  em 24 h com relógio injetado); ciclo de vida com um servidor HTTP falso (wiremock) respondendo
  `/ping`, e com processo que morre (reinício único).
- Rede (Windows): instalar `bgutil` real em diretório temporário; subir servidor; `-v --simulate`
  com `--extractor-args youtube:player_client=mweb` em FX2 ⇒ stderr contém
  `Retrieved a gvs PO Token` e código 0; derrubar ⇒ nenhum `deno` órfão (verificar com `sysinfo`).
