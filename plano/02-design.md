# 02 — Design: sistema visual, telas e interação

Referência visual: `referencias/design/reverb-vinyl-glassmorphism.html` (abra no navegador /
Playwright) e `referencias/design/DESIGN-HANDOFF.md`. Screenshots do app antigo:
`referencias/design/tela1..6.png`. Identidade: **vinil + vidro fosco**, escuro e quente,
acento âmbar. O redesenho moderniza a navegação (menos telas, barra de comando) mas **mantém a
identidade**.

## 1. Tokens (copiar para `src/styles/tokens.css`)

### Tema escuro (padrão — extraído da referência)
```css
--bg-deep:#0f0f14; --bg-base:#18181f; --bg-warm:#1c1b1e;
--glass-bg:rgba(28,27,32,.72); --glass-bg-hover:rgba(36,34,42,.78); --glass-bg-elevated:rgba(32,30,38,.82);
--glass-border:rgba(255,255,255,.07); --glass-border-hover:rgba(255,255,255,.13); --glass-border-focus:rgba(245,158,75,.35);
--glass-blur:24px; --glass-blur-heavy:40px; --acrylic-bg:rgba(18,17,22,.78); --acrylic-blur:32px;
--accent:#f59e4b; --accent-hover:#f6b06a; --accent-pressed:#e08a35; --accent-muted:rgba(245,158,75,.12);
--accent-glow:rgba(245,158,75,.25); --accent-soft:rgba(245,158,75,.08); --gold:#e2a854; --gold-muted:rgba(226,168,84,.15);
--fg:#f2efe9; --fg-secondary:#cfccc6; --fg-muted:#a09d97; --fg-dim:#706d68;
--success:#5cdb8b; --warning:#f5c842; --error:#f8716b; --info:#6db3f5;
--groove-1:rgba(245,158,75,.04); --groove-2:rgba(226,168,84,.03);
--font-display:'Segoe UI Variable Display','Segoe UI',Inter,system-ui,sans-serif;
--font-body:'Segoe UI Variable Text','Segoe UI',Inter,system-ui,sans-serif;
--font-mono:'Cascadia Code','JetBrains Mono',ui-monospace,monospace;
--space-1:4px; --space-2:8px; --space-3:12px; --space-4:16px; --space-5:20px; --space-6:24px;
--space-8:32px; --space-10:40px; --space-12:48px; --space-16:64px;
--radius-sm:6px; --radius-md:10px; --radius-lg:14px; --radius-xl:20px; --radius-full:999px;
--shadow-sm:0 1px 2px rgba(0,0,0,.4); --shadow-md:0 4px 16px rgba(0,0,0,.5); --shadow-lg:0 8px 40px rgba(0,0,0,.6);
--shadow-glow:0 0 28px var(--accent-glow); --shadow-glow-sm:0 0 12px rgba(245,158,75,.18);
--titlebar-h:36px; --sidebar-w:58px;
--ease-out:cubic-bezier(.16,1,.3,1); --ease-in-out:cubic-bezier(.65,0,.35,1); --dur-fast:140ms; --dur-normal:240ms;
```
Antes de codar, abra a referência e extraia também os tokens/estilos não listados acima
(tamanhos de fonte, durações extras, estilos de componentes) — registre em `tokens.css`.

### Tema claro (derivado; contraste AA obrigatório — verificado por axe)
```css
--bg-deep:#efebe5; --bg-base:#f7f4ef; --bg-warm:#f3efe9;
--glass-bg:rgba(255,255,255,.72); --glass-bg-hover:rgba(255,255,255,.85); --glass-bg-elevated:rgba(255,255,255,.9);
--glass-border:rgba(20,16,10,.09); --glass-border-hover:rgba(20,16,10,.16); --glass-border-focus:rgba(196,106,22,.45);
--acrylic-bg:rgba(250,247,242,.82);
--accent:#c46a16; --accent-hover:#b05e12; --accent-pressed:#97500f; --accent-muted:rgba(196,106,22,.12);
--accent-glow:rgba(196,106,22,.22); --accent-soft:rgba(196,106,22,.07); --gold:#a8741f; --gold-muted:rgba(168,116,31,.14);
--fg:#1d1a16; --fg-secondary:#3b362f; --fg-muted:#5f584e; --fg-dim:#857d71;
--success:#1f8a4c; --warning:#9a6a00; --error:#c23b33; --info:#2a6fb5;
--shadow-sm:0 1px 2px rgba(0,0,0,.08); --shadow-md:0 4px 16px rgba(0,0,0,.1); --shadow-lg:0 8px 40px rgba(0,0,0,.14);
```
Seleção: atributo `data-theme="dark|light"` no `<html>`; `system` segue `prefers-color-scheme`
(ouvir mudanças).

