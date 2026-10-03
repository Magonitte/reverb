# Uso do aplicativo

## Baixar uma faixa

1. No Início, cole a URL na barra de comando e pressione Enter.
2. Confira título, artista, origem e sugestões na pré-visualização.
3. Escolha o perfil de saída. Perfis que recodificam apresentam essa informação.
4. Clique em Baixar/Adicionar à fila e acompanhe **Atividade**.
5. Ao concluir, abra a pasta ou localize a faixa na **Biblioteca**.

Duplicatas podem pedir confirmação. A fila é persistida e pode ser pausada, retomada, reordenada ou cancelada. Jobs com erro mostram a causa e permitem nova tentativa; falhas de ferramentas também podem acionar a recuperação automática.

## Coleções YouTube, Deezer e Spotify

Links de playlist/álbum abrem a tela **Coleção**. No modo YouTube, selecione as entradas desejadas. No modo importação Deezer/Spotify, o catálogo fornece os metadados e o Reverb procura o áudio correspondente no YouTube Music/YouTube; ele não baixa áudio dos servidores do Deezer ou Spotify.

| Estado de casamento | Como proceder                                                  |
| ------------------- | -------------------------------------------------------------- |
| Encontrada          | Selecionada inicialmente; confira os metadados antes de baixar |
| Revisar             | Não selecionada inicialmente; marque somente após revisar      |
| Não encontrada      | Desabilitada para download nesta análise                       |

Use o filtro de casamento, **Limpar seleção** e **Selecionar encontradas** para ajustar a lista. **Baixar selecionadas** enfileira apenas os IDs marcados, incluindo as faixas Revisar explicitamente selecionadas. Os metadados da fonte e o ISRC são preservados no job e no arquivo quando disponíveis. Confiança é uma indicação do algoritmo, não uma audição humana do resultado.

**Sincronizar esta playlist** cria uma coleção acompanhada periodicamente; a sincronização automática enfileira os casamentos aprovados pelo limiar de confiança. A seleção manual de uma importação avulsa não é um filtro permanente da sincronização. Faixas duvidosas não são baixadas automaticamente.

Spotify pode apresentar restrição de acesso. Consulte [Configuração → Spotify](configuracao.md#spotify); o recurso Deezer independe dessas credenciais.

## Playlists sincronizadas

Em **Playlists**, veja as coleções cadastradas, intervalo, última execução e itens. Configure perfil, pasta, limite de itens, exportação M3U8 e comportamento quando uma faixa desaparece da origem.

O diff usa o ID da faixa na fonte; coleções Deezer podem reutilizar um checksum para evitar análises repetidas. Faixas novas são casadas e enfileiradas, enquanto as já associadas mantêm o vínculo. Quando habilitada, a remoção de arquivos deve ser escolhida conscientemente nas opções da coleção.

## Biblioteca e revisão

Use busca e filtros de formato, artista, álbum, data e estado de revisão. A biblioteca também permite importar arquivos/pastas existentes, reexaminar uma pasta e detectar arquivos ausentes. Escolha Lista ou Álbuns para navegar.

A revisão reúne itens cujos metadados precisam de conferência. No editor, confirme título, artistas, álbum, ano e numeração antes de gravar as tags. Alterações de tags/organização podem modificar o arquivo e seu caminho.

## Seguir artistas e encontrar o que falta

1. Abra **Biblioteca → Artistas seguidos → Seguir artista**, procure pelo nome (Enter também pesquisa) e selecione o resultado. Confira foto, número de fãs e ID Deezer para distinguir artistas com nomes iguais. Uma URL de artista Deezer também abre o diálogo.
2. Escolha o que baixar agora: discografia inteira, lançamento mais recente ou somente acompanhar.
3. Escolha a política de lançamentos novos: download automático, notificar e mostrar em Faltando, ou não monitorar.
4. Selecione pelo menos um tipo (álbum, EP ou single), filtro de variantes, perfil e pasta; salve. Uma pasta vazia usa a pasta padrão de Downloads; o botão de pasta abre o seletor.
5. Expanda **Ver lançamentos** no cartão do artista para ver os estados completo, incompleto, ausente ou não monitorado. Use **Verificar agora** para consultar o catálogo novamente.

O cadastro e as preferências são salvos antes da consulta do catálogo. Se a consulta falhar, o artista continua cadastrado e **Verificar agora** permite repetir a consulta. **Editar** preserva as opções salvas; **Cancelar** descarta alterações do formulário.

**Faltando** mostra lançamentos monitorados com faixas ausentes e permite filtrar por artista. **Baixar faltantes** cruza as gravações com a biblioteca, pelo ISRC ou por título/artista/duração, evitando baixar novamente as presentes. Quando álbum e single contêm a mesma gravação, o ISRC permite deduplicar e preferir a organização do álbum. Faixas sem casamento suficientemente confiável não são enfileiradas automaticamente.

Notificações e downloads automáticos exigem o app em execução. O intervalo pode ser ajustado em Integração. Parar de seguir remove o acompanhamento; a interface usa a opção que preserva os arquivos baixados.

## Qualidade e outras fontes

**Melhorar qualidade** procura uma fonte superior para itens elegíveis. O alvo automático é configurado em Downloads. Recodificar um áudio para bitrate maior ou FLAC não melhora a fonte original.

O verificador de provável lossless usa análise espectral e apresenta um espectrograma. O resultado pode ser inconclusivo; ele não é uma certificação da procedência da gravação.

As abas de fontes permitem pesquisar/abrir SoundCloud, Bandcamp, Internet Archive e Jamendo. Download depende da disponibilidade e da permissão indicada pela fonte. Escolha conteúdo que você tenha autorização para obter e mantenha os créditos/licenças aplicáveis.

## Atalhos e integração

- **Ctrl+K:** abre/foca a barra de comando.
- **Espaço:** pausa/retoma a fila fora de campos de edição.
- **Esc:** fecha os diálogos/overlays que oferecem essa ação.

Bandeja, comportamento ao fechar, notificações, monitoramento da área de transferência e atalho global são configuráveis em Geral. Se fechar a janela apenas esconder o app, use a ação de sair na bandeja para encerrá-lo.
