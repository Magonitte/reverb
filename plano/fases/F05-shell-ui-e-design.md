# F05 — Shell da UI, design system, temas, i18n e backend mock

**Objetivo:** a "casca" do app pronta e fiel à identidade visual: janela, titlebar, navegação,
todas as rotas (com estados vazios), componentes base, temas claro/escuro, transparência,
idiomas, atalhos, e um backend mock completo para desenvolver e testar a UI sem o Rust.

**Pré-requisito:** F04. **Design:** `plano/02-design.md` inteiro. **Arquitetura:** §3, §15.

## Tarefas

1. **Estudo da referência** (antes de codar): abrir `referencias/design/reverb-vinyl-glassmorphism.html`
   no Playwright (`file://`), tirar screenshots em 1366×768 e 1920×1080, e listar no PROGRESS.md
   os tokens/componentes/estados encontrados além dos de `02-design.md` §1. Ver também
   `referencias/design/tela1..6.png`.
2. **Estilos**: `tokens.css` (escuro + claro + modo reduzido), `tailwind.css` (`@import "tailwindcss"`
   + `@theme`), `base.css` (tudo em `@layer base`: reset de foco, fundo com "sulcos" de vinil
   sutis usando `--groove-*`, scrollbar fina, seleção de texto). Fonte de fallback para Linux:
   `@fontsource-variable/inter`.
3. **Layout**: `Titlebar` (região de arrasto `data-tauri-drag-region`, botões minimizar/
   maximizar/fechar chamando a API de janela do Tauri — no mock são no-op), `Sidebar` (itens de
   §3 do design, tooltips, badges de contagem e ponto de atualização vindos das stores),
   `BottomNav` (< 640 px), `ScreenOutlet` (transições com `motion`, reduzidas com
   `prefers-reduced-motion`), `ToastHost`, `CommandBarOverlay` (Ctrl+K — nesta fase só abre,
   foca e fecha; a lógica vem na F07).
4. **Rotas** (`routes.tsx`, `HashRouter`, `React.lazy` por tela): todas as de §3 do design, cada
   uma com título, subtítulo e `EmptyState` com ícone e texto i18n.
5. **Componentes base** (§4 do design) com todos os estados, cada um com teste.
6. **Temas e transparência**: hook `useAppearance` aplica `data-theme` e `data-transparency`
   conforme `settings.theme`/`settings.transparency` e plataforma (`app_info.platform`);
   `system` segue `matchMedia('(prefers-color-scheme: dark)')` com listener.
7. **i18n**: `src/locales/pt-BR/translation.json` e `en/translation.json` com namespaces por tela
   (`nav`, `home`, `preview`, `collection`, `activity`, `library`, `review`, `tagEditor`,
   `playlists`, `settings`, `onboarding`, `errors`, `common`, `profiles`, `stages`, `updater`,
   `tools`). Use `referencias/translation.pt-BR.antigo.json` como referência de tom e termos.
   Troca de idioma pela configuração, sem recarregar.
8. **Atalhos** (`src/lib/shortcuts.ts`): tabela do design §5 + handler global; função pura
   `resolveShortcut(event, context) -> Action | null` testável.
9. **Backend mock** (`src/lib/ipc/mock/`): implementação em memória de **todos** os comandos
   registrados no Rust até aqui (settings, tools, fila com simulação temporizada de
   progresso por estágios, analyze/search com dados das fixtures FX1–FX4 convertidas para TS),
   emitindo os mesmos eventos. Cenários selecionáveis por query string (`?scenario=empty|busy|errors|heal`)
   para testes. Cada fase seguinte estende o mock junto com os comandos reais.
9b. **Contrato de IPC verificável**: `src/lib/ipc/api.ts` exporta `COMMANDS` (lista de nomes);
   `scripts/check-ipc.mjs` extrai os nomes do `tauri::generate_handler![…]` em
   `src-tauri/src/lib.rs` e as chaves de handlers do mock, e **falha** se os três conjuntos
   diferirem. Entra no `npm run verify` a partir desta fase (toda fase seguinte que criar um
   comando precisa criá-lo nos três lugares).