### Transparência
`data-transparency="full|reduced"` no `<html>`. `auto` ⇒ `reduced` no Linux (WebKitGTK é lento
com `backdrop-filter`) e `full` no Windows. Em `reduced`: sem `backdrop-filter`, fundos de vidro
viram cores sólidas equivalentes (`--glass-bg` com alfa ≥ .96). No Windows a janela é
transparente com efeito Mica; no Linux a janela não é transparente. Como a janela é criada em
código no `setup()` (arquitetura §17), isso é configurado no builder da janela conforme o SO
(F05, tarefa 11) — não nos arquivos `tauri.*.conf.json`.

### Tailwind 4
`src/styles/tailwind.css`: `@import "tailwindcss";` + `@theme { … }` mapeando os tokens para
utilidades (cores `bg`, `fg`, `accent`…, raios, sombras). `src/styles/base.css`: tudo dentro de
`@layer base { … }`. Ordem de import em `main.tsx`: `tokens.css` → `tailwind.css` → `base.css`.

## 2. Estrutura da janela

```
┌──────────────────────────────────────────────────────────────────────┐
│ ◉ Reverb                         (área de arrastar)        ─  ▢  ✕   │ Titlebar 36px
├────┬─────────────────────────────────────────────────────────────────┤
│ ⌂  │                                                                 │
│ ♪  │                 Conteúdo da rota (ScreenOutlet)                 │
│ ≡  │                 transição: fade + 8px, 240ms ease-out           │
│ ⇅³ │                                                                 │
│ ✓² │                                                                 │
│    │                                                                 │
│ ⚙• │                                                                 │
└────┴─────────────────────────────────────────────────────────────────┘
Sidebar 58px (trilho de ícones com tooltip). ³ = contador de ativos. ² = itens para revisar.
• = ponto de "atualização disponível".
```
Abaixo de 640 px de largura (preparação para Android): sidebar some, aparece **BottomNav** com
5 itens (Início, Biblioteca, Playlists, Atividade, Mais→Revisar/Configurações) e a titlebar
customizada some.

## 3. Navegação e telas

| Rota | Tela | Ícone (lucide) | Atalho |
|------|------|----------------|--------|
| `/` | Início | `House` | Ctrl+D |
| `/library` | Biblioteca | `Library` | Ctrl+H (paridade "Histórico") |
| `/playlists`, `/playlists/:id` | Playlists sincronizadas | `ListMusic` | — |
| `/activity` | Atividade (fila + concluídos + falhas) | `ArrowDownUp` | Ctrl+Q (paridade "Fila") |
| `/review` | Revisar metadados | `ListChecks` | — |
| `/tag-editor?path=` | Editor de tags | (acessado por ações) | — |
| `/settings/:tab?` | Configurações | `Settings` | Ctrl+, |
| `/onboarding` | Primeiro uso | — | — |

### 3.1 Início
- **Barra de comando** grande no topo (o mesmo componente do Ctrl+K, aqui embutido):
  placeholder "Cole um link ou pesquise uma música…". Detecta tipo (§14 da arquitetura):
  link de vídeo ⇒ abre **Preview**; coleção ⇒ abre **Coleção**; texto ⇒ resultados de busca
  com abas "YouTube Music" | "YouTube" (F14 adiciona "Internet Archive", "Jamendo").
- **Ações rápidas** (cards): Colar e baixar (usa área de transferência), Analisar playlist,
  Editar tags de um arquivo, Abrir pasta de músicas.
- **Em andamento**: até 3 jobs ativos (mini-cards com disco de vinil girando + progresso) e link "Ver tudo".
- **Recentes**: grade de 8 capas dos últimos itens da biblioteca.
- **Faixa de playlists**: "3 playlists sincronizadas · próxima em 2 h" (F11).

### 3.2 Preview (painel deslizante à direita ou rota modal; fecha com Esc)
Capa grande com motivo de vinil, título, artista, duração, canal; selos: tipo de conteúdo
(Música/Outro), **qualidade real da fonte** ("Fonte: Opus 129 kbps", "Premium · AAC 256"),
"Já baixada" se duplicata. Card **"Versão oficial disponível"** (F08) com toggle pré-marcado.
Chips de perfil (§7 da arquitetura) com aviso nos perfis que recodificam. Opções: metadados,
letra, capa, SponsorBlock, organizar, dividir por capítulos (F13, só se houver ≥ 2 capítulos).
Pasta de destino (com botão escolher). Botão "Pré-visualizar metadados" (F08) ⇒ mostra o
resultado do pipeline com nota de confiança e permite **editar antes de baixar** (paridade com
MetadataPreviewModal/MetadataEditModal do antigo). Botões: **Baixar agora** (vai para o topo da
fila) e **Adicionar à fila** (fim).

