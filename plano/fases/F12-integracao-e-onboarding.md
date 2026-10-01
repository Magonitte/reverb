# F12 — Integração com o sistema, onboarding e diagnóstico

**Objetivo:** o Reverb vive na bandeja, notifica, inicia com o sistema, recebe links do
navegador (`reverb://`), percebe links copiados, responde a um atalho global, guia o primeiro
uso e se autodiagnostica semanalmente.

**Pré-requisito:** F11. **Design:** §3.9 (Geral, Integração, Avançado), §3.10.
**Arquitetura:** §6 (configs F12), §15, §18.

## Tarefas

1. **Instância única + deep link**: registrar `tauri-plugin-single-instance` (feature
   `deep-link`) **como primeiro plugin**; `tauri-plugin-deep-link` com
   `plugins.deep-link.desktop.schemes = ["reverb"]`; em Linux e em debug no Windows chamar
   `register_all()`. Formato: `reverb://add?url=<urlencoded>[&profile=<id>]` ⇒ enfileira com o
   perfil (padrão se ausente) e mostra toast/notificação; `reverb://open` ⇒ mostra a janela.
   Segunda instância ⇒ foca a janela existente e repassa os argumentos. **App fechado**: o
   sistema abre o Reverb com o link como argumento ⇒ na inicialização, processar o link recebido
   (`deep_link().get_current()` e/ou argumentos de linha de comando) depois que a fila estiver
   pronta. Parser puro `parse_deep_link(&str) -> DeepLinkAction` no core.
2. **Bookmarklet**: comando `bookmarklet_code()` ⇒
   `javascript:location.href='reverb://add?url='+encodeURIComponent(location.href)`; UI com
   botão Copiar e instruções (arrastar para a barra de favoritos). `deeplink_test()` simula o
   recebimento.
3. **Bandeja** (`tray.rs`): ícone, tooltip "Reverb — N ativos", menu: título desabilitado com
   contagem, Abrir, Pausar/Retomar fila (rótulo dinâmico), Baixar link copiado (habilitado só se
   a área de transferência tiver URL suportada), Abrir pasta de músicas, Sair. Clique esquerdo
   mostra a janela. Textos do menu via i18n do lado Rust (tabela pequena pt-BR/en no core).
4. **Janela**: `closeToTray` ⇒ fechar esconde; `minimizeToTray` ⇒ minimizar esconde;
   `tauri-plugin-window-state` lembra tamanho/posição (paridade).
5. **Notificações** (`tauri-plugin-notification`, do Rust): conclusão (se
   `completionNotifications`; agrupar: mais de 3 conclusões em 10 s ⇒ uma notificação
   "N faixas baixadas"), falha permanente, resumo de sync, atualização do app disponível,
   autocura em andamento/falhou, diagnóstico semanal falhou.
6. **Autostart** (`tauri-plugin-autostart`, argumento `--minimized`): `launchAtStartup`
   liga/desliga; ao iniciar com `--minimized` e `startMinimized` ⇒ não mostra a janela.
7. **Área de transferência** (`clipboardWatch`): tarefa que lê o texto a cada 1,5 s; se for URL
   suportada **diferente da última vista** ⇒ evento `clipboard://url`; com a janela visível ⇒
   toast com "Baixar" e "Analisar"; escondida ⇒ notificação discreta + item da bandeja habilitado.
   Lógica de deduplicação pura e testável.
8. **Atalho global** (`globalShortcut`, plugin global-shortcut): ação = baixar a URL da área de
   transferência com o perfil padrão (ou avisar se não houver URL). Validação do acelerador e
   tratamento de conflito ("atalho já em uso") com mensagem i18n. Componente `ShortcutRecorder`.
   Observação no texto da UI: no Linux com Wayland atalhos globais podem não funcionar.
9. **Onboarding** (design §3.10): rota exibida quando `onboardingCompleted = false`; passos com
   validação; o passo Ferramentas usa `tools_install_missing` com progresso e permite usar
   Node/Deno do sistema; ao concluir grava `onboardingCompleted = true`.
