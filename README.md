# Reverb

Aplicativo desktop para baixar áudio, conferir metadados e organizar uma biblioteca musical local. A interface está disponível em português do Brasil e inglês, com temas claro e escuro.

O projeto usa **Tauri 2, Rust, React e TypeScript**. Windows é a plataforma principal; Linux recebe pacotes e validação no CI. A reconstrução está na linha **0.1.x**: consulte o [andamento das fases](PROGRESS.md) antes de continuar o desenvolvimento. A F16 e o Android permanecem etapas futuras, descritas no plano.

## Começar a usar

1. Baixe o instalador da sua plataforma em [Releases](https://github.com/Magonitte/reverb/releases).
2. Instale e abra o Reverb. No primeiro uso, escolha a pasta da biblioteca e aguarde a preparação das ferramentas.
3. Cole um link na barra de comando, confira a pré-visualização e escolha o formato de saída.
4. Acompanhe o download em **Atividade** e encontre o resultado na **Biblioteca**.

Não é necessário instalar Node.js, Rust ou Python para usar os pacotes publicados. O Reverb gerencia yt-dlp, Deno, FFmpeg e ferramentas auxiliares.

## Recursos

- Downloads de YouTube/YouTube Music, fila persistente, pausa, cancelamento e recuperação de falhas.
- Perfis Original, MP3 V0/320, AAC 256, Opus 96 e FLAC; capas, tags, letras e organização de nomes.
- Biblioteca com busca, importação de arquivos, revisão de metadados e editor de tags.
- Playlists sincronizadas, detecção de alterações e exportação M3U8.
- SoundCloud, Bandcamp gratuito, Internet Archive e Jamendo com Client ID próprio.
- Importação de catálogos Deezer/Spotify, casamento no YouTube por ISRC ou texto, artistas seguidos e visão **Faltando**.
- Melhoria de qualidade e análise de provável lossless com espectrograma.
- Atualizador do Reverb em Configurações, com consulta manual e instalação assinada; ferramentas com botões de instalação/atualização e progresso.
- Bandeja, notificações, atalhos e diagnóstico.

**Deezer funciona sem credenciais.** Spotify requer credenciais próprias e permissões compatíveis. Os testes contratuais do Spotify não comprovam acesso real da conta; a verificação de rede sem credenciais foi registrada como N/A com consentimento do usuário. Veja as [limitações e a configuração](docs/configuracao.md#spotify).

## Documentação

| Preciso de…                                              | Onde encontrar                                            |
| -------------------------------------------------------- | --------------------------------------------------------- |
| Instalar, atualizar ou conhecer os diretórios de dados   | [Instalação](docs/instalacao.md)                          |
| Configurar formatos, metadados, credenciais e integração | [Configuração](docs/configuracao.md)                      |
| Baixar, importar, sincronizar e acompanhar artistas      | [Uso](docs/uso.md)                                        |
| Resolver erros e coletar um diagnóstico                  | [Solução de problemas](docs/solucao-de-problemas.md)      |
| Rodar o código, testes, CLI e publicar uma versão        | [Desenvolvimento](docs/desenvolvimento.md)                |
| Entender os módulos, dados e contratos                   | [Arquitetura](docs/arquitetura.md)                        |
| Contribuir ou retomar uma fase                           | [Contribuição](CONTRIBUTING.md) e [PROGRESS](PROGRESS.md) |
| Consultar mudanças por versão                            | [Changelog](CHANGELOG.md)                                 |

O [índice de documentação](docs/README.md) reúne os guias. O [plano original](LEIA-ME_PRIMEIRO.md) contém especificações e regras de execução; requisitos planejados não são, por si só, recursos já entregues.

## Estrutura do repositório

```text
Reverb/
├── docs/                 Guias de uso e desenvolvimento
├── src/                  Interface React, traduções, IPC e mocks
├── src-tauri/            Aplicativo desktop e comandos nativos
├── crates/
│   ├── reverb-core/      Serviços, banco, fila, metadados e importação
│   ├── reverb-cli/       Interface de linha de comando
│   └── fake-tool/        Ferramenta controlada para testes
├── tests/                Fixtures, Playwright e testes do app real
├── scripts/              Validação, fixtures, versões e release
├── .github/workflows/    CI e publicação Windows/Linux
├── plano/                Arquitetura detalhada, estudos e fases
└── referencias/          Material histórico de referência
```

`target/`, `dist/`, `node_modules/`, `.test-tools/` e relatórios locais são gerados e ficam fora do Git. Os arquivos de lock são versionados para permitir instalações reproduzíveis.

## Desenvolvimento rápido

```sh
git clone https://github.com/Magonitte/reverb.git
cd reverb
npm ci
npm run dev:mock
```

Esse comando abre uma interface com dados simulados em `http://localhost:1421`. Para o app nativo, instale os pré-requisitos do [guia de desenvolvimento](docs/desenvolvimento.md) e execute `npm run tauri -- dev`.

## Licença

O código do Reverb é distribuído sob **GPL-3.0-only**. Consulte [LICENSE](LICENSE). Ferramentas externas e conteúdos de terceiros possuem suas próprias licenças.
