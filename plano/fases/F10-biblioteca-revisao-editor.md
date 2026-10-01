# F10 — Biblioteca, revisão de metadados, editor de tags, importação e vigia de pasta

**Objetivo:** gerenciar o que já foi baixado (e arquivos de fora): buscar, filtrar, abrir,
editar tags, corrigir metadados duvidosos, reorganizar, excluir com segurança e manter a
biblioteca sincronizada com a pasta de música.

**Pré-requisito:** F09. **Design:** §3.5–3.7. **Arquitetura:** §5, §12, §13, §15.

## Tarefas

1. **Repo da biblioteca**: `library_list(LibraryQuery { text?, artist?, album?, format?,
   date_range?: today|week|month|all, needs_review?, missing?, sort: added_desc|title|artist|album,
   offset, limit }) -> Page<LibraryItem>` (FTS para `text`, prefixo `termo*`), `library_get`,
   `library_artists`/`library_albums` (para filtros e grade de álbuns), `library_cover(id, size)`.
2. **Ações**: `library_reveal`, `library_open_file` (opener), `library_delete(ids, delete_files)`
   (com arquivos ⇒ lixeira do sistema via crate `trash`; também `.lrc` irmão), `library_clear`
   (só registros, com confirmação na UI — paridade "Limpar histórico"), "Baixar novamente"
   (`enqueue` com `allow_duplicate: true` e mesmo perfil).
3. **Importação**: `library_import(paths)` aceita arquivos e pastas (recursivo), extensões
   mp3/m4a/opus/ogg/flac/wav, lê tags com `read_tags` + `probe`, insere `origin=import`
   (ignora já existentes por `file_path`). Progresso por evento `library://import-progress`.
4. **Reexame e vigia**: `library_rescan()` marca `missing=1` para arquivos que sumiram e importa
   novos em `outputDir`; vigia com `notify` + debouncer (2 s) quando `watchLibrary` (ignorar
   arquivos sendo escritos pelo próprio app: lista de caminhos "em movimento" no `organize`).
5. **Revisão**: `review_list()`, `review_apply(id, candidate | manual TrackTags)` ⇒ reescreve tags
   (capa do candidato também), reorganiza o arquivo pelo modelo (se `autoOrganize`), atualiza
   biblioteca/FTS, `needs_review=0`; `review_dismiss(id)`.
6. **Editor de tags**: `tags_read(path)`, `tags_write(path, TrackTags, reorganize: bool)`,
   `pick_audio_file`, `pick_image_file`, capa por URL (`artwork_fetch(url)` com as mesmas
   regras de recorte), `metadata_search(query)` (F08) para preencher. Funciona também para
   arquivos fora da biblioteca.
7. **UI**: telas Biblioteca (lista virtualizada + grade de álbuns, filtros, seleção múltipla,
   ações em lote, importar, reexaminar, `<th>` com `text-left`), Revisar (atalhos J/K/1–5/Enter/M),
   Editor de tags (design §3.7). "Recentes" do Início passa a vir da biblioteca. Badge de
   revisão na sidebar. Toda a lógica também no mock (com 5.000 itens gerados para o cenário `big`).

## Testes de verificação

| # | Teste | Ferramenta |
|---|-------|-----------|
| T1 | Busca FTS sem acento acha com acento ("musica" ⇒ "Música"), prefixo ("nev" ⇒ "Never…"), filtros combinados, ordenação, paginação estável | cargo test |
| T2 | Importar pasta com 6 arquivos sintéticos taggeados (formatos variados, subpastas) ⇒ 6 itens com campos corretos; reimportar não duplica | cargo test |
| T3 | Reexame: apagar 1 arquivo e criar outro ⇒ `missing=1` no apagado, novo importado | cargo test |
| T4 | Vigia: com o serviço ativo, copiar arquivo novo para a pasta ⇒ aparece na biblioteca em ≤ 5 s; arquivo escrito pelo próprio pipeline **não** é importado em duplicidade | cargo test |
| T5 | `library_delete` com arquivos ⇒ arquivo some da pasta (está na lixeira) e o registro sai; sem arquivos ⇒ arquivo permanece | cargo test (Windows e Linux CI) |
| T6 | `review_apply` com candidato ⇒ tags reescritas (releitura), arquivo movido para o novo caminho do modelo, FTS atualizado, `needs_review=0` | cargo test |
| T7 | `tags_write` em arquivo fora da biblioteca não cria registro; com `reorganize` em arquivo da biblioteca move e atualiza | cargo test |
| T8 | UI Biblioteca: com 5.000 itens o DOM tem < 200 linhas renderizadas; busca filtra; filtros de data (paridade hoje/semana/mês/tudo); ações em lote chamam os comandos certos; diálogo "Limpar biblioteca" | Vitest + Playwright |
| T9 | UI Revisar: atalhos de teclado funcionam; aplicar remove o item da lista; badge da sidebar atualiza | Vitest + Playwright |
| T10 | UI Editor: carregar, editar, trocar capa por arquivo (mock), "Buscar metadados" preenche campos, salvar chama `tags_write` | Vitest + Playwright |
| T11 | axe e screenshots das telas novas (inspecionadas) | Playwright |
| T12 | **App real**: importar uma pasta de teste com 2 arquivos sintéticos ⇒ aparecem na Biblioteca; editar o título de um pelo editor ⇒ releitura via `reverb-cli tags read <arquivo>` mostra o novo título (adicionar esse subcomando ao CLI) | WebdriverIO |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run e2e:app` ✔

## Armadilhas
- `trash` no Linux CI: o runner tem `$HOME/.local/share/Trash`; se o crate falhar por falta de
  ambiente, o teste deve criar o diretório antes (não pular).
- O vigia dispara vários eventos por arquivo; use o debouncer e confira se o arquivo já está
  completo (tamanho estável por 1 s) antes de ler tags.