10. **Diagnóstico** (`diagnostics_run`): ferramentas presentes e executáveis; versão do yt-dlp
    vs última; runtime JS resolvido; **teste real** `--simulate` em FX1 com o runtime (falha se
    stderr contiver "No supported JavaScript runtime"); escrita na pasta de saída; espaço livre
    (> 500 MB); banco íntegro (`PRAGMA integrity_check`); provedor de PO token (E4): modo
    (`potProvider`), instalado/versão, servidor ativo ou não, ligado pela autocura até quando.
    Resultado com itens ok/aviso/erro.
    `weeklySelfTest`: roda 1×/semana (kv), e em falha ⇒ tenta autocura de ferramentas e notifica.
11. **Logs/backup**: `logs_export()` (zip com logs redigidos + `diagnostics.json`),
    `data_export(path)` (zip com `reverb.db` via backup online do SQLite + `settings.json` sem
    segredos), `data_import(path)` (valida, faz backup do atual em `backups/`, restaura, reinicia),
    `open_data_dir`.
12. UI: abas Geral, Integração e Avançado completas; tela de diagnóstico em Avançado.

## Testes de verificação

| # | Teste | Ferramenta |
|---|-------|-----------|
| T1 | `parse_deep_link`: `add` com URL codificada, com `profile`, sem `url` (erro), URL não suportada (erro), `open`, esquema errado | cargo test |
| T2 | Deduplicação da área de transferência: mesma URL repetida não reemite; URL nova emite; texto comum ignorado | cargo test |
| T3 | Agrupamento de notificações (relógio pausado): 5 conclusões em 3 s ⇒ 1 notificação "5 faixas" | cargo test (sink de notificações falso) |
| T4 | Validação de aceleradores (`Ctrl+Shift+D` ok; `Ctrl+` inválido; vazio = desligado) | cargo test |
| T5 | Diagnóstico com ferramentas falsas: runtime ausente ⇒ erro; espaço insuficiente simulado ⇒ aviso; tudo ok ⇒ ok | cargo test |
| T6 | `logs_export`: zip não contém nenhum segredo configurado nem cookies (procurar os valores literais) | cargo test |
| T7 | `data_export` ⇒ `data_import` em outro diretório restaura biblioteca e configurações (sem segredos) | cargo test |
| T8 | UI Onboarding (mock): fluxo completo; falha de instalação ⇒ "Tentar de novo" funciona; não conclui sem ferramentas | Vitest + Playwright |
| T9 | UI Integração: bookmarklet copiado (mock da área de transferência), gravador de atalho, toggles chamam `settings_update` | Vitest + Playwright |
| T10 | **App real**: com o app aberto, executar `reverb.exe "reverb://add?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DjNQXAC9IVRw"` ⇒ continua existindo **um** processo `reverb.exe` principal e um job novo aparece (`reverb-cli jobs list --json --data-dir <dir>`) | script + WebdriverIO |
| T10b | **App real**: com o app **fechado**, executar o exe com o mesmo link ⇒ o app abre e o job é criado | script + WebdriverIO |
| T11 | **App real**: ligar `launchAtStartup` ⇒ existe valor em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` apontando para o exe com `--minimized`; desligar ⇒ removido | script (PowerShell `Get-ItemProperty`) |
| T12 | (rede) `diagnostics_run` no ambiente real ⇒ todos os itens ok | `verify:net` |
| T13 | **Checagem manual do usuário** (pedir e registrar respostas): (a) ícone e menu da bandeja funcionam; (b) notificação de conclusão aparece; (c) bookmarklet no navegador abre o Reverb e cria job; (d) atalho global baixa link copiado; (e) fechar a janela mantém o app na bandeja | manual |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔ · T10, T10b, T11 ✔ · T13 confirmado pelo usuário

## Armadilhas
- Deep link no Windows só é registrado de verdade pelo instalador NSIS; em `tauri dev` use
  `register_all()`. Para T10 use o exe de debug e registre via `register_all()` no início.
- O plugin single-instance precisa ser o primeiro registrado, senão a segunda instância abre outra janela.
- Ler a área de transferência com a janela escondida funciona no Windows; no Linux depende do
  servidor gráfico — tratar erro silenciosamente (log em debug).
