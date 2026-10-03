# Spotify F15

Respostas sintéticas baseadas nos contratos oficiais de Client Credentials e Web API:
https://developer.spotify.com/documentation/web-api/tutorials/client-credentials-flow
https://developer.spotify.com/documentation/web-api/reference/get-playlists-tracks
https://developer.spotify.com/documentation/web-api/reference/get-playlists-items

Não contêm credenciais de usuários. Token, paginação, formatos track/item, restrição
de playlist e Retry-After são exercitados por wiremock. T12 real ficou N/A com
consentimento do usuário, pois não foram fornecidas credenciais. Playlists podem
exigir autenticação do proprietário/colaborador; fixtures não comprovam esse acesso.
