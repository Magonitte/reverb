# Solução de problemas

Comece por **Configurações → Avançado → Diagnóstico** e pela situação das ferramentas em **Atualizações**. Guarde a mensagem de erro completa e a versão do app. Os logs ficam na pasta de dados descrita em [Instalação](instalacao.md).

| Sintoma                                     | Verificação e ação                                                                                                                                            |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| App Windows não abre                        | Confira instalação e WebView2; tente abrir pelo menu Iniciar. Consulte logs antes de apagar dados.                                                            |
| Janela desaparece, mas o processo continua  | O comportamento de fechar para bandeja pode estar habilitado. Use Sair na bandeja ou altere Geral.                                                            |
| Ferramenta ausente ou instalação incompleta | Verifique internet/espaço; consulte Atualizações e tente novamente. O gerenciador verifica os downloads e pode recuperar versões.                             |
| Vídeo indisponível ou exige sessão          | Confirme a URL e se sua sessão possui acesso. Configure cookies próprios quando aplicável; cookies não garantem disponibilidade.                              |
| Playlist Deezer retorna erro 800            | Confirme se ela existe e está acessível. Links removidos/privados não podem ser importados pela API pública.                                                  |
| Spotify pede credenciais                    | Salve seu Client ID/Secret em Metadados; Deezer não exige essa etapa.                                                                                         |
| Spotify retorna acesso restrito             | Credenciais válidas não garantem leitura da playlist. Consulte as limitações OAuth em [Configuração](configuracao.md#spotify).                                |
| Faixa aparece como Revisar/Não encontrada   | Confira artista, versão e duração. Se houver casamento duvidoso, selecione manualmente após revisar; ausência de casamento bloqueia o download nessa análise. |
| Uma letra/capa não aparece                  | O provedor pode não ter o conteúdo ou estar indisponível. Confira as opções de pós-processamento e revise os dados da faixa.                                  |
| Música aparece como arquivo ausente         | Verifique se o arquivo foi movido/apagado e reexamine a pasta correta.                                                                                        |
| Qualidade-alvo não enfileira nada           | Confira alvo, autoUpgrade, bitrate conhecido, origem YouTube, arquivo presente e existência de uma melhoria suficiente.                                       |
| Artistas não foram verificados              | Confira app em execução, modo offline, intervalo e política. Use Verificar agora; o agendador possui atraso inicial.                                          |
| FLAC marcado como inconclusivo              | A análise espectral depende do sinal e da taxa de amostragem. FLAC é um contêiner/codec, não prova origem sem perdas.                                         |

## Para desenvolvedores

- **Erro 112/espaço insuficiente:** examine `target/` e o espaço livre. Limpe somente artefatos gerados; uma recompilação pode exigir dezenas de GB.
- **Erro 1455/memória/paginação:** tente `CARGO_BUILD_JOBS=1` e `CARGO_INCREMENTAL=0` e feche builds simultâneos.
- **Teste de log vazio:** confira `RUST_LOG`; a suíte usa eventos `info`.
- **Playwright perde estado/contexto:** execute com fontes estáveis. Build e geração de bindings podem disparar recarga do Vite.
- **Diferença de screenshot:** inspecione imagem/diff e corrija o comportamento antes de aceitar uma nova base.
- **Teste de rede variável:** registre erro e respostas observadas. Execute em sequência para reduzir disputa, preservando as exigências; rede simulada não substitui a validação real.
- **WebdriverIO não inicia:** prepare `tauri-driver`, WebView2 e as ferramentas. O script gerencia o msedgedriver correspondente à versão do runtime.

## Relatar um problema

Abra uma [issue](https://github.com/Magonitte/reverb/issues) com versão, sistema operacional, passos para reproduzir, comportamento esperado/observado e mensagem ou trecho de log relevante. Informe se usou a versão instalada, dev ou mock.

Não anexe o banco completo, cookies ou credenciais. Remova dados privados dos registros e use uma faixa/URL pública de exemplo quando possível. Para continuidade da implementação, registre a falha, hipótese, correção e portão em PROGRESS.

## Atualizações nas Configurações

Para atualizar o aplicativo, use o cartão **Reverb → Verificar atualização do Reverb → Atualizar agora**. Para FFmpeg e outras ferramentas, use o botão da respectiva linha. No modo offline, consultas e instalações ficam desabilitadas. Se aparecer **Aguardando downloads terminarem**, aguarde os trabalhos ativos concluírem. Uma falha mostra mensagem e mantém a ação para nova tentativa; quando necessário, use **Reverter** para voltar à versão anterior da ferramenta.
