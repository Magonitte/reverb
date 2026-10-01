# 04 — Android (A0–A3) — somente após a F16 e com autorização do usuário

**Situação:** viável, mas com um risco técnico central: rodar o yt-dlp **e** um runtime
JavaScript (exigido pelo YouTube) no Android. Por isso a fase A0 é um **spike com decisão
go/no-go**. As fases A1–A3 só existem se A0 passar.

Fatos que já guiam o desenho:
- O `reverb-core` não depende de Tauri e o download é abstraído por `trait DownloadBackend`
  (F03) — o Android implementa outro backend.
- A UI já é responsiva com BottomNav < 640 px (F05).
- O atualizador do Tauri **não** funciona no Android; a atualização será própria (A3).
- Apps que baixam do YouTube **não** são aceitos na Play Store ⇒ distribuição por APK no GitHub.
- Referências de apps existentes que resolvem esse problema: Seal e YTDLnis (ambos usam a
  biblioteca `youtubedl-android`, que empacota Python + yt-dlp + FFmpeg).

## ⛔ Ponto de parada humana (antes da A0)
Pedir ao usuário para instalar (ou autorizar instalar): Android Studio (SDK, Platform-Tools,
NDK, um emulador x86_64 com Android 14+), **JDK 17** (o JDK atual é 1.8), e alvos Rust
(`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`).
Variáveis `ANDROID_HOME`, `NDK_HOME`, `JAVA_HOME`. Opcional: um celular Android físico com
depuração USB para testes reais.

## A0 — Spike: yt-dlp + runtime JS no Android (go/no-go)

**Objetivo:** provar, num app mínimo (pode ser um app Kotlin puro, fora do Tauri), que é
possível baixar áudio do YouTube no Android **com os desafios JS resolvidos**.

Tarefas:
1. Pesquisar o estado atual (web) de: `youtubedl-android` (coordenadas Maven atuais — há o
   original do yausername via JitPack e o fork usado pelo Seal publicado no Maven Central —
   escolher o mais mantido), versão do yt-dlp embutida e mecanismo de atualização do yt-dlp
   em tempo de execução (`updateYoutubeDL`), e **como Seal/YTDLnis lidam com o runtime JS**
   (EJS) atualmente. Registrar fontes no PROGRESS.md.
2. Opções de runtime JS a avaliar, nesta ordem: (a) o que Seal/YTDLnis usam; (b) QuickJS
   (`qjs`) compilado para Android empacotado como `lib*.so` executável em `jniLibs` (técnica
   usada para binários nativos), passado com `--js-runtimes quickjs:<caminho>`; (c) Deno/Node
   para Android (se existir build mantido).
3. App mínimo que baixa FX1 e FX2 em `Music/ReverbSpike` no emulador e, se houver, no celular.

Critérios **go** (todos):
- FX2 baixado com sucesso 3 vezes seguidas (formato de áudio ≥ 128 kbps) no emulador.
- O log do yt-dlp mostra o runtime JS em uso (equivalente a `JS runtimes: …`) **ou** o download
  funciona sem aviso de "No supported JavaScript runtime".
- Atualização do yt-dlp em tempo de execução funciona (trocar de versão e baixar de novo).
- Tamanho do APK do spike ≤ 150 MB.

**Rota B** (avaliar se a Rota A acima — yt-dlp + runtime JS — falhar em algum critério):
seguir o estudo `plano/anexos/estudos/E7-android-rota-innertube.md` — API InnerTube + PO token
gerado em WebView/JavaScriptSandbox + biblioteca de cifra como **dependência externa
atualizável** (não reimplementar a cifra). Exige licença GPL-3.0 no app Android (ver decisão de
licença da F06); se a licença escolhida for MIT, **perguntar ao usuário** antes de testar a Rota B.
Mesmos critérios go, trocando "atualização do yt-dlp" por "atualização da configuração de cifra
sem nova versão do app".

Se **no-go** nas duas rotas: parar, registrar evidências e apresentar ao usuário a alternativa
"**Reverb Remoto**": o app Android vira um cliente leve que envia links para o Reverb do PC
(servidor HTTP local opcional no desktop, pareamento por QR code com token, só na rede local),
e as músicas chegam ao celular via Syncthing. Só seguir com a alternativa se o usuário aprovar
(nesse caso, escrever um plano de fases para ela no mesmo formato antes de implementar).

## A1 — Base Android do Reverb (se go)

1. `npm run tauri android init`; ajustar `src-tauri/gen/android` (commitado).
2. Plugin Tauri próprio em Kotlin (`tauri-plugin-reverb-android`) expondo ao Rust: download via
   biblioteca escolhida na A0 (com progresso e cancelamento), atualização do yt-dlp, caminho do
   runtime JS. Implementação `AndroidBackend: DownloadBackend` no Rust chamando o plugin.
3. `reverb-core` compilando para Android: módulos desktop-only (`tools` de binários, `notify`,
   `trash`, autostart, tray, atalho global) atrás de `#[cfg(not(target_os = "android"))]` ou
   features; ffmpeg via a biblioteca do Android.
4. Armazenamento: salvar em `Music/Reverb` via MediaStore (Android 10+), sem permissão ampla.

Testes: `cargo check --target aarch64-linux-android -p reverb-core`; `npm run tauri android build
-- --debug` gera APK; instrumentação no emulador (`adb shell am instrument …` ou Maestro) baixa
FX1 pela UI ⇒ arquivo existe (`adb shell ls`).

## A2 — Funcionalidades móveis

1. **Compartilhar** (intent `ACTION_SEND` `text/plain`) a partir do app do YouTube ⇒ abre o Reverb
   com o Preview do link.
2. **Serviço em primeiro plano** com notificação de progresso para filas/playlists longas
   (Android encerra apps em segundo plano).
3. Fila, Biblioteca (MediaStore), Playlists sincronizadas (sincronização ao abrir o app e por
   WorkManager periódico, respeitando Wi-Fi apenas — nova configuração `syncOnlyOnWifi`).
4. UI móvel: BottomNav, gestos, telas adaptadas (sem titlebar), modo escuro do sistema.

Testes: instrumentação (Maestro ou Espresso via plugin) cobrindo compartilhar ⇒ baixar,
serviço em primeiro plano sobrevivendo a 2 min com a tela desligada (emulador), sync periódica
(WorkManager com `TestDriver`). Checagem manual do usuário no celular dele.

## A3 — Atualização e distribuição Android

1. Assinatura: keystore de release gerada localmente (⛔ usuário guarda cópia e senha; segredos no GitHub).
2. CI: job Android no `release.yml` gerando APK assinado (universal e/ou arm64) anexado à mesma
   release do desktop.
3. **Botão "Atualizar" no Android**: consultar `https://api.github.com/repos/<owner>/<repo>/releases/latest`,
   comparar versão, baixar o APK (com progresso), verificar SHA-256 publicado no release,
   abrir o instalador (`ACTION_VIEW` com `FileProvider`, permissão `REQUEST_INSTALL_PACKAGES`).
   Verificação automática diária; mesma aba "Atualizações" da UI (ferramentas = versão do yt-dlp
   da biblioteca, com "Atualizar").
4. Documentar a alternativa **Obtainium** (atualiza APKs direto de releases do GitHub).

Testes: unitários da comparação de versão e verificação de hash; teste real: instalar APK
versão N no emulador, publicar N+1 (⛔ confirmação), app detecta e abre o instalador
(confirmar via `adb shell dumpsys activity` que a activity do instalador abriu).