### 3.3 Coleção (playlist/álbum/canal)
Cabeçalho: título, canal, nº de faixas, duração total. Lista virtualizada com checkbox,
miniatura, título, duração, marca "já baixada". Filtro por texto, Selecionar tudo/nenhum,
"Selecionar só as novas". Perfil. Botões: **Baixar selecionadas (N)** e **Sincronizar esta
playlist** (F11 — abre diálogo: intervalo, pasta, perfil, limitar às N primeiras, remover
excluídas, gerar .m3u8). Aviso de limite da fila (paridade).

### 3.4 Atividade
Abas: Em andamento | Concluídos | Falhas. Contadores "2 ativos · 5 pendentes · 12 concluídos"
(paridade). Controles: Pausar/Retomar fila, Cancelar todos, Limpar concluídos.
Item: miniatura, título/artista, rótulo do estágio, barra de progresso (cor por estágio),
velocidade/ETA, "Tentativa 2/3", erro traduzido + "Tentar novamente", alça de arrastar
(pendentes), remover, abrir pasta (concluídos). **Banner de autocura** quando `heal://state`
ativo: "O YouTube mudou algo — atualizando o yt-dlp…". **Aviso de limite** (paridade) quando a
fila passar de 90 % do `queueLimit`: "A fila está quase cheia (450/500)".

### 3.5 Biblioteca
Busca instantânea (FTS), filtros (formato, data: hoje/semana/mês/tudo — paridade; artista;
álbum; "precisa revisão"; "arquivo ausente"), ordenação. Visualização **Lista** (tabela:
capa, faixa, artista, álbum, formato, data, ações — `<th>` com `text-left`) e **Álbuns** (grade).
Ações por item: Abrir pasta, Abrir arquivo, Editar tags, Baixar novamente (paridade),
Melhorar qualidade (F13), Verificar lossless (F14), Remover da biblioteca, Excluir arquivo
(lixeira do sistema). Seleção múltipla com as mesmas ações em lote. "Limpar biblioteca" =
remover registros, **não** arquivos (confirmação — paridade "Limpar histórico").
Botões: Importar arquivos/pasta, Reexaminar pasta.

### 3.6 Revisar
Lista de itens com `needs_review`. Card comparando "Atual" × candidatos (capa, título,
artista, álbum, ano, duração, % de confiança, fonte). Ações: Aplicar candidato, Editar
manualmente (abre editor de tags), Manter como está. Teclado: J/K navega, 1–5 escolhe
candidato, Enter aplica, M mantém.

### 3.7 Editor de tags (paridade TagEditor/Metadata)
Abre por ação da biblioteca ou "Abrir arquivo…". Campos: título, artista, álbum, artista do
álbum, ano, gênero, faixa/total, disco, letra (textarea; indica se é sincronizada), capa
(trocar por arquivo, por URL ou por resultado de busca). "Buscar metadados" (`metadata_search`)
preenche os campos com o candidato escolhido. Salvar ⇒ grava tags e, se `autoOrganize`,
oferece reorganizar o arquivo.

### 3.8 Playlists
Lista de sincronizações: capa, título, nº de faixas, última/próxima sincronização, status,
toggle ativo, "Sincronizar agora", editar, excluir (perguntar se apaga arquivos). Detalhe
`/playlists/:id`: faixas com estado (baixada, na fila, falhou, removida da playlist).

### 3.9 Configurações (abas)
- **Geral**: iniciar com o sistema, iniciar minimizado, minimizar/fechar para a bandeja,
  notificações; aparência: tema, idioma, transparência.
- **Downloads**: pasta de destino, modelo de nomes com **prévia ao vivo** (exemplo renderizado),
  organizar automaticamente, perfil padrão, downloads simultâneos, limite de velocidade,
  limite da fila, tentativas, SponsorBlock (+categorias), capítulos, pausa entre faixas de playlist.
- **Metadados**: buscar metadados, extrair do título, preferir versão oficial, limites de
  confiança (sliders), modo offline, capa, `cover.jpg` na pasta, letras, `.lrc`, ReplayGain;
  chaves opcionais (AcoustID, Spotify, Discogs, Jamendo) com status e botão "Testar"
  (paridade ApiConfigSection) — valores mascarados.
- **Integração**: monitorar área de transferência, atalho global (gravador de teclas),
  bookmarklet (mostrar código + Copiar + instrução), testar link `reverb://`, cookies do
  navegador (+ "Testar cookies"; recomendar Firefox; aviso sobre Chrome no Windows).
- **Atualizações**: conforme o mockup abaixo.
- **Avançado**: runtime JS, canal do yt-dlp, diagnóstico (rodar agora, último resultado),
  exportar logs, backup/restauração, abrir pasta de dados, modo portátil (informativo),
  restaurar padrões.

