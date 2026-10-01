# F14 — Provedores extras (lossless legal e gratuito) e verificador de FLAC

**Objetivo:** baixar de fontes legais além do YouTube — SoundCloud (arquivo original quando
liberado), Bandcamp (lançamentos gratuitos), Internet Archive (FLAC) e Jamendo (FLAC via API) —
e verificar se um FLAC é lossless de verdade.

**Escopo legal (obrigatório):** apenas conteúdo que o próprio serviço/artista disponibiliza
para download gratuito. **Não** implementar contornos de pagamento, extração de streams de
serviços pagos (Qobuz, Tidal, Deezer HiFi etc.) nem automação de cadastro por e-mail.

**Pré-requisito:** F13. **Arquitetura:** §14 (URLs dos provedores), §7, §12.

## Tarefas

1. **Abstração de fonte**: `trait SourceProvider { fn id(); fn matches(url) -> Option<UrlKind>;
   async fn analyze(url); async fn search(q) (opcional); async fn download(item, workspace,
   progress, cancel) -> DownloadedFile }`. YouTube passa a ser um `SourceProvider` (via yt-dlp).
   `url_classify` consulta todos.
2. **SoundCloud** (via yt-dlp): analisar com `-F`/`-J` e preferir o formato do arquivo original
   (no yt-dlp aparece como formato de download do artista — confirmar o `format_id` real com
   uma faixa que tenha download liberado e registrar em `fixtures.md`); se indisponível, usar o
   melhor stream e marcar aviso "original indisponível". Pausa de 10 s entre downloads de
   originais (limite de taxa conhecido — issue yt-dlp #15093).
3. **Bandcamp** (download direto gratuito): ler o JSON `data-tralbum` da página; se o
   lançamento oferecer `freeDownloadPage` **sem exigir e-mail**, seguir para a página de
   download, ler `data-blob`, escolher o formato `flac` e baixar. Caso contrário ⇒ resultado
   "Este lançamento exige compra ou e-mail — abrir no navegador" (abre com opener). Estudar o
   fluxo atual em `https://github.com/7x11x13/free-bandcamp-downloader` (somente leitura,
   respeitando a licença; reimplementar em Rust).
4. **Internet Archive**: busca `https://archive.org/advancedsearch.php?q=<q> AND mediatype:audio&fl[]=identifier&fl[]=title&fl[]=creator&rows=20&output=json`;
   item `https://archive.org/metadata/<id>` ⇒ arquivos com `format` contendo `Flac` (preferir
   `24bit Flac` se existir, senão `Flac`, senão `VBR MP3`); download
   `https://archive.org/download/<id>/<arquivo>`; um item com várias faixas vira **coleção**
   (selecionar faixas como na tela Coleção); metadados do item (`title`, `creator`, `date`) e
   por arquivo (`title`, `track`).
5. **Jamendo** (requer `jamendoClientId`): `GET https://api.jamendo.com/v3.0/tracks/?client_id=<id>&format=json&limit=20&search=<q>&audiodlformat=flac`
   ⇒ usar `audiodownload` só se `audiodownload_allowed = true`; metadados da própria API.
6. **Verificador de lossless** (`verify_lossless(path) -> { verdict: lossless|lossy_16k|lossy_19k|inconclusive,
   details, spectrogram_png_base64 }`): medir com ffmpeg o nível RMS da banda acima de 16,5 kHz
   (`highpass=f=16500` + `astats`) e acima de 19,5 kHz, comparar com o RMS total; corte
   abrupto ⇒ `lossy_16k` (típico MP3 128) ou `lossy_19k` (MP3 320/AAC); taxa de amostragem
   < 44,1 kHz ⇒ `inconclusive`. Espectrograma: `showspectrumpic=s=800x300` em PNG temporário.
   Aplicar automaticamente em importações de FLAC/WAV se `verifyLosslessOnImport`;
   gravar `lossless_verdict` na biblioteca. Ação "Verificar lossless" na Biblioteca.
7. **UI**: abas de fonte na busca (YouTube Music | YouTube | Internet Archive | Jamendo),
   reconhecimento de URLs SoundCloud/Bandcamp/archive.org/Jamendo na barra de comando, selo de
   provedor e "Lossless" nos itens, tela/diálogo do verificador com o espectrograma.
8. Perfis continuam valendo (perfil `original` mantém o FLAC).
9. Gravar respostas reais (Internet Archive, Jamendo se houver chave, página Bandcamp de um
   lançamento gratuito encontrado e validado) em `tests/fixtures/http/`.

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | `matches`/`url_classify` para URLs de cada provedor (válidas e inválidas) | não |
| T2 | Parsers com respostas gravadas: Internet Archive (busca, metadados, escolha do arquivo FLAC), Jamendo (`audiodownload_allowed` true/false), Bandcamp (gratuito sem e-mail ⇒ link do FLAC; exige e-mail/compra ⇒ resultado "abrir no navegador") | não |
| T3 | Verificador: ruído branco 44,1 kHz em FLAC ⇒ `lossless`; mesmo ruído → MP3 128 kbps → FLAC ⇒ `lossy_16k`; → MP3 320 → FLAC ⇒ `lossy_19k` (ou `lossy_16k`, desde que **não** `lossless`); 22,05 kHz ⇒ `inconclusive`; espectrograma PNG válido | não |
| T4 | Download de provedor HTTP direto com wiremock: progresso, cancelamento, workspace limpo | não |
| T5 | UI: abas de fonte, selos, verificador | Vitest + Playwright |
| T6 | (rede) FX8: buscar/abrir item, baixar a 1ª faixa FLAC ⇒ codec `flac` (ffprobe), veredito `lossless`, organizado com tags | sim |
| T7 | (rede) SoundCloud: com a faixa de fixture registrada, baixar ⇒ arquivo válido; registrar se veio original ou stream | sim |
| T8 | (rede) Bandcamp: com o lançamento gratuito validado (registrado em `fixtures.md`), baixar 1 faixa FLAC; se não houver nenhum lançamento gratuito sem e-mail encontrável, **parar e perguntar ao usuário** se ele fornece uma URL | sim |
| T9 | (rede, só se o usuário fornecer `jamendoClientId`) busca + download FLAC; sem chave ⇒ registrar "N/A com consentimento do usuário" | sim |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- A estrutura das páginas do Bandcamp muda; o parser deve falhar com erro claro, não com pânico.
- O verificador é heurístico: textos da UI devem dizer "provável", nunca "certeza".
