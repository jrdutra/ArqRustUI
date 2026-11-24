# Publicação

*English: [RELEASING.en.md](RELEASING.en.md).*

Como publicar uma versão da ArqRustUI no crates.io.

## Preparação do repositório no GitHub

Use o **diretório completo `ArqRustUI` como raiz do repositório**, preservando seu
histórico Git. Inclua `src/`, `crates/`, `assets/`, `examples/`, `tests/`, `docs/`,
`.github/`, manifestos, lockfile, licenças e arquivos de configuração do Git.
A suíte ArqRust e o aplicativo PocketTools são projetos independentes.
Não envie somente `src/`: o pacote principal precisa das crates locais de ícones.

O [README.md](../README.md) principal está em inglês e usa capturas armazenadas em
`docs/screenshots/`. O campo `readme` do pacote principal aponta para esse arquivo.
Mantenha caminhos relativos das imagens, a origem dos ícones e as atribuições.
Saídas de compilação e caches locais são ignorados; pacotes antigos de trabalho
foram retirados do diretório de fontes e preservados fora do repositório.

Revise `git status` antes de preparar o commit. Este checkout contém alterações
anteriores e subpacotes próprios ainda não rastreados; mantenha esses subpacotes
ao publicar o repositório. A limpeza não deve apagar `.git/`, reescrever histórico
nem descartar alterações de código. O Git transmite o histórico; `.git/` não é
uma pasta para adicionar manualmente pela interface de upload de arquivos do GitHub.

Depois de criar o repositório desejado, configure sua URL real como remoto e
adicione essa URL confirmada ao campo `repository` dos pacotes. Nenhuma URL é
inventada antecipadamente. Enviar fontes ao GitHub não publica crates no crates.io.

Conferência local, dentro da biblioteca:

```powershell
cargo fmt --all --check
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline --doc
cargo build --offline --release --examples
cargo metadata --offline --format-version 1
cargo package --offline --workspace --allow-dirty --no-verify
```

`--allow-dirty` serve à revisão local antes do commit. `--no-verify` confere o
empacotamento sem recompilar o conteúdo extraído e não substitui a validação da
versão final. Inspecione o metadata: todos os pacotes devem ser locais e próprios;
o modo offline sozinho não comprova a origem das dependências.

A execução Windows registrada ainda tem falhas conhecidas (veja README e matriz).
Resolva-as e valide o commit pretendido antes de publicar uma versão. Montar os
pacotes não certifica o funcionamento de todos os recursos na plataforma.

## As dez crates

| Crate | Pasta | Conteúdo |
|---|---|---|
| `arqrustui` | `./` | a biblioteca; o recurso `icons` (padrão) depende de `arqrustui-icons` |
| `arqrustui-icons` | `crates/arqrustui-icons/` | fachada: reexporta as oito crates de dados |
| `arqrustui-icons-data1` … `-data8` | `crates/arqrustui-icons-dataN/` | os bytes dos ícones (`.ico`, `.png`, `.svg`), até 9 MB cada |

O crates.io aceita no máximo 10 MiB por pacote compactado, e o pacote de ícones tem
cerca de 66 MB; por isso ele vem em partes. As dez crates têm **sempre a mesma
versão** e são publicadas juntas, nesta ordem: as oito de dados, a fachada e, por
último, a `arqrustui` (cada uma só pode ir depois das que ela usa).

## Antes de publicar

1. O CI do commit está verde, incluindo o job "Pacotes do crates.io (tamanho)".
2. Troque a versão no `Cargo.toml` da raiz: em `[workspace.package]` e nas nove
   entradas de `[workspace.dependencies]` (é o único lugar; as crates de `crates/`
   herdam a versão com `version.workspace = true`).
3. Em `CHANGELOG.md`, renomeie `[Não publicado]` para a versão com a data e abra
   um `[Não publicado]` vazio acima dela; faça o mesmo em `CHANGELOG.en.md` com
   `[Unreleased]`.
4. Confira os pacotes localmente: `cargo package --workspace` empacota e compila
   cada crate a partir do arquivo `.crate`, resolvendo as crates ainda não
   publicadas entre si.

## Publicar

Com uma versão do Cargo cujo `cargo publish --help` ofereça `--workspace`, um
comando publica as dez em ordem de dependência:

```powershell
cargo publish --workspace
```

Caso contrário, publique individualmente em ordem de dependência:

```powershell
foreach ($n in 1..8) { cargo publish -p "arqrustui-icons-data$n" }
cargo publish -p arqrustui-icons
cargo publish -p arqrustui
```

Depois, crie a tag `vX.Y.Z` no commit publicado.

## Acrescentar ícones depois da primeira publicação

Um ícone nunca muda de crate de dados (o nome da constante tem que continuar
resolvendo na mesma fachada). Ícones novos entram na última crate de dados enquanto
ela tiver espaço (o job de tamanho do CI avisa); quando não couber, crie
`arqrustui-icons-data9` (copie a estrutura de uma existente) e acrescente-a como
dependência e `pub use` em `crates/arqrustui-icons`. O passo a passo está em
"Acrescentar um ícone", em [`assets/icons/README.md`](../assets/icons/README.md).
