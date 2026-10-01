# F13 — Qualidade avançada: cookies/Premium, "Melhorar qualidade", capítulos, AcoustID, Spotify/Discogs

**Objetivo:** extrair o máximo de qualidade possível (256 kbps com Premium), atualizar faixas
antigas, dividir álbuns completos por capítulos, identificar pelo som e usar provedores com
chave opcionais.

**Pré-requisito:** F12. **Arquitetura:** §6, §8, §11, §12, §16 (fpcalc).

## Tarefas

1. **Cookies**: `cookiesSource`/`cookiesFile` ⇒ `--cookies-from-browser <nav>` ou
   `--cookies <arquivo>` em **todas** as chamadas do yt-dlp. `cookies_test()` ⇒ analisa FX2 com
   cookies ⇒ `{ ok, premium: bool, best_audio: "AAC 256" | …, error_kind? }`. Premium detectado
   se existir formato `141` ou `774` (ou `abr ≥ 200` com `vcodec none`). Erros específicos:
   navegador não instalado, banco de cookies bloqueado (Chrome aberto), falha de
   descriptografia (Chrome/Edge no Windows) ⇒ mensagens i18n recomendando Firefox.
   Arquivo de cookies nunca é copiado para logs/backup.
2. **Selo Premium/qualidade** no Preview e na Biblioteca (`source_abr_kbps`).
3. **Melhorar qualidade**: `upgrade_scan()` ⇒ itens `provider=youtube` com `source_abr_kbps < 200`
   ⇒ reanálise (com cookies) ⇒ candidatos onde o melhor áudio disponível tem abr ≥ atual + 40;
   `upgrade_enqueue(ids)` ⇒ jobs `kind=upgrade`: baixa o original ⇒ converte para o **perfil do
   item** ⇒ grava as **tags atuais** do arquivo existente (lidas com `read_tags`, incluindo
   capa/letra/RG recalculado) ⇒ substitui o arquivo atomicamente (o antigo vai para a lixeira) ⇒
   atualiza `source_abr_kbps`/`bitrate_kbps`. UI: ação por item, em lote e "Verificar
   biblioteca inteira" em Avançado.
4. **Capítulos** (`splitChapters`): vídeo com ≥ 2 capítulos e duração > 10 min ⇒ Preview
   oferece "Dividir em faixas" (`ask`) ou aplica sempre/nunca. Job `kind=chapters`: baixa
   inteiro ⇒ para cada capítulo `ffmpeg -ss <start> -to <end> -i <arq> -c copy` (perfil
   original) ou com o codec do perfil ⇒ tags: álbum = título do vídeo limpo (`parse_title`),
   artista do álbum = artista analisado/canal, faixa = índice/total, título = título do capítulo
   limpo (remover numeração inicial `01.`, `1 -`, timestamps `00:00`) ⇒ capa = miniatura
   recortada ou capa encontrada para o álbum ⇒ organiza cada faixa pelo modelo ⇒ uma linha na
   biblioteca por faixa.
5. **AcoustID**: com `acoustidKey`, instalar `fpcalc` sob demanda (F02); no passo `identify`,
   se a confiança < `confidenceAutoApply`, rodar `fpcalc -json <arquivo>` ⇒
   `GET https://api.acoustid.org/v2/lookup?client=<key>&meta=recordings+releasegroups&duration=<d>&fingerprint=<fp>`
   ⇒ candidatos (score AcoustID ≥ 0.8 vira candidato com confiança = score AcoustID × 0.95) e
   `mb_recording_id`/`acoustid_id` guardados. **Duplicatas** passam a considerar também
   `acoustid_id`/`mb_recording_id` iguais (aviso no enfileiramento/importação).
6. **Spotify** (opcional): Client Credentials (`POST https://accounts.spotify.com/api/token`),
   busca `GET https://api.spotify.com/v1/search?type=track&limit=5&q=track:<t> artist:<a>`;
   token em memória até expirar. **Discogs** (opcional): `GET https://api.discogs.com/database/search?type=release&track=<t>&artist=<a>&token=<token>`
   (gênero/estilo/ano/capa). Ambos entram no pool de candidatos do §11 quando configurados.
   Botões "Testar" em Configurações › Metadados (paridade ApiConfigSection) com status.
   Se a API do Spotify recusar (restrições de apps novos), mostrar o erro claramente e seguir sem ela.
