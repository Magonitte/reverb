# E1 — Correspondência por ISRC e filtros de matching

## Conceito
ISRC (International Standard Recording Code) identifica **uma gravação** (12 caracteres:
2 letras de país + 3 alfanuméricos do registrante + 2 dígitos de ano + 5 dígitos;
regex `^[A-Z]{2}[A-Z0-9]{3}\d{7}$`). A mesma música pode ter ISRCs diferentes (original,
remaster, coletânea), mas um ISRC aponta para uma gravação exata.

## Fatos validados (2026-10-01)
- A busca do YouTube Music aceita o ISRC **como texto** e devolve a faixa oficial
  correspondente em primeiro lugar:
  `https://music.youtube.com/search?q=GBARL9300135#songs` ⇒ 1º resultado `lYBUbBu4W08`
  ("Never Gonna Give You Up", álbum original de 1987). O ISRC da versão de coletânea
  `GBARL0600786` devolve **outra** faixa oficial (`-aIiQj79b6Q`) — ou seja, o ISRC distingue versões.
- Deezer devolve ISRC: `GET https://api.deezer.com/track/{id}` (campo `isrc`),
  `GET https://api.deezer.com/album/{id}/tracks` (cada faixa tem `isrc`, `track_position`,
  `disk_number`) e `GET https://api.deezer.com/playlist/{id}` / `/playlist/{id}/tracks`
  (cada faixa tem `isrc`). Busca reversa: `GET https://api.deezer.com/track/isrc:{ISRC}`.
  Observação: `/album/{id}` (sem `/tracks`) **não** traz `isrc` nas faixas.
- **Correção importante:** a busca avançada combinada do Deezer
  `q=artist:"A" track:"T"` retorna **0 resultados** hoje. Funcionam: busca simples
  `q=<artista> <título>` e `q=track:"<título>"` isolado. Usar a busca simples e filtrar pela
  pontuação (§11.2 da arquitetura).
- MusicBrainz pode devolver ISRCs de uma gravação com `inc=isrcs` no lookup de recording.

## Comportamento a implementar
1. **Quando há ISRC conhecido** (de Deezer, MusicBrainz, Spotify ou playlist importada):
   a) buscar no YouTube Music por `<ISRC>` na seção de músicas (`#songs`), pegar até 3 resultados;
   b) se houver **exatamente 1** resultado que seja faixa oficial (`is_official_track` após
      analisar) ⇒ aceitar direto (confiança 1.0, fonte `isrc`);
   c) se houver vários, pontuar todos com §11.2 e aceitar o melhor se score ≥ 0.80;
   d) senão, cair na busca normal por texto (§11 passo 3).
2. **Busca normal por texto**: primeiro na seção de músicas (faixas oficiais); só se nenhum
   resultado passar, tentar a seção de vídeos. Resultados de faixa oficial ("verificados") têm
   preferência sobre vídeos com o mesmo score.
3. **Filtros eliminatórios** (antes da média), inspirados no comportamento estudado:
   - descartar resultado sem **nenhuma palavra em comum** com o título buscado (após normalização);
   - descartar se `title_sim < 0.60` ou `artist_sim < 0.70` (exceto quando a fonte é ISRC);
   - descartar se a diferença de duração for > 15 s (faixa) — clipes usam a tolerância de clipe (§11.2);
   - **palavras de versão** presentes no resultado mas **ausentes** na busca
     (live, ao vivo, remix, remaster(ed)?, acoustic, acústico, instrumental, cover, slowed, sped up,
     reverb, bass boost(ed)?, 8d, acapella, concert/show, karaokê) ⇒ penalidade de 0.15 **por palavra**
     no `title_sim` (cumulativa), além da penalidade de versão já existente.
4. **Artistas múltiplos**: se a busca tem vários artistas e o resultado só um, considerar
   correspondência se os demais artistas aparecem no **título** do resultado ou no nome único do
   artista (ex.: "A, B & C" como um só artista); `artist_sim` = fração dos artistas encontrados.
5. **Desempate**: entre resultados com diferença de score ≤ 0.08, preferir faixa oficial; depois,
   maior número de visualizações (se disponível na análise) com peso máximo de +0.05.
6. Explícito × limpo: se ambos os lados informam e divergem ⇒ −0.05.

## Casos de borda
- ISRC inválido (regex falha) ⇒ ignorar e usar texto.
- ISRC que devolve faixa de outra versão (ex.: remaster) é **aceitável** se o usuário pediu
  aquela versão (ISRC vem dela); não "corrigir" para o original.
- YouTube Music sem resultados para o ISRC (faixas regionais/novas) ⇒ texto.

## Testes de aceitação (para a fase que implementar)
- Unit: validação de ISRC (≥ 8 casos); filtros eliminatórios (tabela ≥ 12 casos); penalidade
  cumulativa de palavras de versão; artistas múltiplos (≥ 5 casos); desempate.
- Rede: `GBARL9300135` ⇒ `lYBUbBu4W08`; `GBARL0600786` ⇒ resultado diferente de `lYBUbBu4W08`;
  Deezer `track/isrc:GBARL9300135` responde com título "Never Gonna Give You Up";
  busca simples Deezer "rick astley never gonna give you up" tem o 1º resultado do artista "Rick Astley".
