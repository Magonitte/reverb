# Configuração

Abra **Configurações** pela barra lateral. Alterações são persistidas pelo aplicativo.

## Geral e downloads

Em Geral, escolha idioma, tema e transparência. Em Downloads, configure a pasta de saída, formato padrão, paralelismo, limite de velocidade e pós-processamento. Comece com os valores padrão e ajuste conforme o espaço disponível e a capacidade da conexão.

| Perfil   | Resultado                                                             |
| -------- | --------------------------------------------------------------------- |
| Original | Preserva o áudio obtido, sem recodificação                            |
| MP3 V0   | MP3 com bitrate variável                                              |
| MP3 320  | MP3 com bitrate de saída de 320 kbps                                  |
| AAC 256  | AAC com bitrate de saída de 256 kbps                                  |
| Opus 96  | Opus com bitrate de saída de 96 kbps                                  |
| FLAC     | Saída FLAC; converter uma fonte com perdas não recupera áudio perdido |

O modelo de nomes padrão é `{albumartist}/{album}/{track:02} - {title}`. Ele cria pastas de artista/álbum e numeração de dois dígitos. Tokens de playlist também são suportados; a validação rejeita tokens desconhecidos. Consulte o contrato de nomes em [arquitetura §13](../plano/01-arquitetura.md).

Capas, letras/LRC, ReplayGain, cortes, capítulos e remoção de silêncio possuem opções próprias. A disponibilidade de metadados e letras depende dos provedores e da faixa; nem todo arquivo terá todos os campos.

### Qualidade-alvo

Escolha desativado, 160 ou 256 kbps e habilite **Melhorar automaticamente** quando desejar. O agendador diário procura melhorias para itens YouTube abaixo do alvo, com bitrate da fonte conhecido. A F13 exige um ganho mínimo de 40 kbps e a F15 mantém esse critério. O formato nominal do arquivo, por exemplo MP3 320, não prova que a fonte possui essa qualidade. O app nunca deve substituir o item por uma fonte de qualidade inferior.

## Metadados e credenciais

Deezer não requer chave. As integrações opcionais usam credenciais do próprio usuário, configuradas na seção correspondente; não são fornecidas credenciais compartilhadas no repositório.

### Spotify

1. Abra **Metadados → Spotify → Como configurar o Spotify**.
2. Use o botão para abrir o painel de desenvolvedores e crie um app na sua própria conta.
3. Copie Client ID e Client Secret para os campos do Reverb e salve.
4. Teste a conexão e tente importar uma coleção acessível ao seu aplicativo.

O Reverb implementa Client Credentials. Autenticar o app **não comprova permissão para ler uma playlist**. A [API atual de itens de playlist](https://developer.spotify.com/documentation/web-api/reference/get-playlists-items) exige acesso do proprietário/colaborador autenticado; o fluxo de login OAuth do usuário não está implementado nesta fase. A importação de uma playlist pode retornar uma mensagem de acesso restrito mesmo com credenciais válidas. Álbuns e faixas também dependem das permissões e dos campos disponibilizados pela API.

Playlists usam `/playlists/{id}/items`, conforme a [mudança de fevereiro/2026](https://developer.spotify.com/documentation/web-api/references/changes/february-2026). Os testes simulados cobrem token, renovação, paginação, erros e Retry-After. O teste real Spotify da F15 foi N/A com consentimento do usuário, sem credenciais fornecidas. **Não há garantia de acesso real Spotify nesta entrega.** Deezer continua disponível sem essa configuração.

### Outras integrações

- **Jamendo:** forneça seu Client ID em Integração para consultar a API; só conteúdos cuja resposta permita download são enfileirados.
- **AcoustID/Discogs:** configure credenciais próprias se quiser usar esses provedores opcionais na identificação.
- **Cookies:** selecione o navegador compatível ou um arquivo de cookies nas opções de qualidade. Cookies só permitem usar o acesso que sua própria sessão já possui.

Segredos ficam nas configurações locais do app; a visão pública de configurações e os logs filtram valores registrados como secretos. **O banco local não é um cofre criptografado.** Proteja sua conta do sistema, backups e arquivo de cookies. Não publique esses arquivos, Client Secrets, tokens ou cookies em commits, capturas ou issues.

## Integração, atualizações e avançado

Em Geral, configure início com o sistema, bandeja, notificações, área de transferência e atalho global. Em Integração ficam bookmarklet/deep links e o intervalo de verificação de artistas (padrão: 24 horas). O monitoramento de artistas e as tarefas automáticas dependem do app em execução; artistas vencidos são verificados após o atraso inicial de cinco minutos.

Em Atualizações, consulte o app e as ferramentas gerenciadas. Em Avançado, execute diagnóstico, exporte/restaure dados, selecione runtime/canal do yt-dlp ou ative o modo offline. O modo offline impede recursos que precisam consultar fontes externas; ele não disponibiliza catálogos remotos sem conexão.
