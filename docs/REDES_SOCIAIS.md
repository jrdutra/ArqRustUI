# Integração do pacote de redes sociais

Data: 16/11/2026.

## Resultado

Os 40 desenhos do arquivo `pacote_icones_redes_sociais_xp.zip`, fornecido pelo
responsável, estão disponíveis em `arqrustui::icons`, na categoria
`IconCategory::Social`. Foram acrescentados 39 assets; o `09_Threads.ico` é
idêntico ao `THREADS` existente e reutiliza seus bytes. O catálogo completo
passou de 335 para 374 ícones, mantendo as 22 categorias.

Exemplos de constantes: `YOUTUBE`, `INSTAGRAM`, `FACEBOOK`, `WHATSAPP`, `TELEGRAM`,
`TIKTOK`, `X_TWITTER`, `YOUTUBE_SHORTS`, `BLUESKY`, `WHATSAPP_BUSINESS` e `SIGNAL`.
As constantes funcionam nos mesmos controles que já recebem `IconSource`.
O navegador existente, executado por `cargo run --offline --example icons`,
lista os 40 desenhos na categoria Redes sociais.

Os logotipos `GITHUB` e `GITLAB` anteriores permanecem intactos em `Technologies`;
os recebidos são `GITHUB_SOCIAL` e `GITLAB_SOCIAL`. A variante roxa do Threads é
`THREADS_ALTERNATE`. Os apelidos `21_github`, `22_gitlab` e
`31_threads_alternate` passaram a encontrar os desenhos correspondentes recebidos;
os nomes canônicos anteriores continuam encontrando seus assets originais.

Os sete tamanhos de cada ICO (16, 24, 32, 48, 64, 128 e 256 px) foram preservados.
O PNG de 256 px é lido diretamente do ICO pelo mecanismo existente, sem armazenar
uma segunda cópia. A parte 7 recebeu 15 arquivos; a nova crate própria
`arqrustui-icons-data8` recebeu os outros 24. Nenhum ícone existente foi movido.
A fachada continua opcional pelo recurso `icons`; não há dependências de terceiros.

## Origem e condição visual

A comparação SHA-256 confirmou os bytes dos 40 ICOs com o ZIP recebido, incluindo
o Threads reutilizado. Os 335 ICOs anteriores também mantêm seus hashes.
A inspeção da prancha mostrou cortes e fragmentos nas bordas de vários desenhos
recebidos. Os arquivos foram incluídos conforme solicitados, sem reparos nem
alegação de aprovação visual dos desenhos.

O texto original está em `assets/icons/social/LEIA-ME.txt`; a relação de arquivos,
constantes, destinos e hashes está em `assets/icons/social/ORIGEM.md`. As marcas
pertencem aos respectivos titulares, conforme o documento recebido.
A prancha completa em `assets/icons/preview.png` e a prancha social em
`assets/icons/social/preview.png` foram atualizadas e inspecionadas.

## Validação executada

Ambiente Windows, rustc 1.99.1 e Cargo 1.99.1. Comandos executados na biblioteca
(ou com seu `--manifest-path`):

| Verificação | Resultado |
|---|---|
| `cargo fmt --all --check` | Passou |
| `cargo clippy --offline --workspace --all-targets` | Passou com o aviso preexistente `clippy::map_or_identity` em `src/input/drag/reorder.rs:294`; nenhum aviso novo de código |
| `cargo test --offline --lib icons:: -- --test-threads=1` | 7 testes de catálogo passaram |
| Novo teste de integração social | Passou: 40 ícones carregados nos sete tamanhos, PNGs decodificados e variantes anteriores preservadas |
| Teste geral de carregamento dos ícones | Passou para os 374 assets |
| `cargo test --offline --no-fail-fast -- --test-threads=1 --skip print_and_page_setup_dialogs_cancel_cleanly` | 348 passaram e 33 falharam; os mesmos 33 nomes de testes já falhavam antes desta integração |
| `cargo build --offline --release` | Passou |
| `cargo build --offline --release --no-default-features` | Passou |
| `cargo doc --offline --workspace --no-deps`, com `RUSTDOCFLAGS=-D warnings` | Passou |
| `cargo package --workspace --offline --allow-dirty --no-verify` | Passou; dez pacotes abaixo de 10 MiB; maior pacote com 9.036.359 bytes |
| `cargo metadata --offline --format-version 1` | Dez pacotes locais na UI; 17 na árvore do piloto; nenhuma origem registry/Git nem dependência externa |

O teste de impressão foi omitido nesta execução porque travou nas verificações
anteriores, tanto antes como depois da renomeação da suíte; não foi considerado
aprovado. Os testes de pixels, eventos e integração do ambiente Windows conservam
as 33 falhas registradas em [RENOMEACAO_ARQRUST.md](../../docs/RENOMEACAO_ARQRUST.md).

O empacotamento usou `--no-verify`: foi verificada a geração e o tamanho dos
pacotes, enquanto a compilação foi validada separadamente nos projetos locais.
Nenhum pacote foi publicado. Os logs desta execução estão em `target/social-import/`.
Não houve alteração de layout nem nova captura de tela de aplicativo.

## Suíte e aplicativo

O catálogo e a descrição da suíte foram atualizados em `ARQRUST.md` da raiz.
No piloto, somente a resolução local do novo subpacote foi registrada no lockfile
e a contagem da auditoria foi atualizada; código, telas e fluxos do aplicativo
não foram modificados. Alterações anteriores e histórico Git foram preservados;
nenhum commit foi criado.
