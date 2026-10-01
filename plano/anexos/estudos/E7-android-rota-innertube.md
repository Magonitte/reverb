# E7 — Rota alternativa de extração no Android (para o spike A0)

## O que foi aprendido
Um cliente Android maduro de YouTube Music obtém o áudio **sem yt-dlp e sem Node/Deno**:
1. Usa a **API interna do YouTube (InnerTube)** para busca, metadados e `player`.
2. **PO token** gerado dentro de um **Android WebView**: carrega o desafio BotGuard do YouTube
   no WebView, executa e extrai o token (inicialização única, reaproveitado; recria o WebView se
   o renderizador morrer ou travar; timeout por geração).
3. **Decifragem de assinatura / parâmetro `n`** via uma biblioteca dedicada (`innertubex`,
   distribuída pelo JitPack — grupo `com.github.MetrolistGroup`, versão estudada `v0.7.0`) que
   baixa **configurações de cifra mantidas pela comunidade** (`player_configs.json` num
   repositório público) e se atualiza quando o YouTube troca o player; ao receber rejeição de
   stream, força atualização da configuração e tenta de novo.
4. Downloads via `DownloadManager`/cache do Media3 (ExoPlayer).

## Como usar no A0
Adicionar ao spike A0 uma **Rota B**, avaliada se a Rota A (yt-dlp via `youtubedl-android` +
runtime JS) falhar em algum critério go:
- **Rota B** = InnerTube + PO token por WebView/`androidx.javascriptengine` (JavaScriptSandbox)
  + biblioteca de cifra **como dependência externa atualizável** (não reimplementar a cifra:
  é a parte que muda junto com o YouTube).
- **Licença:** essas bibliotecas são GPL-3.0. Usá-las como dependência no app Android obriga o
  app Android a ser GPL-3.0 ⇒ só seguir com a Rota B se o usuário escolher licença GPL-3.0 na
  F06 (ver ponto de parada de licença) ou aceitar isso explicitamente no A0.
- **Critérios go da Rota B**: os mesmos do A0, trocando "atualização do yt-dlp" por
  "atualização da configuração de cifra sem nova versão do app".

## Ideia reaproveitável também no desktop
O padrão "**configuração remota atualizável + retry após rejeição**" é o mesmo da nossa
autocura (§9): não há nada novo a implementar no desktop.
