# Atualização das redes sociais com bisel 3D

Data: 16/11/2026.

## Resultado e compatibilidade

O arquivo fornecido `arqrustui-social-icons-bevel-3d.zip` trouxe 40 desenhos em
ICO, PNG RGBA de 512 px e SVG. Foram substituídos 22 desenhos existentes e
acrescentadas 18 plataformas. A categoria Social contém 58 ícones e o catálogo
completo contém 392, em 22 categorias, com 79 fontes SVG.

As novas constantes são `BEREAL`, `BILIBILI`, `DOUYIN`, `KAKAOTALK`, `KICK`,
`KUAISHOU_KWAI`, `LEMON8`, `LIKEE`, `NEXTDOOR`, `ODNOKLASSNIKI`, `QQ`, `QUORA`,
`RUMBLE`, `TUMBLR`, `VK_VKONTAKTE`, `WEIBO`, `XIAOHONGSHU_REDNOTE` e `ZHIHU`.
Os nomes anteriores foram preservados. O arquivo `facebook_messenger` substitui
o desenho de `MESSENGER`, mantendo essa constante e seus apelidos; a busca também
aceita `facebook_messenger`. `kwai`, `vk`, `vkontakte`, `rednote`, `xiaohongshu`,
`be_real` e `kakao_talk` são apelidos adicionais para as novas plataformas.

As outras 18 plataformas anteriores sem substituto no pacote permanecem
disponíveis. Nenhum ícone existente mudou de subpacote: os substituídos ficam
nas partes 6, 7 e 8; as novas plataformas ficam na parte 8. O logo padrão das
janelas e os recursos fora desse lote foram preservados. O código de carregamento
não mudou e não há novas dependências de pacote.

## Origem, créditos e formatos

Os 120 arquivos gráficos recebidos foram conferidos por SHA-256 com suas cópias
integradas. Dos recursos existentes, somente os 22 ICOs correspondentes foram
substituídos; 573 outros arquivos gráficos mantiveram seus hashes. Os PNGs e SVGs
novos foram incluídos integralmente. A origem, destinos e hashes estão em
`assets/icons/social/ORIGEM.md`; o histórico XP foi preservado em
`assets/icons/social/ORIGEM_XP_2026-11-16.md`.

O lote recebido inclui arte aplicável Font Awesome Free, Fonticons, Inc.,
Copyright 2024, sob CC BY 4.0, com bisel, extrusão, gradientes e cores presentes
nos arquivos fornecidos. A integração não alterou os desenhos. A licença recebida,
os créditos e a origem acompanham os três subpacotes de dados afetados, inclusive
nos pacotes distribuíveis. Seus manifestos indicam `MIT AND CC-BY-4.0`; o código
Rust próprio continua MIT. Nenhuma fonte, JavaScript ou implementação de terceiros
foi adicionada. Veja `docs/ICON_LICENSES.md` e os arquivos de licença do lote.

Os sete tamanhos ICO (16, 24, 32, 48, 64, 128 e 256 px) e os PNGs fornecidos são
usados normalmente nos controles. As fontes SVG de BeReal, KakaoTalk e Lemon8
contêm texto, que o renderizador próprio ainda não desenha; seus ICOs e PNGs são
completos. Os demais limites do subconjunto SVG da biblioteca continuam válidos.
A renderização não vazia de um SVG não comprova fidelidade integral ao arquivo.

As pranchas completa e social foram regeneradas e inspecionadas:
`assets/icons/preview.png`, `assets/icons/social/preview.png` e
`assets/icons/social/preview-bevel.png`. A prancha social exibe os 58 desenhos;
a última mostra somente os 40 desenhos do novo pacote.

## Validação

Ambiente: Windows x64 MSVC, rustc/Cargo 1.99.1.

- `cargo fmt --all --check`: passou.
- `cargo clippy --offline --workspace --all-targets`: concluiu com o aviso
  preexistente `clippy::map_or_identity` em `src/input/drag/reorder.rs:294`.
- Sete testes de catálogo passaram, incluindo nomes únicos, aliases, ordenação,
  392 assets, 58 sociais, 79 SVGs e todos os tamanhos ICO.
- O teste novo `bevel_social_icons_have_png_512_svg_and_searchable_new_platforms`
  passou para os 40 desenhos: PNGs decodificados a 512 px, SVGs analisados e
  renderizados a 32 px, buscas novas e compatibilidade de Messenger.
- O teste `every_bundled_svg_icon_renders` foi atualizado para 79 fontes e passou,
  conferindo pixels opacos em cada resultado.
- Os testes de carregamento de todos os ícones e dos sete tamanhos de toda a
  categoria Social passaram. O teste social anterior foi ampliado para 58 assets
  e respeita os diferentes tamanhos PNG dos dois lotes.
- Builds release, com e sem `--no-default-features`, passaram.
- Rustdoc do workspace com `RUSTDOCFLAGS=-D warnings` passou.
- `cargo package --workspace --offline --allow-dirty --no-verify`: passou; dez
  pacotes abaixo de 10 MiB. Foi conferida a presença de `FONT_AWESOME_LICENSE.txt`,
  `THIRD_PARTY_NOTICES.md` e `SOCIAL_ICONS_ORIGIN.md` nos três pacotes afetados.
  O empacotamento não recompilou os arquivos gerados; a compilação foi validada
  separadamente nos projetos locais.
- Cargo metadata: dez pacotes locais na biblioteca e 17 na árvore consumidora;
  nenhuma origem registry/Git ou dependência externa.

O teste de impressão foi omitido nas execuções completas devido ao travamento
preexistente já registrado; não foi considerado aprovado. Resultados finais
dessas execuções são registrados abaixo. As falhas nativas anteriores de outros
componentes não foram corrigidas como parte desta atualização de recursos.

A execução final de `cargo test --offline --no-fail-fast -- --test-threads=1
--skip print_and_page_setup_dialogs_cancel_cleanly` teve **353 aprovados e 32
falhas**, incluindo doctests. Os 32 nomes de testes com falha pertencem ao conjunto
de 33 falhas preexistentes. O teste `uia_client_sees_the_menu_bar_in_the_window`,
que falhava anteriormente, passou nesta execução; isso não é uma alegação de
correção de sua instabilidade. Não restou falha nova após ampliar a contagem
esperada do teste geral de renderização SVG.

A aplicação consumidora foi recompilada em release para validar a integração;
o teste que confere os oito quadros de seu ícone de executável também passou.
Nenhum código ou layout da aplicação foi modificado.

Logs em `target/bevel-import/`. Não houve mudança de layout nem nova captura de
tela de aplicativo. Alterações anteriores e histórico foram preservados; nenhum
commit ou publicação foi realizado.
