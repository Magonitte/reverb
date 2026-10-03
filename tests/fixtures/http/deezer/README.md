# Deezer F15

Respostas públicas gravadas em 2026-10-02, sem credenciais:

- chart/0/playlists?limit=1: playlist de teste obtida dinamicamente.
- playlist/5207214368 e /tracks: Top 50 Sertanejo.
- album/301775 e /tracks: Homework, incluindo ISRC/posição/disco.
- artist/6160 e /albums?index=0&limit=25, index=25: Rick Astley e discografia paginada.
- erro 800: resposta para objeto inexistente.

Os testes offline montam duas páginas com conteúdo gravado e links wiremock locais.
T10/T11 consultam a rede novamente, sem tratar estas capturas como prova de disponibilidade.
