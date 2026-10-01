# E5 — Seguir artistas, "Faltando" e qualidade-alvo

Conceitos de gerenciadores de coleção (estilo Lidarr), adaptados ao Reverb. Nenhum código foi
usado — apenas o modelo de comportamento.

## 1. Seguir artista
- O usuário segue um artista (a partir da Biblioteca, de um resultado de busca ou de URL do
  Deezer `deezer.com/<lang>/artist/<id>`).
- **Fonte da discografia** (validado 2026-10-01): Deezer, sem chave.
  - `GET https://api.deezer.com/search/artist?q=<nome>&limit=5` ⇒ escolher o artista (UI mostra
    foto/fãs para o usuário confirmar quando houver ambiguidade).
  - `GET https://api.deezer.com/artist/{id}/albums?limit=100` (paginar por `next`) ⇒ cada item tem
    `id, title, release_date, record_type` (`album` | `ep` | `single` | `compile`), `cover_xl`.
    Exemplo real (Rick Astley): 59 lançamentos — 17 álbuns, 13 EPs, 29 singles.
  - `GET https://api.deezer.com/album/{id}/tracks` ⇒ faixas com `isrc`, `track_position`, `disk_number`.
  - Cada faixa é casada no YouTube Music via **E1 (ISRC)** — validado: ISRC da faixa 1 de
    "Whenever You Need Somebody" (`GBARL9300135`) ⇒ `lYBUbBu4W08`.
- **Opções de monitoramento** (por artista):
  - *O que baixar agora*: `all` (toda a discografia filtrada) | `latest` (só o lançamento mais
    recente) | `none` (só acompanhar).
  - *Lançamentos novos*: `all` (baixar automaticamente) | `notify` (só avisar e mostrar em
    "Faltando") | `none`.
  - *Tipos incluídos* (perfil de metadados): álbum, EP, single (padrão: todos os três);
    excluir por padrão títulos que indiquem **ao vivo, coletânea (`compile`), remix, edição
    deluxe duplicada** (heurística por palavras no título: live, ao vivo, remix(es), deluxe,
    anniversary, remaster(ed) — quando já existe a versão base do mesmo álbum).
  - Perfil de saída (§7) e pasta (opcional).
- **Verificação**: a cada 24 h (configurável), buscar lançamentos com `release_date` mais novo
  que a última verificação; aplicar a regra de "lançamentos novos".
- **Deduplicação de faixas**: singles cujas faixas (mesmo ISRC) também estão num álbum
  monitorado ⇒ baixar só a do álbum (preferência: álbum > EP > single).

## 2. "Faltando"
- Visão por artista seguido: lançamentos monitorados ⇒ faixas esperadas (por ISRC/posição) ×
  faixas na biblioteca (por `isrc` gravado na biblioteca, ou por `source_id` do vídeo casado).
- Mostra: álbuns completos ✓, incompletos (x de y), ausentes; ação "Baixar faltantes".

## 3. Qualidade-alvo (cutoff)
- Conceito: um perfil de qualidade tem uma **lista ordenada** de qualidades aceitas e um
  **alvo**. Abaixo do alvo e com "permitir melhorias" ligado ⇒ o item é candidato a melhoria
  automática quando uma fonte melhor aparecer. No alvo ou acima ⇒ nunca mais trocar.
- No Reverb: configuração `qualityTargetKbps` (padrão 0 = desligado; opções 0, 160, 256) e
  `autoUpgrade` (bool, padrão false). Ex.: alvo 256 + Premium configurado ⇒ a verificação periódica
  (junto com a de artistas, a cada 24 h) roda o `upgrade_scan` (F13) só para itens abaixo do alvo
  e enfileira melhorias automaticamente, respeitando a pausa entre faixas.
- "Rebaixamento" nunca acontece: só substitui se a nova fonte for **estritamente** melhor
  (abr ≥ atual + 40, regra da F13).

## Testes de aceitação
- Unit: filtro de tipos/títulos (tabela ≥ 10 casos); deduplicação single×álbum por ISRC;
  "Faltando" (álbum completo, incompleto, ausente); regra de qualidade-alvo (abaixo/igual/acima,
  autoUpgrade ligado/desligado); verificação de novos lançamentos com relógio injetado.
- Respostas gravadas do Deezer (artista, álbuns paginados, faixas do álbum).
- Rede: seguir "Rick Astley" com `latest` e tipos = álbum ⇒ enfileira as faixas do álbum mais
  recente, todas casadas por ISRC (≥ 80 % com confiança ≥ 0.85) — rodar com `max` de 3 faixas
  (parâmetro de teste) para não baixar o álbum inteiro.
