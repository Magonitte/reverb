# Arquitetura: mapa para continuidade

## Camadas

| Diretório                       | Responsabilidade                                        |
| ------------------------------- | ------------------------------------------------------- |
| `src/screens`, `src/components` | Navegação, telas e componentes React                    |
| `src/stores`                    | Estado da interface com Zustand                         |
| `src/lib/ipc`                   | Contrato de chamada/eventos e implementação mock        |
| `src/bindings`                  | Tipos TypeScript gerados dos modelos Rust               |
| `src-tauri/src/commands`        | Adaptadores IPC para os serviços do core                |
| `src-tauri/src`                 | Janela, bandeja, notificações, deep links e atualizador |
| `crates/reverb-core/src`        | Regras de domínio, serviços e persistência              |
| `crates/reverb-cli`             | Interface de terminal sobre o mesmo core                |
| `tests`, `crates/*/tests`       | Fixtures e validações por camada                        |

## Fluxo de download

```text
Interface → IPC → serviço de fila → backend/ferramentas
                                   ↓
                   metadados → pós-processamento → biblioteca SQLite
                                   ↓
                          eventos de progresso → interface
```

SQLite registra configurações, jobs, biblioteca, sincronizações e artistas. O serviço de fila trata estados persistentes, tentativas e cancelamento. O pipeline resolve ferramentas gerenciadas, baixa/processa o áudio, grava tags, organiza o destino e registra a biblioteca. Os serviços publicam eventos por `EventSink`; o app Tauri encaminha-os à interface.

## Importação e coleção (F15)

`import/source.rs` classifica URLs, consulta Deezer/Spotify e normaliza os catálogos. Paginação é restrita à origem esperada. `import/service.rs` faz casamento com o backend, prioriza ISRC e mantém análises recentes para seleção. Faixas e suas análises de candidatos compartilham um limite global de três consultas ao backend, incluindo cancelamento durante a espera por uma vaga.

O job mantém os metadados/ISRC da fonte, `importSourceTrackId` e `importMatch` para documentar o vídeo, confiança e método efetivamente usados ao enfileirar. A escolha de áudio não transforma o provedor de catálogo em origem de download.

`sync/service.rs` estende as playlists: diff pelo ID da fonte, vídeo casado/nota/metadados persistidos em `sync_items` e checksum no resultado. `artists.rs` consulta discografias, filtra lançamentos, aplica políticas de acompanhamento, cruza a biblioteca, deduplica ISRC e agenda melhorias de qualidade. As notificações e o agendamento nativo são conectados na inicialização Tauri.

## Documentação técnica de referência

- [Arquitetura detalhada](../plano/01-arquitetura.md): schema, configurações, IPC, nomes e algoritmos.
- [Design](../plano/02-design.md): sistema visual, navegação e interação.
- [Paridade](../plano/03-paridade.md): relação com o aplicativo anterior.
- [Contrato yt-dlp](../plano/anexos/ytdlp-contrato.md) e [fixtures](../plano/anexos/fixtures.md): comportamento externo esperado.
- [Estudos](../plano/anexos/estudos/LEIA-ME.md): especificações de sala limpa.
- [Fases](../plano/fases/) e [PROGRESS](../PROGRESS.md): requisitos, decisões e o que já foi validado.

O plano contém especificações futuras. Em caso de diferença, confira o código e o desvio registrado no PROGRESS; não presuma que todo comando proposto já foi implementado. Não mova código ou renomeie comandos sem atualizar imports, scripts, testes, bindings e documentação.
