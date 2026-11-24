# Logo padrão das janelas

Em 16/11/2026, o logo fornecido no arquivo `RustArq_Logo_PNG_SVG_ICO.zip` passou a
ser o padrão de toda janela criada por `Window::builder()` sem ícone explícito.
O mesmo vale para diálogos próprios criados por `Dialog`, janelas secundárias
e demais componentes que constroem uma `Window`.

`WindowBuilder::icon(&icone)` substitui o padrão antes de criar a janela;
`window.set_icon(&icone)` o substitui em execução. A origem escolhida fica
armazenada na janela e é reaplicada quando ela recebe uma atualização de DPI.
Cada janela faz sua escolha independentemente, inclusive quando possui uma dona.

O logo está disponível como `icons::DEFAULT_WINDOW_ICON`, mesmo com
`default-features = false`. O ICO original, de 131.515 bytes, é embutido diretamente
na biblioteca, fora do catálogo opcional de 374 ícones. Contém os tamanhos 16, 24,
32, 48, 64, 96, 128 e 256 px; as versões pequena e grande são carregadas no tamanho
pedido pelo Windows para o DPI da janela. O PNG usa o quadro de 256 px desse ICO.
O manifesto inclui explicitamente o ICO no pacote distribuível.

Os arquivos gráficos originais e seus hashes estão documentados em
`assets/branding/README.md`. PNG e SVG foram preservados como fontes gráficas,
sem acrescentar os arquivos maiores à implementação ou ao pacote distribuível.
O exemplo `examples/demo.rs` usa o novo padrão sem mudar sua composição.

Essa regra define o ícone de `Window`. Diálogos nativos do sistema mantêm os
ícones e regras das APIs do Windows. O ícone do arquivo executável de cada
consumidor continua sendo um recurso de compilação definido pelo aplicativo.

## Validação em Windows x64 MSVC

- `cargo fmt --all --check`: passou.
- `cargo clippy --offline --workspace --all-targets`: concluiu com o aviso
  preexistente `clippy::map_or_identity` em `src/input/drag/reorder.rs:294`.
- `cargo test --offline --test default_window_icon -- --test-threads=1`: os
  três novos testes passaram. Foram conferidos janelas, diálogos próprios,
  os oito tamanhos, os dois ícones nativos da janela, prioridade de builder e
  setter, retorno ao logo e reaplicação com DPI simulado.
- O mesmo comando com `--no-default-features`: os três testes passaram.
- Suíte completa com `--no-fail-fast -- --test-threads=1 --skip
  print_and_page_setup_dialogs_cancel_cleanly`: 351 testes passaram e 33 falharam.
  Os nomes dos 33 testes coincidem com as falhas anteriores à alteração.
  O teste de impressão não foi repetido devido ao travamento já registrado nas
  validações anteriores; não foi marcado como aprovado.
- `cargo build --offline --release`, com e sem `--no-default-features`: passou.
- `cargo doc --offline --workspace --no-deps`, com `RUSTDOCFLAGS=-D warnings`:
  passou.
- `cargo package --workspace --offline --allow-dirty --no-verify`: passou;
  o pacote da biblioteca inclui o ICO e o documento de origem. O empacotamento
  não recompilou os arquivos gerados; a compilação foi verificada separadamente.
- Metadata: dez pacotes próprios locais, sem origens de registry ou Git.

Logs em `target/default-icon/`. A mudança de DPI foi simulada, sem troca real de
monitor. Não houve alteração de layout nem nova captura de tela de aplicativo.
Alterações existentes foram preservadas e não foram criados commits.
