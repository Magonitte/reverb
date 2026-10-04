# Revisão de UI/UX — 3 de outubro de 2026

A avaliação começou na janela nativa do Reverb, com a biblioteca existente, e continuou na interface aberta em modo mock. Os cenários `empty`, `busy`, `big`, `playlists` e `onboarding` permitem experimentar estados de interface sem baixar músicas ou alterar os arquivos da biblioteca real. A simulação `big` contém 5.000 faixas.

## Avaliação por tela

| Tela | Observação | Ajuste realizado |
| --- | --- | --- |
| Navegação | Ícones isolados exigiam memorizar o destino. | Nomes visíveis a partir de 1.024 px; trilho compacto em janelas intermediárias; navegação inferior mantida abaixo de 640 px. Indicadores continuam disponíveis. |
| Início | Campo sem botão explícito e ações rápidas sem explicação. | Área de entrada destacada, instrução de uso, botão Pesquisar/Analisar link, cartões com descrições e distribuição em uma, duas ou quatro colunas. |
| Resultados da busca | A seleção da fonte precisava de maior diferenciação visual. | Abas com fundo e contorno no item selecionado; campo e botão continuam disponíveis para outra busca. |
| Pré-visualização | Fluxo existente já separa fonte, formato, opções e destino, com ações no rodapé. | Contraste dos textos discretos e dimensões dos botões compartilhados melhorados; estrutura e avisos de recodificação preservados. |
| Coleção | Lista e seleção múltipla precisam permanecer claras em janelas menores. | Cabeçalho com ações agrupadas e botões maiores; seleção, filtros e contagem continuam funcionando. |
| Biblioteca: lista e álbuns | Controles densos, campo de busca desalinhado e ações no cabeçalho precisam acomodar menos espaço horizontal. | Busca alinhada aos filtros, hierarquia do cabeçalho, agrupamento das ações e leitura global melhorados; virtualização, filtros, seleção e visualização de álbuns preservados. |
| Artistas seguidos / Faltando | Os estados vazios já explicam como iniciar o acompanhamento. | Recebem navegação com rótulos, cabeçalhos e botões consistentes. Fluxo de acompanhamento verificado na suíte de interface. |
| Playlists e detalhe | Criar uma sincronização tinha o mesmo peso das ações secundárias. | Criação destacada em âmbar, cabeçalhos e controles consistentes; detalhe e estados de sincronização conferidos com dados simulados. |
| Diálogo de sincronização | Campos, opções e avisos precisam continuar legíveis sem deslocar a ação final. | Botões compartilhados maiores e textos discretos com contraste melhor; informações dos campos Select associadas por aria-describedby. |
| Atividade | Abas precisavam diferenciar melhor o filtro ativo. | Abas selecionadas com fundo e contorno; ações do cabeçalho agrupadas; progresso, pausa, reordenação e erros continuam disponíveis. |
| Revisar | Aplicar o candidato não se destacava das opções de manter ou editar. | Aplicar candidato usa o botão principal; leitura do cabeçalho, campos e navegação melhorada. |
| Editor de tags | Tela vazia orientava somente a abrir pela Biblioteca; cortar áudio aparecia desativado sem arquivo. | Instrução contempla abrir um arquivo diretamente; cortar aparece com arquivo aberto; campos agrupados em cartão; Salvar destacado e com largura adequada. |
| Configurações: Geral | Formulários muito largos aumentavam a distância entre rótulos e controles. | Conteúdo limitado a 960 px e centralizado; cartões estáticos deixam de parecer clicáveis no hover. |
| Configurações: Downloads e Metadados | Cartões de seções distintas ficavam encostados. | Espaço entre as seções, largura de leitura limitada e controles consistentes. |
| Configurações: Integração, Atualizações e Avançado | Densidade e largura dificultavam identificar os grupos. | Melhor hierarquia, abas selecionadas claras, largura limitada e agrupamento das ações. |
| Primeiro uso | Continuar tinha aparência de ação secundária. | Continuar/Concluir destacado como ação principal; cartões estáticos e textos mais legíveis. |

## Critérios de implementação

