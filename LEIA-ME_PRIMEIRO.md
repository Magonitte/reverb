# Reverb 3 — Plano de Implementação (ponto de entrada)

> **Para usuários e contribuidores:** comece pelo [README](README.md) e pelo
> [índice de documentação](docs/README.md). Este arquivo continua sendo a entrada do plano.

> **Para a LLM executora:** este arquivo é o seu ponto de partida. Leia-o inteiro antes de
> qualquer ação. Ele diz em que ordem ler o resto e como trabalhar.

## O que é o projeto

Reverb é um app desktop pessoal (Windows principal, Linux secundário, Android opcional depois)
para baixar áudio do YouTube/YouTube Music — faixas individuais e playlists — com metadados
corretos, capas, letras, organização automática da biblioteca e **atualização automática
obrigatória** (do app e das ferramentas externas).

É uma **reconstrução do zero** do projeto antigo (Tauri + React + sidecar Python).
O código novo está na raiz deste repositório, atualmente
`D:\Documentos\4 - Pessoal\Reverb`; o projeto antigo foi movido para `Reverb_old`,
conforme decisão registrada no PROGRESS. O antigo serve **somente como referência de
leitura**: nunca modifique nada lá. `Reverb_claude` era o nome proposto no plano inicial,
mas não é o diretório de execução atual. A reconstrução usa Rust, sem o sidecar Python.

## Ordem de leitura obrigatória

1. `LEIA-ME_PRIMEIRO.md` (este arquivo)
2. `plano/00-protocolo-de-execucao.md` — **regras de trabalho, portões de teste, ciclo de correção**
3. `plano/01-arquitetura.md` — stack, versões, estrutura, banco, contratos, algoritmos
4. `plano/02-design.md` — sistema visual, telas, navegação, atalhos
5. `plano/03-paridade.md` — tudo que o app antigo fazia e em qual fase entra no novo
6. `plano/anexos/fixtures.md` e `plano/anexos/ytdlp-contrato.md` — dados reais validados
7. `plano/anexos/estudos/LEIA-ME.md` — especificações de "sala limpa" (E1–E7); leia cada
   estudo quando a fase atual o citar. **Nunca abra os repositórios de origem** (nem
   `G:\Estudo_Repo`): implemente só a partir dos estudos.
8. **Somente a fase atual** em `plano/fases/` (ver `PROGRESS.md` para saber qual é)
9. `plano/04-android.md` — só depois que todas as fases desktop estiverem concluídas

Ao iniciar **qualquer** sessão de trabalho (inclusive retomando após interrupção):
leia este arquivo, o protocolo (`00`), e o `PROGRESS.md`. Continue de onde parou.

## Mapa das fases

| Fase  | Arquivo                                   | Resultado verificável                                                                           |
| ----- | ----------------------------------------- | ----------------------------------------------------------------------------------------------- |
| F00   | `fases/F00-fundacao.md`                   | Monorepo compila; testes, lint, CI e self-test headless funcionando                             |
| F01   | `fases/F01-banco-e-configuracoes.md`      | SQLite com migrações, configurações, logs, tipos TS gerados                                     |
| F02   | `fases/F02-gerenciador-de-ferramentas.md` | yt-dlp, Deno, FFmpeg baixados, verificados, atualizáveis, com rollback                          |
| F03   | `fases/F03-motor-de-download.md`          | Analisar/baixar/converter via CLI com progresso, cancelamento e erros classificados             |
| F04   | `fases/F04-fila-e-autocura.md`            | Fila persistente, paralela, com retry e autocura do yt-dlp                                      |
| F05   | `fases/F05-shell-ui-e-design.md`          | UI base fiel ao design, temas, i18n, mock de backend, testes visuais                            |
| F06   | `fases/F06-atualizacoes-e-release.md`     | Atualizador do app + botão "Atualizar" + CI de release Windows/Linux testados de verdade        |
| F07   | `fases/F07-fluxo-de-download-ui.md`       | Início, barra de comando, preview, coleção, Atividade — fluxo completo na UI                    |
| F08   | `fases/F08-metadados.md`                  | Identificação com nota de confiança, YouTube Music oficial, tipo de conteúdo                    |
| F09   | `fases/F09-pos-processamento.md`          | Capas, letras, ReplayGain, tags, organização em pastas, biblioteca indexada                     |
| F10   | `fases/F10-biblioteca-revisao-editor.md`  | Biblioteca, fila de revisão, editor de tags, importação, vigia de pasta                         |
| F11   | `fases/F11-playlists-e-sincronizacao.md`  | Playlists sincronizadas, diff, .m3u8, agendamento                                               |
| F12   | `fases/F12-integracao-e-onboarding.md`    | Bandeja, notificações, deep link, área de transferência, atalho global, onboarding, diagnóstico |
| F13   | `fases/F13-qualidade-avancada.md`         | Cookies/Premium, "Melhorar qualidade", capítulos, AcoustID, Spotify/Discogs                     |
| F14   | `fases/F14-provedores-extras.md`          | SoundCloud, Bandcamp, Internet Archive, Jamendo, verificador de FLAC                            |
| F15   | `fases/F15-importacao-e-colecao.md`       | Importar playlists do Deezer/Spotify, seguir artistas, "Faltando", qualidade-alvo               |
| F16   | `fases/F16-polimento-e-release.md`        | Desempenho, acessibilidade, regressão total, release 3.0.0                                      |
| A0–A3 | `04-android.md`                           | Android (condicional ao spike A0 dar certo)                                                     |

As fases são **estritamente sequenciais**. Nunca comece uma fase antes de a anterior estar
marcada como `CONCLUÍDA` no `PROGRESS.md` com todos os testes do portão passando.

## Estrutura do planejamento inicial

```
Reverb/
├── LEIA-ME_PRIMEIRO.md          ← você está aqui
├── PROGRESS.md                  ← registro vivo do andamento (você atualiza)
├── plano/                       ← o plano (NÃO altere o conteúdo técnico sem registrar no PROGRESS)
│   ├── 00-protocolo-de-execucao.md
│   ├── 01-arquitetura.md
│   ├── 02-design.md
│   ├── 03-paridade.md
│   ├── 04-android.md
│   ├── anexos/
│   └── fases/
└── referencias/                 ← material de referência copiado do projeto antigo
    ├── design/                  ← HTML de referência visual + screenshots do app antigo
    ├── icones/                  ← ícones .ico/.png do app
    ├── parseTrackTitle.antigo.ts
    └── translation.pt-BR.antigo.json   ← textos do app antigo (referência de tom/termos)
```

O código do app será criado na raiz desta pasta (ao lado de `plano/`), conforme a estrutura
definida em `plano/01-arquitetura.md` §3.
