# F06 — Atualização automática do app, botão "Atualizar" e pipeline de release

**Objetivo:** desde cedo o app se atualiza sozinho (requisito obrigatório do usuário).
Ao fim desta fase: repositório no GitHub, CI verde em Windows e Linux, releases assinados
gerados por tag, e **prova real** de que uma versão instalada detecta, baixa, instala e reinicia
na versão nova. Também a aba "Atualizações" com o botão manual, incluindo ferramentas.

**Pré-requisito:** F05. **Arquitetura:** §16, §17. **Design:** §3.9 (mockup).

## ⛔ Ponto de parada humana (antes de qualquer ação externa)

Pergunte ao usuário e **aguarde respostas explícitas**:
1. Nome do repositório e dono. Sugestão: `Magonitte/reverb` (conta autenticada no `gh`).
2. Visibilidade: **público** é o recomendado (o atualizador baixa `latest.json` e instaladores
   sem autenticação; repositório privado exigiria publicar releases em outro repositório
   público). Confirmar que o usuário aceita o código público (sem segredos — a varredura garante).
3. Autorização para: criar o repositório, fazer o primeiro push, cadastrar segredos via
   `gh secret set`, publicar **duas releases de teste** (`v0.1.0` e `v0.1.1`) e **instalar a
   versão de teste** no Windows do usuário (instalação por usuário, em `%LOCALAPPDATA%`,
   desinstalável depois).
4. **Licença do Reverb** (arquivo `LICENSE` na raiz, criado antes do primeiro push):
   - **MIT** — mais permissiva; impede incorporar código GPL no futuro;
   - **GPL-3.0** — permite usar bibliotecas GPL no app Android (Rota B do A0, estudo E7).
   Explique as duas opções e registre a escolha. (Ferramentas externas como yt-dlp e bgutil rodam
   como programas separados e não dependem dessa escolha.)
5. Senha da chave de assinatura: o usuário define (recomendado) ou autoriza gerar uma aleatória.
   Avise: **perder a chave privada ou a senha impede atualizações futuras**; o usuário deve
   guardar cópia de `%USERPROFILE%\.tauri\reverb.key` e da senha em local seguro (fora do repo).

## Tarefas

1. **Chave de assinatura**: `npx tauri signer generate --ci -p "<senha>" -w "%USERPROFILE%\.tauri\reverb.key"`
   (conferir flags com `npx tauri signer generate --help` da versão instalada). Chave pública vai
   no `tauri.conf.json` (`plugins.updater.pubkey`). A privada **nunca** entra no repositório.
2. **Repositório**: `gh repo create <owner>/<repo> --public --source . --remote origin`;
   `git push -u origin main`. Segredos: `TAURI_SIGNING_PRIVATE_KEY` (conteúdo do arquivo da
   chave) e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` via `gh secret set` (lendo de arquivo/stdin,
   nunca colocando o segredo na linha de comando que vai para o log).
3. **Config do bundle**: em `tauri.conf.json`: `bundle.active = true`,
   `bundle.createUpdaterArtifacts = true`, `bundle.windows.nsis.installMode = "currentUser"`,
   ícones; `plugins.updater` com `pubkey`, `endpoints` (§17) e `windows.installMode = "passive"`.
   Alvos por plataforma (§17): `tauri.windows.conf.json` ⇒ `{"bundle":{"targets":["nsis"]}}`,
   `tauri.linux.conf.json` ⇒ `{"bundle":{"targets":["appimage","deb"]}}`.
   Teste local: `npm run tauri build` no Windows gera `…_x64-setup.exe` **e** `…_x64-setup.exe.sig`
   (com `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD` definidos no ambiente da sessão).
4. **Serviço de atualização** (`src-tauri/src/updater.rs`): `check()` (devolve
   `AppUpdateInfo { version, currentVersion, notes, date }` ou `None`), `install()` com eventos
   `updater://progress {downloaded, total}` e reinício (`app.restart()`). Verificação automática
   na inicialização (atraso de 10 s) e a cada 6 h se `autoCheckAppUpdates`; emite
   `updater://available`. Comandos `updater_check`, `updater_install`, `app_restart`.
   Plugins `tauri-plugin-updater` e `tauri-plugin-process` registrados.
5. **Modos headless** (§17): `--headless-update-check <arquivo>` ⇒ escreve
   `{"currentVersion","available":bool,"version"?}`; `--headless-update-install` ⇒ instala
   e sai com 0 (sem reiniciar o app). Executados dentro do `setup()` sem criar janela.
6. **UI "Atualizações"** (Configurações › Atualizações, mockup do design §3.9): bloco do app
   com todos os estados; bloco de ferramentas (versão, canal, status, "Reverter"); botão
   **"Verificar atualizações"** (app + ferramentas, com `force`); toggles `autoCheckAppUpdates`
   e `autoUpdateTools`; seletor de canal do yt-dlp. Ponto na sidebar e toast quando houver
   atualização do app. Store `updater` + extensão do mock (cenários `update-available`,
   `update-downloading`, `update-error`).
