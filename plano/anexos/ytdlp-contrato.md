# Anexo — Contrato do yt-dlp (saídas reais capturadas em 2026-10-01)

## Comando de download validado (Windows, yt-dlp 2026.08.19 standalone, Deno 2.9.7)

```
yt-dlp.exe --ignore-config --no-playlist --newline --progress --color never
  --js-runtimes "deno:C:/…/deno.exe" -f "bestaudio[format_id!*=-drc]/bestaudio"
  -x --audio-format best --ffmpeg-location "D:/…/bin"
  --progress-template "download:REVERB_PROGRESS %(progress)j"
  --print "after_move:REVERB_DONE %(.{id,title,filepath,ext,abr,acodec,format_id})j"
  -o "out/%(id)s.%(ext)s" -- "https://www.youtube.com/watch?v=jNQXAC9IVRw"
```
Código de saída 0. Arquivo final: `out/jNQXAC9IVRw.opus` (o `.webm` intermediário é removido pelo yt-dlp).
(No plano, `REVERB_DONE` também inclui `duration`.)

## stdout real (linhas relevantes)

```
REVERB_PROGRESS {"status": "downloading", "downloaded_bytes": 1024, "total_bytes": 252182, "tmpfilename": "out\\jNQXAC9IVRw.webm.part", "filename": "out\\jNQXAC9IVRw.webm", "eta": null, "speed": null, "elapsed": 0.13831233978271484, "ctx_id": null, "_eta_str": "Unknown", "_speed_str": " Unknown B/s", "_percent": 0.40605594372318404, "_percent_str": "  0.4%", "_total_bytes_str": " 246.27KiB", "_total_bytes_estimate_str": "       N/A", "_downloaded_bytes_str": "   1.00KiB", "_elapsed_str": "00:00:00", "_default_template": "  0.4% of  246.27KiB at  Unknown B/s ETA Unknown"}
REVERB_PROGRESS {"status": "downloading", "downloaded_bytes": 3072, "total_bytes": 252182, "tmpfilename": "out\\jNQXAC9IVRw.webm.part", "filename": "out\\jNQXAC9IVRw.webm", "eta": 0, "speed": 3071490.3189511322, "elapsed": 0.1393125057220459, "ctx_id": null, "_eta_str": "00:00", "_speed_str": "   2.93MiB/s", "_percent": 1.218167831169552, "_percent_str": "  1.2%", "_total_bytes_str": " 246.27KiB", "_total_bytes_estimate_str": "       N/A", "_downloaded_bytes_str": "   3.00KiB", "_elapsed_str": "00:00:00", "_default_template": "  1.2% of  246.27KiB at    2.93MiB/s ETA 00:00"}
REVERB_PROGRESS {"downloaded_bytes": 252182, "total_bytes": 252182, "filename": "out\\jNQXAC9IVRw.webm", "status": "finished", "elapsed": 0.14431262016296387, "ctx_id": null, "speed": 1747470.1776963477, "_speed_str": "1.67MiB/s", "_total_bytes_str": " 246.27KiB", "_elapsed_str": "00:00:00", "_percent": 100.0, "_percent_str": "100.0%", "_default_template": "100% of  246.27KiB in 00:00:00 at 1.67MiB/s"}
REVERB_DONE {"id": "jNQXAC9IVRw", "title": "Me at the zoo", "filepath": "C:\\Users\\…\\out\\jNQXAC9IVRw.opus", "ext": "opus", "abr": 106.064, "acodec": "opus", "format_id": "251"}
```

Observações para o parser:
- `total_bytes` pode faltar; usar `total_bytes_estimate`. Ambos ausentes ⇒ progresso indeterminado.
- `eta`/`speed` podem ser `null`.
- Ignore campos que começam com `_` (strings formatadas para humanos).
- Podem existir linhas `REVERB_PROGRESS` com `"status": "finished"` antes do pós-processamento;
  o fim real é a linha `REVERB_DONE`.
- Saída é UTF-8; títulos com acentos/emoji devem sobreviver (teste com título sintético no parser).

## stderr real (modo `-v`, trechos)

```
[debug] Optional libraries: Cryptodome-3.23.0, brotli-1.2.0, certifi-2026.07.22, curl_cffi-0.16.0, mutagen-1.48.1, requests-2.34.2, sqlite3-3.40.1, urllib3-2.7.0, websockets-16.1.1, yt_dlp_ejs-0.8.0
[debug] JS runtimes: deno-2.9.7
[debug] [youtube] [jsc] JS Challenge Providers: bun (unavailable), deno, node (unavailable), quickjs (unavailable)
```
(Com Node no lugar do Deno, o log mostrou `[youtube] [jsc:node] Solving JS challenges using node`
e `Using challenge solver lib script v0.8.0 (source: python package…)`.)
⇒ O executável oficial **já embute `yt_dlp_ejs`**; basta fornecer o runtime JS.
Sem runtime JS o yt-dlp avisa: `WARNING: [youtube] No supported JavaScript runtime could be found…`
— o diagnóstico (F12) deve tratar esse aviso como **falha**.

## Metadados de faixa oficial (FX2, `-J`)

```
id=lYBUbBu4W08  title=Never Gonna Give You Up  track=Never Gonna Give You Up
artist=Rick Astley  artists=['Rick Astley']  creators=['Rick Astley']
album=Whenever You Need Somebody  album_artist=None  release_year=1987  release_date=19871112
track_number=None  categories=['Music']  channel=Rick Astley  duration=214  chapters=None
thumbnails (últimos): hq720.webp, maxresdefault.jpg 1280x720, maxresdefault.webp 1920x1080
```
Miniaturas são 16:9 ⇒ recorte central quadrado quando forem usadas como capa.
Formatos `sb0..sb3` (storyboards, `mhtml`) e `233/234` (sem codec informado) aparecem na lista —
ignore-os ao calcular "qualidade da fonte" (considere só `vcodec == "none"` e `acodec` não nulo/`none`).

## Busca no YouTube Music (FX7)

`-J --flat-playlist --playlist-end 3 "https://music.youtube.com/search?q=rick+astley+never+gonna+give+you+up#songs"`
⇒ `extractor = youtube:music:search_url`, entradas: `lYBUbBu4W08` "Never Gonna Give You Up",
`wxu4rvTr1L4` "Never Gonna Give You Up (En Directo En La Voz / 2022)", `reR1cGtnZFc` "Never Gonna Fall in Love";
**duration e channel = None** nas entradas ⇒ buscar detalhes.
Busca de álbuns (`#albums`) devolve URLs `music.youtube.com/browse/MPREb_…` (sem título no modo flat).
