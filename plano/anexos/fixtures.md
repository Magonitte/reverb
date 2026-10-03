# Anexo — Fixtures de teste (validadas em 2026-10-01)

Todas foram verificadas com yt-dlp 2026.08.19 + Deno 2.9.7 no Windows do usuário.
Se alguma deixar de existir, substitua por uma equivalente que cumpra o **mesmo papel**,
atualize este arquivo e registre no PROGRESS.md.

| ID | URL | Papel | Fatos verificados |
|----|-----|-------|-------------------|
| FX1 | `https://www.youtube.com/watch?v=jNQXAC9IVRw` | Vídeo curto, **não é música** | "Me at the zoo", 19 s, canal `jawed`, melhor áudio formato 251 (Opus ~106 kbps) ⇒ arquivo `.opus`. `content_type` esperado: `other`. Download ~250 KB. |
| FX2 | `https://music.youtube.com/watch?v=lYBUbBu4W08` | **Faixa oficial** do YouTube Music | `track`="Never Gonna Give You Up", `artist`="Rick Astley", `album`="Whenever You Need Somebody", `release_year`=1987, `release_date`=19871112, `categories`=["Music"], duração 214 s. Formatos de áudio sem Premium: 139, 249, 250, 140 (AAC ~129k), 251 (Opus ~129k). |
| FX3 | `https://www.youtube.com/watch?v=dQw4w9WgXcQ` | **Clipe** (não oficial de áudio) | "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)", canal "Rick Astley", 213 s. A busca de versão oficial deve encontrar FX2 (`lYBUbBu4W08`). |
| FX4 | `https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE` | **Álbum/playlist** (10 faixas) | Título "Album - Whenever You Need Somebody", 10 entradas no modo `--flat-playlist` **com duração**: 1º `lYBUbBu4W08` (214 s), 2º `raBobo3GZYA` "Whenever You Need Somebody" (234 s), 3º `i_Q88T1HI_w` "Together Forever" (206 s), 4º `dc7UCha20yw` "It Would Take a Strong Strong Man" (221 s). |
| FX5 | `https://music.youtube.com/browse/MPREb_dcYZhAh5urI` | Álbum pela URL do YouTube Music | O yt-dlp redireciona para FX4 (emite WARNING "YouTube Music is not directly supported. Redirecting…"). |
| FX6 | `https://www.youtube.com/watch?v=OZTNn4wlegM` | Vídeo com **capítulos** (pesado) | "The Northern Hymn - Kammarheit - Full Album", 2643 s, 6 capítulos. Só para o teste de rede opcional-lento da F13. |
| FX7 | busca YouTube Music `rick astley never gonna give you up` (`#songs`) | Busca | 1º resultado `lYBUbBu4W08`; entradas da busca **sem duração** (buscar detalhes). |
| FX8 | `https://archive.org/details/bach-well-tempered-clavier-book-1` | Internet Archive, coleção musical, FLAC gratuito (CC0) | Kimiko Ishizaka — Bach: Well-Tempered Clavier, Book 1; 48 faixas 24bit Flac. Primeira: Prelude No. 1 in C major, BWV 846, track 1, 2015. Substituição autorizada em 2026-10-02: servidor do OpenGoldbergVariations sem resposta; mesma pianista e papel, licença CC0 explícita. |
| FX9 | `https://archive.org/details/MusopenCollectionAsFlac` | Coleção grande em FLAC | Só listar arquivos (não baixar tudo). F14. |

## URLs para a tabela de classificação (§14 da arquitetura)

Válidas (vídeo): `https://youtu.be/dQw4w9WgXcQ`, `https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42`,
`https://www.youtube.com/watch?si=abc&v=dQw4w9WgXcQ`, `https://m.youtube.com/watch?v=dQw4w9WgXcQ`,
`https://www.youtube.com/shorts/aqz-KE-bpKQ`, `https://www.youtube.com/live/jfKfPfyJRdk`,
`https://music.youtube.com/watch?v=lYBUbBu4W08`, `HTTPS://WWW.YOUTUBE.COM/watch?v=dQw4w9WgXcQ`.
Vídeo com dica de playlist: `https://www.youtube.com/watch?v=lYBUbBu4W08&list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE`.
Coleção: FX4, FX5, `https://music.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE`,
`https://www.youtube.com/@RickAstleyYT`, `https://www.youtube.com/@RickAstleyYT/videos`,
`https://www.youtube.com/channel/UCuAXFkgsw1L7xaCfnd5JJOw`.
Busca: `rick astley`, `never gonna give you up`.
Não suportadas: `https://vimeo.com/123`, `ftp://youtube.com/watch?v=x`, `https://youtube.com.evil.com/watch?v=x`,
`https://www.youtube.com/`, `javascript:alert(1)`, `` (vazio), `   ` (espaços).

## Arquivo de checksum real (exemplos de formato)
- yt-dlp `SHA2-256SUMS`: linhas `<64 hex>  <nome-do-arquivo>` (dois espaços).
- FFmpeg-Builds `checksums.sha256`: linhas `<64 hex>  <nome>`.
- Deno `<asset>.sha256sum`: formato pode variar; extrair o **primeiro** token de 64 hex.
Na F02, baixe um exemplo real de cada e salve em `tests/fixtures/checksums/` para os testes offline.

## Fixtures de provedores F14 (2026-10-02)
- Archive anterior: respostas OpenGoldbergVariations preservadas para regressão offline; nova busca e metadados em tests/fixtures/http/f14/archive-replacement-*.json.
- SoundCloud: https://soundcloud.com/scottbuckley/icarus-cc-by — Scott Buckley, Icarus, CC-BY; yt-dlp selecionou hls_aac_160k (stream, original indisponível). O seletor download/bestaudio prefere o original quando oferecido; aviso explícito no fallback e intervalo de 10 s no roteador.
- Bandcamp: https://soundslikeanearful.bandcamp.com/album/creative-commons-vol-1, primeira faixa https://soundslikeanearful.bandcamp.com/track/mellow-harmonics — download gratuito sem cadastro/e-mail, FLAC validado; páginas reais gravadas em tests/fixtures/http/f14/bandcamp-*.html. Links temporários são renovados pelo statdownload público.
- Jamendo: chave não fornecida; teste de rede opcional N/A. Parsers de permissão true/false e UI verificados offline; configuração pode ser feita depois pelo Client ID.