7. **Versionamento**: `scripts/bump-version.mjs <x.y.z>` atualiza `package.json` e todos os
   `Cargo.toml` (versão do workspace) e roda `cargo update -w`; `scripts/release.mjs <x.y.z>`
   ⇒ exige árvore limpa e `npm run verify` verde ⇒ bump ⇒ commit `chore(release): vX.Y.Z` ⇒
   tag ⇒ push (**só após confirmação do usuário**, ver ponto de parada).
8. **Workflow** `.github/workflows/release.yml`: gatilho `push` de tags `v*`; job que confere
   `tag == v<package.json version>`; matriz `windows-latest` e `ubuntu-22.04` (deps de sistema da F00);
   passos `actions/setup-node`, `dtolnay/rust-toolchain@stable`, `npm ci` (a tauri-action **não**
   instala dependências) e então `tauri-apps/tauri-action@v0` com `tagName: ${{ github.ref_name }}`, `releaseName`,
   `releaseBody` (texto do CHANGELOG da versão), `releaseDraft: false`, `prerelease: false`,
   `includeUpdaterJson: true`; env `GITHUB_TOKEN`, `TAURI_SIGNING_PRIVATE_KEY`,
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Permissão `contents: write`.
9. **CI**: o `ci.yml` da F00 passa a rodar a cada push; corrigir o que falhar no Linux
   (inclusive a pendência T13 da F02).

## Testes de verificação

| # | Teste | Tipo |
|---|-------|------|
| T1 | Lógica de decisão (Rust): intervalo de verificação, não verificar se desligado, comparação semver | unit |
| T2 | UI: cada estado do bloco do app (ocioso, verificando, atualizado, disponível, baixando com %, pronto, erro) e do bloco de ferramentas; botão dispara `updater_check` + `tools_check_updates(force)`; ponto na sidebar aparece com `updater://available` | Vitest |
| T3 | E2E mock: cenário `update-available` ⇒ toast + ponto + clicar "Atualizar agora" mostra progresso até "Reiniciando" | Playwright |
| T4 | CI verde nos dois SOs: `gh run watch` do último push termina com sucesso; registrar URL do run | CI |
| T5 | Release `v0.1.0` (após confirmação): workflow verde; `gh release view v0.1.0 --json assets` contém `*_x64-setup.exe`, `*_x64-setup.exe.sig`, `*.AppImage`, `*.AppImage.sig`, `*.deb`, `latest.json` | real |
| T6 | `latest.json` (baixado de `releases/latest/download/latest.json`) tem `version` = 0.1.0 e `platforms` com `windows-x86_64` e `linux-x86_64`, cada um com `url` e `signature` não vazios | real |
| T7 | Instalar `v0.1.0` silenciosamente (`<setup>.exe /S`), localizar o exe instalado (`%LOCALAPPDATA%\Reverb\reverb.exe` ou conforme o NSIS — descobrir e registrar), rodar `--headless-update-check` ⇒ `available: false` | real |
| T8 | Publicar `v0.1.1` (mudança visível mínima, ex. texto no CHANGELOG) ⇒ `--headless-update-check` na instalação 0.1.0 ⇒ `available: true, version: "0.1.1"` | real |
| T9 | `--headless-update-install` ⇒ o processo encerra (no Windows o próprio updater fecha o app para rodar o instalador); o script de teste espera até 180 s, consultando a cada 5 s o `--headless-selftest` do exe instalado, até obter `version: "0.1.1"` | real |
| T10 | Checagem manual do usuário (pedir): abrir o app 0.1.1 instalado ⇒ aba Atualizações mostra "atualizado"; botão "Verificar atualizações" responde | manual |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · T4–T9 ✔ · T10 confirmado pelo usuário

## Armadilhas
- `releaseDraft: true` (como no projeto antigo) faz `releases/latest/download/latest.json`
  apontar para a release anterior — por isso **false**.
- A segunda job da matriz atualiza o `latest.json` da mesma release (tauri-action faz o merge);
  confira que as duas plataformas aparecem (T6).
- O updater exige que a versão nova seja **maior** (semver).
- No Windows, `download_and_install` encerra o app ao iniciar o instalador (limitação do
  Windows) — salve o estado (fila já é persistente) antes de chamar. Verifique na T10 se o
  instalador reabre o app sozinho; se não reabrir, registre e informe o usuário (a UI deve
  avisar "O Reverb será fechado para instalar a atualização"). No Linux (AppImage) chame
  `app.restart()` depois de instalar.
- A chave privada no segredo deve ser o **conteúdo** do arquivo, não o caminho.
- Após os testes, desinstale a versão de teste se o usuário pedir (perguntar).
