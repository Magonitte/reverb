# E6 — Fontes de letras: decisão

## O que foi encontrado
Clientes de YouTube Music estudados usam, além do LRCLIB:
- **Servidores comunitários "LyricsPlus"/"BetterLyrics"/"Paxsenix"** — oferecem letras
  sincronizadas **palavra por palavra** (formato TTML). São espelhos mantidos por voluntários;
  o próprio cliente mantém uma lista de 4–6 espelhos porque eles caem ou atingem limites
  diários. O conteúdo vem de catálogos licenciados de serviços pagos.
- **KuGou** (API não oficial de um serviço chinês) — bom para catálogo asiático.
- **Letras do próprio YouTube Music** (texto simples, via API interna).

## Decisão para o Reverb
- **Manter o LRCLIB como única fonte** (aberto, comunitário, com API pública documentada).
- **Não** integrar os espelhos LyricsPlus/BetterLyrics/Paxsenix nem a API não oficial do KuGou:
  instáveis e de origem duvidosa quanto a licenciamento.
- Adotar duas boas ideias de comportamento:
  1. **Registro de provedores com fallback** (lista ordenada; lembrar o último que funcionou)
     — já compatível com a arquitetura de provedores.
  2. **Formato LRC "enhanced"** (marcas `<mm:ss.xx>` por palavra) — se o LRCLIB devolver letras
     nesse formato, preservar ao gravar o `.lrc` (não "achatar" para linhas).
- Reavaliar se o usuário pedir explicitamente.
