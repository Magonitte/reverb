# Contribuir e continuar o desenvolvimento

O projeto é desenvolvido em fases sequenciais. Comece pelo [README](README.md) e pelo [guia de desenvolvimento](docs/desenvolvimento.md). Para implementar ou retomar uma fase, leia também [LEIA-ME_PRIMEIRO](LEIA-ME_PRIMEIRO.md), o [protocolo](plano/00-protocolo-de-execucao.md) e o [PROGRESS](PROGRESS.md).

## Preparar uma mudança

1. Confira o ponto de parada, pendências e decisões registradas.
2. Crie uma branch de trabalho; o padrão usado é `codex/<descricao>`.
3. Leia as especificações da fase e os estudos citados. O projeto segue sala limpa: implemente pelos estudos, sem copiar código dos repositórios de origem.
4. Preserve alterações existentes e mantenha a mudança dentro do objetivo autorizado.

A F16 e o Android possuem planos próprios e não devem ser apresentados como concluídos antes dos respectivos portões. O aplicativo antigo é material histórico de leitura, não o código a modificar.

## Implementar e validar

- Mantenha regras de domínio no core e adaptadores nativos em Tauri; a interface chama a API IPC.
- Atualize tipos gerados, mock e os dois idiomas ao mudar contratos ou telas.
- Inclua testes que comprovem o comportamento e mantenha as asserções das fases anteriores.
- Atualize guias de uso/configuração quando o usuário precisar de uma nova instrução.
- Rode os portões definidos para a fase e registre contagens, falhas/correções e limitações no PROGRESS.
- Não versione caches, binários de ferramentas, dados pessoais, tokens ou chaves privadas.

## Entregar

Use commits descritivos; fases seguem `feat(FNN): descrição` e tag `fase-NN-ok` após validação. A versão publicada utiliza tag `v<x.y.z>`. Uma PR deve explicar o problema, o comportamento final e como foi verificado, incluindo recursos opcionais não testados na rede.

Antes de publicar, confira documentação, CHANGELOG e locks. Acompanhe CI/release e valide os artefatos; enviar uma tag não comprova que o instalador foi produzido corretamente. Ao atualizar uma instalação existente, registre a versão efetiva e confira preservação dos dados.

## Organização da documentação

- `README.md`: entrada pública e visão do produto.
- `docs/`: guias de usuários e desenvolvedores, ligados pelo índice.
- `CONTRIBUTING.md`: fluxo de contribuição.
- `CHANGELOG.md`: alterações por versão.
- `PROGRESS.md`: registro vivo de execução e evidências.
- `plano/`: especificações, estudos e fases, incluindo trabalho futuro.
- `referencias/`: material histórico e visual, separado do código em execução.

Evite duplicar contratos técnicos em vários lugares. Prefira links ao plano/código para detalhes e atualize o guia apropriado quando o comportamento mudar.