- Preservar a identidade de vinil, vidro e âmbar, com menos linhas decorativas no fundo.
- Elevar a tipografia base a 14 px e clarear textos discretos no tema escuro.
- Manter os atalhos, foco de teclado, traduções pt-BR/en, temas claro/escuro e preferências de movimento reduzido.
- Não alterar contratos IPC, serviços Rust, biblioteca real ou mecanismos de download.
- Validar comportamento e acessibilidade antes de renovar as referências visuais. Referências de screenshots dependem de Windows e da fonte instalada.

## Limites da avaliação

Os testes mock comprovam comportamento da interface e seus contratos simulados. Eles não comprovam disponibilidade dos provedores, acesso a credenciais ou downloads reais de rede. A inspeção manual cobriu as telas principais, as seis seções de configurações, busca, prévia, coleção, diálogo de sincronização e estados preenchidos de Biblioteca, Revisar e Editor. A suíte cobre adicionalmente os fluxos de primeiro uso, importação, artistas, atalhos e ferramentas.

## Validação final

- 256 testes unitários passando (39 arquivos, execução com dois workers).
- 85 testes de interface passando, incluindo o novo fluxo pelo botão Pesquisar/Analisar link.
- Após o último ajuste de alinhamento na Biblioteca, os 18 testes das telas afetadas passaram novamente.
- Axe sem violações graves/críticas nas rotas testadas em claro e escuro; telas preenchidas e diálogos também verificadas pela suíte.
- Sem rolagem horizontal nas rotas testadas em 390×844, 900×600, 1366×768 e 1920×1080.
- TypeScript, ESLint, consistência de traduções e `git diff --check` passando.
- Build Vite e compilação nativa `cargo build -p reverb --offline` concluídos. Executável local: `target/debug/reverb.exe`.
- Referências visuais afetadas renovadas e imagens representativas conferidas manualmente.

A nova interface ficou aberta na prévia local. A instalação existente não foi substituída. A verificação da nova compilação na janela nativa ficou pendente de autorização para encerrar e reabrir a instância antiga: a revisão automática bloqueou seu encerramento por possível perda de alterações não salvas. A janela nativa inspecionada inicialmente era a versão instalada.

## Refinamentos dos comentários do navegador (03/10/2026)

- Ferramentas: descrição de finalidade em linguagem simples; status e ações em colunas fixas, botões com a mesma largura e organização vertical em telas pequenas.
- Provedores: botão direto para cada serviço, explicação de finalidade, guia de quatro passos e confirmação ao salvar. A chave AcoustID de consulta é a chave de aplicativo, não a de usuário.
- Biblioteca: ações de cada faixa aparecem ao passar o mouse ou focar por teclado. Em dispositivos sem hover permanecem visíveis.
- Artistas: sugestões automáticas após duas letras e 350 ms sem digitar; respostas antigas são descartadas, busca manual continua disponível e carregamento é anunciado.
- Navegação: opção de recolher/expandir em desktop, com preferência persistente e rótulos acessíveis no modo de ícones.
- Identidade: SVG fornecido aplicado à barra de título; placeholder redesenhado como vinil com sulcos, reflexo, rótulo e furo central.

