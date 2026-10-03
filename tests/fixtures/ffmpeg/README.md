# Fixtures FFmpeg

`ebur128-pink-noise.txt` é o stderr real capturado em 2026-10-02 com FFmpeg
`N-127083-g65a3870462-20261001` (build gerenciado de teste, Windows).
Inclui as leituras por frame e o resumo final; a asserção do parser
usa o resumo final: I = −25.1 LUFS, True Peak = −12.6 dBFS.

Comando de captura (stdout descartado; stderr salvo integralmente):

```text
ffmpeg -hide_banner -nostdin -nostats -f lavfi -i anoisesrc=color=pink:amplitude=0.3:duration=2:seed=1 -af ebur128=peak=true -f null -
```