Mockup da aba Atualizações:
```
┌─ Atualizações ──────────────────────────────────────────┐
│  Reverb        3.0.0 → 3.1.0 disponível                  │
│  • notas da versão (corpo do release)                    │
│                                        [Atualizar agora] │
│  Ferramentas                                             │
│  yt-dlp   2026.08.19   ✓ atualizado     [Reverter]       │
│  Deno     2.9.7        ✓ atualizado                      │
│  FFmpeg   (data)       ✓ atualizado                      │
│  [Verificar atualizações]   Canal yt-dlp: (•) Estável    │
│                                           ( ) Nightly    │
│  [x] Verificar o app automaticamente                     │
│  [x] Atualizar ferramentas automaticamente               │
└──────────────────────────────────────────────────────────┘
```
Estados do bloco do app: ocioso, verificando, atualizado, disponível, baixando (barra %),
pronto para reiniciar, erro (com "Tentar de novo").

### 3.9b Importação, artistas seguidos e "Faltando" (F15)
- **Coleção em modo importação** (URL do Deezer/Spotify): mesma tela da Coleção com uma coluna
  de casamento por faixa — ✓ (com % de confiança e selo "ISRC" quando casou por ISRC),
  ⚠ "revisar", ✗ "não encontrada" — e filtros por estado. Barra de progresso do casamento.
- **Biblioteca › Artistas seguidos**: grade de cards (foto, nome, "12 lançamentos · 9 completos",
  próxima verificação, menu: verificar agora, editar, deixar de seguir).
- **Biblioteca › Faltando**: agrupado por artista ⇒ lançamento (capa, tipo, ano, "7 de 10
  faixas"), ação "Baixar faltantes" por lançamento e em lote.
- **Diálogo "Seguir artista"**: o que baixar agora (tudo / só o mais recente / nada),
  lançamentos novos (baixar / só avisar / ignorar), tipos (álbum, EP, single), "excluir ao vivo,
  remixes, edições duplicadas e coletâneas", perfil e pasta.

### 3.10 Onboarding (paridade + novo)
Passos: Boas-vindas → Pasta de músicas → Ferramentas (instala yt-dlp/FFmpeg/Deno com barras
de progresso; detecta Node/Deno do sistema e oferece usar; erro ⇒ "Tentar de novo") →
Metadados (explica que funciona sem chaves; chaves opcionais "depois") → Integração
(área de transferência, iniciar com o sistema — opt-in) → Pronto. Não pode ser concluído sem
as ferramentas instaladas (FFmpeg é obrigatório; não existe mais o "modo rápido" do antigo).

## 4. Componentes base (`src/components/ui/`)

Button (variantes primary/secondary/ghost/danger; tamanhos; loading), IconButton, Card (vidro),
Input, SearchInput, Select, Toggle (`role="switch"`), Checkbox, Chip/ProfileChips, Badge,
StatusDot, ProgressBar (`role="progressbar"`, cor por estágio), Dialog (focus trap, Esc,
`aria-modal`), Sheet (painel lateral), Toast (fila, auto-fechar 4 s, ação opcional), Tooltip,
Tabs (setas do teclado), Skeleton, EmptyState, VinylDisc (gira durante download; para com
reduced-motion), Slider, Kbd, ShortcutRecorder, VirtualTable, ConfirmDialog.
Estados obrigatórios: hover, focus-visible (anel com `--glass-border-focus`), pressed,
disabled, loading.

## 5. Atalhos de teclado (paridade + novos)

| Atalho | Ação | Regra |
|--------|------|-------|
| Ctrl+K | Abrir barra de comando (overlay) | global |
| Ctrl+V (fora de campos de texto) | Ir ao Início e colar na barra de comando | não interceptar dentro de inputs |
| Ctrl+D | Início | |
| Ctrl+Q | Atividade | |
| Ctrl+H | Biblioteca | |
| Ctrl+, | Configurações | |
| Espaço | Pausar/retomar fila | ignorar em inputs, botões e links |
| Esc | Fechar diálogo/painel aberto; senão limpar barra de comando | |
| Enter | Enviar barra de comando | quando focada |
(Ctrl = Cmd no macOS, irrelevante aqui.) Implementar em `src/lib/shortcuts.ts` com tabela
testável.

## 6. Movimento e acessibilidade
- Transições 140–240 ms com `--ease-out`; `prefers-reduced-motion` ⇒ sem animações de
  deslocamento/rotação.
- Contraste AA nos dois temas; foco sempre visível; todos os controles alcançáveis por teclado;
  `aria-label` em botões só-ícone (texto via i18n); `aria-live="polite"` para toasts e progresso
  agregado.

## 7. Responsividade
Larguras de teste: 900×600 (janela mínima), 1366×768, 1920×1080 e 390×844 (preparação Android).
Sem rolagem horizontal da página em nenhuma delas. Tabelas viram listas de cards < 760 px.
