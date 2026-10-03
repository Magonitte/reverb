# Instalação e atualização

## Windows

1. Abra [Releases](https://github.com/Magonitte/reverb/releases) e escolha a versão desejada.
2. Baixe o instalador Windows x64 com final `-setup.exe`.
3. Execute o instalador e abra o Reverb pelo menu Iniciar.
4. Complete a configuração inicial. O primeiro preparo das ferramentas precisa de internet.

O instalador é por usuário. A instalação padrão fica em `%LOCALAPPDATA%\Reverb`; os dados ficam em `%APPDATA%\com.reverb.desktop`. O aplicativo usa Microsoft WebView2. Se faltar esse runtime, instale-o antes de tentar abrir o app novamente.

As assinaturas dos artefatos de atualização são verificadas pelo atualizador do Reverb. Isso é diferente de um certificado de editor reconhecido pelo Windows; uma assinatura de atualização não garante que o SmartScreen deixará de apresentar avisos.

## Linux

São publicados pacotes **DEB** e **AppImage**. A compilação Linux usa Ubuntu 22.04 no CI; o teste de interface nativa automatizado completo é executado no Windows.

Para o DEB, substitua o nome abaixo pelo arquivo baixado:

```sh
sudo apt install ./nome-do-arquivo.deb
```

Para o AppImage:

```sh
chmod +x ./nome-do-arquivo.AppImage
./nome-do-arquivo.AppImage
```

Os dados ficam normalmente em `~/.local/share/com.reverb.desktop`, ou em `$XDG_DATA_HOME/com.reverb.desktop` quando essa variável estiver definida. Não há pacote macOS publicado por este projeto atualmente.

## Atualizar

Use **Configurações → Atualizações** para consultar a versão e instalar uma atualização disponível. O app consulta os artefatos publicados no GitHub, verifica a assinatura e executa a instalação. Também é possível usar o instalador de uma versão mais recente.

As músicas ficam na pasta de saída configurada; atualizar o executável não deve remover essa pasta ou o banco de dados. Antes de mudanças de ambiente, use a exportação de dados em **Configurações → Avançado** e guarde também uma cópia dos arquivos de música.

## Dados e cópias de segurança

| Local dentro da pasta de dados | Conteúdo                                              |
| ------------------------------ | ----------------------------------------------------- |
| `reverb.db`                    | Configurações, jobs, biblioteca, playlists e artistas |
| `tools/`                       | Ferramentas gerenciadas e versões                     |
| `logs/`                        | Registros de diagnóstico                              |
| `tmp/`                         | Arquivos temporários de processamento                 |
| `cache/`                       | Cache local                                           |
| `backups/`                     | Cópias de segurança gerenciadas                       |

A pasta de saída padrão é a pasta de músicas do sistema com a subpasta `Reverb`. O caminho pode ser alterado nas configurações. **Exportar dados do app não substitui uma cópia das músicas.**

A exportação integrada remove segredos dos dados exportados. Em uma nova instalação, configure novamente as credenciais necessárias. Uma cópia manual da pasta inteira pode conter segredos e deve ser protegida.

Para uma cópia manual do banco, encerre o app e copie a pasta de dados completa, incluindo eventuais arquivos SQLite `-wal` e `-shm`. Uma cópia feita enquanto há gravações pode ficar inconsistente.

## Modo portátil

O executável reconhece um arquivo `portable.txt` ao seu lado. Nesse modo, os dados ficam em `data/` junto ao executável. Use uma pasta gravável e mantenha esse conjunto unido ao mover o app. Os instaladores publicados seguem o modo normal; criar o marcador não move automaticamente dados de uma instalação anterior.

Para compilar a partir do código, veja [Desenvolvimento](desenvolvimento.md).