Guias consultados em fontes oficiais: [AcoustID](https://acoustid.org/webservice), [Spotify](https://developer.spotify.com/documentation/web-api/concepts/apps), [Discogs](https://support.discogs.com/hc/en-us/articles/360007423833-How-Do-I-Change-My-Account-Settings) e [Jamendo](https://developer.jamendo.com/v3.0).

Outras oportunidades: resumir os filtros ativos em chips removíveis; incluir explicações de termos como ReplayGain e qualidade-alvo junto aos controles. Estas sugestões não foram implementadas neste refinamento.

Validação destes refinamentos: 256 testes unitários passaram; a execução de 88 testes de interface teve 86 sucessos e duas falhas. Uma era o seletor incorreto do novo teste, corrigido; a outra foi intermitente no fluxo do Internet Archive. A repetição dos seis testes dos dois arquivos afetados passou integralmente. TypeScript, ESLint, traduções, build Vite e orçamento de bundle passaram. A simulação visual não testa disponibilidade real dos provedores nem as credenciais pessoais.

Compilação nativa destes refinamentos concluída com `cargo build -p reverb --offline`. A execução da nova versão na janela nativa segue sem validação; a prévia local mostra o resultado atualizado.

## Notificações e ajustes de clareza (03/10/2026)

- Melhoria automática de qualidade ganhou rótulo visível e explicação de finalidade junto à chave.
- Guia Spotify corrigido para indicar os campos abaixo. Integração agora explica em quatro passos como mostrar a barra, arrastar o favorito, usá-lo e criar manualmente; botão e link ficam separados.
- Atualização concluída ganha símbolo verde, além do texto. Na Atividade, trabalhos em execução usam o mesmo vinil animado do Início, inclusive quando há miniatura. A preferência de movimento reduzido continua respeitada.
- Avisos redesenhados com título, mensagem, detalhes, ícone, ações separadas e barra de tempo. Avisos comuns duram 8 s; avisos com ações têm prazo maior. Mouse, foco, documento oculto e diálogo aberto pausam a contagem. Diálogos ocultam avisos temporariamente para não cobrir seus controles.
- Links copiados: confirmação Baixar/Cancelar, URL e formato. Prazo padrão de 35 s, configurável em 20/35/60 s. Fechar, cancelar ou expirar não enfileiram downloads. Confirmações ficam bloqueadas durante o pedido para evitar envio duplo; erros permitem tentar novamente.
- Sons: Cristal suave, Marimba, Notas luminosas e Sem som; volume e prévia. Arquivo personalizado em WAV/MP3/OGG/M4A, até 2 MB, validado pelo decodificador antes de salvar; reprodução limitada a 5 s. Preferência e cópia do áudio são armazenadas no perfil local da WebView, compartilhadas com a janela de avisos. Restaurar as configurações também restaura esses valores. O áudio não é enviado a serviços externos.
- Janela flutuante local para o app sem foco/minimizado, com as mesmas ações e estilos. Usa o diretório WebView da janela principal para compartilhar o som escolhido. Fundo do cartão opaco para manter o contraste sobre outros aplicativos. Na falha de criação da janela, há aviso nativo de reserva; ele não inicia download.

Validação: 262 testes unitários passaram. A execução de 96 testes de interface teve 94 sucessos; um teste temporizado de metadados falhou e o teste novo do popup encontrou contraste e marcos acessíveis inadequados. Após a correção do popup, os 16 testes dos três arquivos afetados passaram (CSS, metadados e notificações). Depois de incluir sons na restauração de preferências, mais 9 testes unitários e o teste de restauração de interface passaram. Sons gerados têm 1,6 s e nenhum sample saturado; o teste real do navegador decodifica e reproduz o arquivo personalizado e confirma persistência após recarregar.

Limite: a interface de aviso flutuante foi simulada no navegador e o código Windows foi compilado; sua exibição real sobre outros aplicativos ainda não foi validada. A instância instalada não foi reiniciada. Credenciais, músicas e configurações reais não foram usadas nos testes de simulação.

Build final: Vite e orçamento de bundle aprovados. Executável Windows com a interface incorporada gerado pelo Tauri CLI em modo debug, sem bundle e offline: target/debug/reverb.exe. TypeScript, ESLint, traduções, consistência dos 95 comandos IPC e git diff --check aprovados. A compilação exigiu liberar somente o cache incremental do Rust após falta de espaço em disco.

## Tema das notificações e integração simplificada

As notificações informativas e a barra de tempo usam o accent do tema; fundo, bordas, textos e botões continuam lendo os tokens claro/escuro. Ícones de sucesso, erro e alerta preservam suas cores semânticas. O popup reage às configurações via settings://changed e acompanha o tema do sistema. A integração prioriza copiar o link e confirmar no aviso, com três passos e explicação de que não há download sem confirmação; favorito, código e testes ficam em uma seção avançada recolhida.

Validação: 6 testes unitários e 11 testes de interface passaram, incluindo cancelamento, expiração, som personalizado, opção avançada, tema escuro/claro e cores efetivas do popup. Axe sem violações nos dois temas. TypeScript, ESLint, traduções, build Vite e git diff --check passaram. Capturas conferidas: integracao-simples.png, notificacao-link-dark.png e notificacao-link-light.png.

Executável Windows atualizado com a interface incorporada pelo Tauri CLI; a instância instalada não foi substituída.