10. **Stores** zustand: `ui` (sidebar, overlay, toasts), `jobs` (aplica `job://updated`,
    `job://removed`, `queue://state`), `tools`, `updater` (estado vazio por enquanto),
    `heal`. Assinatura de eventos centralizada em `src/lib/ipc/events.ts`.
11. Janela por plataforma: `tauri.windows.conf.json` não se aplica a janelas criadas em código —
    portanto, ao criar a janela no `setup()`, no Windows usar `transparent(true)` e aplicar
    efeito Mica (`WebviewWindowBuilder::effects` / `window.set_effects` com `Effect::Mica`);
    no Linux `transparent(false)`. Fundo do `body` explícito em `reduced`.

## Testes de verificação

| # | Teste | Ferramenta |
|---|-------|-----------|
| T1 | Cada componente base: renderiza, estados (disabled/loading), acessibilidade básica (role/aria), interação (Toggle alterna `aria-checked`; Tabs navegam com setas; Dialog prende foco e fecha com Esc devolvendo o foco; Toast some após 4 s com timers falsos) | Vitest |
| T2 | `resolveShortcut`: tabela completa do design §5, incluindo "Espaço não pausa dentro de input/botão" e "Ctrl+V dentro de input não é interceptado" | Vitest |
| T3 | Tema: `dark`/`light`/`system` aplicam `data-theme` correto; `system` reage à mudança do media query | Vitest |
| T4 | Transparência: `auto` + plataforma linux ⇒ `reduced`; windows ⇒ `full` | Vitest |
| T5 | i18n: trocar idioma muda textos da sidebar; `check:i18n` passa; lint sem textos literais | Vitest + scripts |
| T6 | **Guarda da cascata CSS** (lição do projeto antigo): um elemento com `class="pl-10 text-accent"` tem `padding-left: 40px` e a cor do token de acento calculados | Playwright |
| T7 | Navegação: clicar em cada item da sidebar e usar cada atalho leva à rota certa; título visível; sem erros no console | Playwright |
| T8 | Responsividade: em 900×600, 1366×768, 1920×1080 e 390×844 não há rolagem horizontal (`scrollWidth <= clientWidth`); em 390×844 a BottomNav aparece e a sidebar não | Playwright |
| T9 | Acessibilidade: `@axe-core/playwright` em todas as rotas, temas escuro e claro ⇒ zero violações `serious`/`critical` | Playwright |
| T10 | Screenshots de base (`toHaveScreenshot`) de cada rota em 1366×768 nos dois temas — gerar com `--update-snapshots` nesta fase, **abrir e inspecionar as imagens** comparando com as screenshots da referência; registrar no PROGRESS.md as diferenças encontradas e corrigidas; commitar as bases | Playwright |
| T11 | Mock: cenário `busy` mostra badge de contagem na sidebar; cenário `heal` mostra banner | Playwright |
| T12 | App real compila e o autoteste headless continua passando | `cargo build -p reverb` + `verify-app-headless` |
| T13 | `check:ipc` passa; e falha num teste com um comando a mais só no mock (testar a função do script com entradas de exemplo) | Vitest + script |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ (T6–T11) · T12 ✔ · **checagem visual registrada** (T10)

## Armadilhas
- Tailwind 4 não lê `tailwind.config.*` sem `@config`; aqui não usamos config — tokens via `@theme`.
- Estilos globais fora de `@layer base` vencem as utilidades (T6 existe para pegar isso).
- `backdrop-filter` em muitos elementos pesa: aplicar só em superfícies grandes (cards, sidebar, overlays).
- Screenshots variam por fonte/SO: os testes visuais rodam só no Windows (CI Windows e local).
