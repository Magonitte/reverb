# F16 — Polimento, regressão total e release 3.0.0

**Objetivo:** app rápido, acessível, completo nos dois idiomas, sem regressões, com paridade
100 % comprovada, documentação de uso, e release `v3.0.0` publicado e atualizável.

**Pré-requisito:** F15. **Tudo** do plano.

## Tarefas

1. **Orçamentos de desempenho** (cada um com teste automatizado):
   - JS inicial (chunk de entrada) ≤ 250 KB gzip — script `scripts/check-bundle.mjs` lendo `dist/`.
   - Busca na biblioteca com 10.000 itens ≤ 100 ms (teste Rust com banco populado; medir mediana de 20 execuções).
   - Primeira tela interativa em ≤ 2 s no mock (Playwright: do `goto` até a barra de comando focável).
   - Uso de memória do app real ocioso ≤ 300 MB (soma dos processos `reverb.exe` + WebView2 filhos,
     medido 30 s após abrir, via script com `sysinfo`/PowerShell) — se exceder, otimizar ou
     **perguntar ao usuário** se aceita o valor medido.
2. **Acessibilidade**: axe em todas as rotas e diálogos, dois temas, zero `serious/critical`;
   navegação só por teclado nos fluxos: baixar por URL, baixar seleção de playlist, editar tag,
   aplicar revisão, atualizar o app (teste Playwright usando apenas `keyboard`).
3. **i18n**: revisão completa do inglês; nenhuma chave sem uso (script lista chaves não
   referenciadas no código e falha se houver); datas/números formatados por idioma (`Intl`).
4. **Estados**: toda tela tem estados vazio, carregando e erro (checklist + screenshots).
5. **Segurança**: teste que lê `capabilities/default.json` e falha se houver permissão fora da
   lista do §15; CSP conferida; `scan:secrets` no histórico completo do git
   (`git log -p` varrido pelo mesmo script); dependências: `npm audit --omit=dev` sem
   vulnerabilidades altas/críticas e `cargo audit` (instalar `cargo-audit`) sem vulnerabilidades
   (exceções só com justificativa registrada).
6. **Paridade**: preencher `plano/03-paridade.md` marcando cada linha com o teste que a cobre;
   nenhuma linha sem marca.
7. **Documentação** (`docs/`): `README.md` do projeto (o que é, instalar, usar, atualizar,
   onde ficam os dados, modo portátil), `TROUBLESHOOTING.md` (YouTube mudou ⇒ autocura/nightly;
   bloqueio "não é um robô" ⇒ provedor de PO token automático (bgutil), depois cookies;
   Chrome cookies ⇒ Firefox; Linux Wayland atalhos; playlist editorial do Spotify ⇒ limitação
   da API), `CREDITS.md` (ferramentas externas usadas como programas e projetos cujos
   comportamentos foram estudados — lista de `plano/anexos/estudos/LEIA-ME.md`),
   `CHANGELOG.md`, `ARQUITETURA.md` (resumo do plano real implementado, com desvios).
8. **Regressão total**: `npm run verify`, `npm run e2e`, `npm run verify:net`, `npm run e2e:app`
   e CI verde nos dois SOs — tudo na mesma revisão.
9. **Release 3.0.0** (⛔ parada humana: confirmar publicação): `node scripts/release.mjs 3.0.0`
   ⇒ workflow verde ⇒ assets e `latest.json` conferidos (como F06 T5–T6).
10. **Atualização real até a 3.0.0**: na instalação de teste da F06 (0.1.1) ou numa nova
    instalação da 0.1.1, `--headless-update-check` ⇒ `3.0.0` disponível ⇒ instalar ⇒ selftest
    mostra 3.0.0 (mesmo procedimento da F06 T8–T9).
11. **Checagem final do usuário** (pedir): usar o app por alguns minutos — baixar uma faixa,
    uma playlist pequena, editar uma tag, ver a aba Atualizações — e confirmar.
12. **Linux** (a máquina do usuário não tem Linux com interface): o CI garante compilação,
    testes offline, bundle AppImage/.deb e execução das ferramentas Linux. Perguntar ao usuário
    se ele tem como testar o AppImage numa máquina/VM Linux; se sim, passar um roteiro curto
    (abrir, baixar FX1, ver a aba Atualizações); se não, registrar "UI no Linux não verificada
    manualmente" como limitação conhecida no PROGRESS.md e no README.

## Testes de verificação
Todos os itens 1–10 acima são testes; além deles, a suíte completa de todas as fases.

## Portão final
Todos os comandos do item 8 ✔ · orçamentos ✔ · paridade 100 % ✔ · release 3.0.0 ✔ ·
atualização real ✔ · confirmação do usuário (item 11) ✔.

Ao concluir: escrever no PROGRESS.md um resumo final (o que foi entregue, desvios do plano,
limitações conhecidas) e perguntar ao usuário se deseja iniciar a parte Android (`04-android.md`).