7. **Cortar áudio** (ideia aprovada na discussão):
   - Automático: nova configuração `trimSilence` (bool, padrão `false`, aba Downloads) ⇒ passo
     opcional antes do `loudness` que remove silêncio do início e do fim com ffmpeg
     `silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0.3:stop_periods=1:stop_threshold=-50dB:stop_silence=0.5`
     (implica recodificar no perfil do item; no perfil `original`, recodificar no mesmo codec
     com bitrate ≥ o da fonte e registrar aviso). A chave já consta da tabela §6 (criada na F01).
   - Manual: ação "Cortar" na Biblioteca/Editor de tags ⇒ diálogo com a **forma de onda**
     (`waveform(path) -> PNG base64` via ffmpeg `showwavespic=s=1200x160`), campos de início/fim
     (mm:ss.s) arrastáveis sobre a imagem, e `trim_audio(path, start_s, end_s)` que gera o novo
     arquivo no tmp com `-ss/-to` (cópia de stream quando o formato permitir), preserva as tags
     (reler e regravar), substitui atomicamente e manda o antigo para a lixeira.
     (Sem player de áudio embutido nesta versão — evita habilitar o protocolo de assets.)

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | Argumentos com cookies (cada navegador e arquivo) presentes em analyze/search/download; segredos/caminho de cookies não aparecem nos logs (redação) | não |
| T2 | Detecção de Premium com fixtures de formatos (com e sem 141/774); classificação dos erros de cookies a partir de stderr de exemplo | não |
| T3 | Decisão de upgrade: tabela (atual 129 / disponível 129 ⇒ não; 129/256 ⇒ sim; 160/192 ⇒ não; provedor não-YouTube ⇒ não) | não |
| T4 | Execução de upgrade com `FakeBackend`: arquivo substituído, tags anteriores preservadas (releitura), antigo na lixeira, banco atualizado | não |
| T5 | Capítulos: áudio sintético de 30 s + 3 capítulos (0–10, 10–20, 20–30) ⇒ 3 arquivos com duração 10 ±0.3 s, tags de faixa 1/3, 2/3, 3/3, álbum correto, títulos limpos; tabela de limpeza de títulos ≥ 8 casos | não |
| T6 | AcoustID com wiremock + `fpcalc` real (de `.test-tools`) num arquivo sintético ⇒ impressão digital não vazia; resposta gravada ⇒ candidato e ids salvos; duplicata por `mb_recording_id` detectada | não |
| T7 | Spotify e Discogs com wiremock: token, busca, expiração do token, erro 403 tratado | não |
| T8 | UI: teste de cookies mostra Premium/sem Premium/erro; ação "Melhorar qualidade"; opção de capítulos no Preview; status das chaves; diálogo "Cortar" com forma de onda (mock) e validação início < fim | Vitest + Playwright |
| T8b | Corte: áudio sintético "2 s silêncio + 5 s seno + 3 s silêncio" com `trimSilence` ⇒ duração final 5 ±0.6 s; `trim_audio(…, 1.0, 4.0)` num arquivo taggeado ⇒ duração 3 ±0.2 s e tags/capa preservadas; `waveform` devolve PNG válido 1200×160 | não |
| T9 | (rede) `cookies_test` com `cookiesSource = firefox`: se o Firefox não estiver instalado ⇒ erro `browser_not_found` tratado (teste passa verificando a mensagem); se instalado ⇒ `ok` (Premium ou não) | sim |
| T10 | (rede, lento — rodar uma vez no portão) Capítulos reais em FX6 ⇒ 6 arquivos organizados num álbum | sim |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- `-c copy` com `-ss` antes de `-i` corta no quadro-chave mais próximo; para Opus/AAC o erro é
  de milissegundos (aceitável). Se a tolerância de T5 falhar, use `-ss` depois de `-i`.
- Não registre o conteúdo do token do Spotify nem a chave do AcoustID em logs.
