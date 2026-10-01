# E2 — Casos de borda da sincronização de playlists

Complementa a F11. Comportamentos observados em uma ferramenta madura de sincronização:

1. **Estado salvo por sincronização**: a lista de itens da última execução (com a URL/ID de
   origem e os metadados usados para nomear) — no Reverb isso já é `sync_items`.
2. **Removidos**: item presente antes e ausente agora ⇒ remover o arquivo **somente** se a
   opção de remoção estiver ligada (padrão desligado no Reverb: `remove_deleted = 0`).
   Arquivos companheiros (`.lrc`, e a capa só se nenhum outro arquivo da pasta a usa) seguem o
   mesmo destino.
3. **Renomear em vez de baixar de novo**: se o modelo de nomes usa variáveis que dependem da
   playlist (`{playlist_index}`, `{playlist}`) e a posição/título mudou, o arquivo existente é
   **renomeado** para o novo caminho (e o `.lrc` junto) — nunca baixado de novo. Se o destino
   já existir, mantenha o existente e remova a origem.
4. **Faixa duplicada na mesma playlist** (mesmo ID em duas posições): baixar **uma vez**; com
   variáveis de posição no modelo, a segunda posição recebe uma **cópia** (não mover a única),
   ou — preferível no Reverb — usar apenas a primeira ocorrência e registrar aviso. Decisão:
   **usar a primeira ocorrência** e listar duplicatas na UI.
5. **Falhas parciais**: itens que falharam continuam "pendentes" e são tentados de novo na
   próxima execução; não contam como "removidos".
6. **.m3u8** é regenerado ao fim de cada execução, a partir do estado novo.
7. **Detecção barata de mudança** (fontes que oferecem): o Deezer devolve `checksum` na
   listagem de faixas da playlist; se igual ao da execução anterior ⇒ nada mudou (pular o diff).
   Guardar em `syncs.last_result_json`.

## Testes de aceitação (acrescentar à F11)
- Mudança de posição com `{playlist_index}` no modelo ⇒ arquivo renomeado (mesmo conteúdo,
  sem novo download); `.lrc` renomeado junto.
- Duplicata na playlist ⇒ 1 download e aviso.
- Item que falhou na execução 1 é re-enfileirado na execução 2.
- Checksum igual (Deezer) ⇒ execução termina sem analisar itens.
