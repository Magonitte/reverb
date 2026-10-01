# Estudos de referência (especificações de "sala limpa")

Estes documentos descrevem **comportamentos, regras, endpoints e casos de borda** aprendidos
estudando projetos open source. Eles **não contêm código** dos projetos originais.

**Regra para a LLM executora (obrigatória):** implemente **somente** a partir destes
documentos. **Nunca** abra, clone, copie ou consulte o código-fonte dos repositórios de origem
(nem a pasta `G:\Estudo_Repo`). Se algo estiver ambíguo, decida pelo comportamento descrito,
registre a decisão no PROGRESS.md, ou pergunte ao usuário.

| Estudo | Tema | Origem estudada (somente leitura) | Usado em |
|--------|------|-----------------------------------|----------|
| E1 | Correspondência por ISRC e filtros de matching | spotDL `spotify-downloader` @ `cd4a420` (2026-07-20, MIT) | F08, F15 |
| E2 | Casos de borda da sincronização de playlists | spotDL @ `cd4a420` | F11 |
| E3 | Importação de playlists (Deezer sem chave, Spotify com credenciais do usuário) | spotDL @ `cd4a420` + testes diretos nas APIs | F15 |
| E4 | Provedor de PO token (bgutil) como ferramenta externa | `Brainicism/bgutil-ytdlp-pot-provider` release `2.0.0` (GPL-3.0) — **usado como programa externo, não reimplementado** | F02, F04 |
| E5 | Seguir artistas, "Faltando", qualidade-alvo | Lidarr @ `da7b4df` (2026-09-13, GPL-3.0) — só conceitos | F15 |
| E6 | Fontes de letras — decisão | Metrolist @ `e23a4d6`, SimpMusic @ `7621407` | F09 (decisão) |
| E7 | Rota alternativa de extração no Android | Metrolist @ `e23a4d6` (GPL-3.0) | A0 |

Todos os testes práticos citados foram executados em 2026-10-01 na máquina do usuário.
