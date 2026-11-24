# Pacote de ícones

Ícones embutidos na ArqRustUI pelo módulo `arqrustui::icons` (recurso `icons`, ligado
por padrão). Estilo retrô "glossy" inspirado no Windows XP, sem copiar ícones
oficiais.

Esta pasta guarda a documentação do pacote (este arquivo) e a prancha
`preview.png`, com todos os ícones por categoria e o nome da constante. Os
arquivos dos ícones ficam nas crates próprias de `crates/`:

| Onde | Conteúdo |
|---|---|
| `crates/arqrustui-icons-data1` … `-data8` | os 392 ícones com catálogo em ordem alfabética e dados acrescentados sem mover ícones existentes; o crates.io aceita até 10 MiB por pacote. Em cada parte, `ico/`, `png/`, `svg/` e `src/lib.rs` com uma constante `(.ico, .png, .svg)` por ícone |
| `ico/` | `.ico` com 16, 24, 32, 48, 64, 128 e 256 px (quadros PNG com alfa), um por ícone |
| `png/` | PNG transparente de 512 px em 222 ícones, incluindo os 40 desenhos do lote social com bisel. Nos outros 170, o PNG é o quadro de 256 px do `.ico`, sem uma segunda cópia |
| `svg/` | 79 fontes vetoriais: 39 anteriores e 40 do lote social com bisel; três destas contêm texto, ainda não desenhado pelo renderizador próprio |
| `crates/arqrustui-icons` | fachada que reexporta as oito; é a dependência opcional do recurso `icons` |

O nome do arquivo é o nome canônico do ícone (`folder.ico` → `icons::FOLDER`).
A relação nome → categoria → apelidos está em `src/icons/catalog.rs`.

## Origem

Os arquivos vêm do arquivo `pacote_retro_xp_volume_2_refeito.zip`, que reúne
quatro pacotes:

| Pacote | Ícones | Formatos |
|---|---|---|
| `pacote_icones_retro_xp_original` | 55 | ICO, PNG 512 px, SVG |
| `pacote_retro_xp_volume_2_refeito` | 80 | ICO, PNG 512 px |
| `Pacote_Icones_Tecnologia_Desenvolvimento_XP` | 87 | ICO, PNG 256 px |
| `pacote_icones_retro_xp_terceiro_volume_novos` | 72 | ICO, PNG 512 px, atlas |

Um segundo lote veio de três pacotes que traziam só `.ico` (16 a 256 px), e
depois uma versão revisada desses mesmos 100 ícones (`pacote_100_icones_revisados.zip`):

| Pacote | Ícones no pacote | Na biblioteca | Categoria |
|---|---|---|---|
| redes sociais | 40 | 1 (`threads`) | `Social` ("Redes sociais") |
| emojis de expressões | 30 | 19 (e `29_robo` como apelido de `robot`) | `Emoji` |
| gestos de mãos | 30 | 22 (21 em `Gestures`; `emoji_hand_over_mouth` é um rosto e fica em `Emoji`) | `Gestures` ("Gestos") |

Um terceiro lote, `Pacote_66_Icones_Extensoes_ICO_PNG_512.zip`, trouxe 66 ícones
de tipo de arquivo (`.ico` de 16 a 256 px e PNG de 512 px), todos na categoria
`FileTypes` ("Tipos de arquivo"); veja "Tipos de arquivo" abaixo.

Os logotipos de linguagens, plataformas, ferramentas e redes sociais (categorias
`Technologies` e `Social`) e os símbolos de aplicativos dentro de alguns ícones de
tipo de arquivo (Word, Excel, PowerPoint, Acrobat, Photoshop, Android, PHP, Python,
Rust, Java...) são referências visuais a marcas de seus respectivos donos.

## Tratamento aplicado

**Duplicados e quase duplicados (43 removidos).** Nenhum arquivo era idêntico byte
a byte; a comparação foi feita por similaridade visual (imagens recortadas ao
conteúdo e normalizadas) seguida de inspeção. Quando dois pacotes desenhavam o
mesmo objeto com a mesma composição, ficou a versão mais detalhada (em geral a de
512 px). Os nomes dos removidos continuam funcionando em `icons::find` como
apelidos. Ícones com o mesmo tema mas significado diferente foram mantidos
(por exemplo `clock` × `alarm_clock`, `mail` × `email`, `html5` × `html_file`).

| Mantido | Arquivo de origem | Unificados a ele |
|---|---|---|
| `bug` | `bug` (terceiro volume) | `07_09_debug` (tecnologia, cortado) |
| `calendar` | `21_calendario` (volume 2) | `calendar` (original) |
| `camera` | `33_camera_fotografica` (volume 2) | `camera` (original) |
| `chat` | `chat_bubble` (terceiro volume) | `chat` (original) |
| `checklist_done` | `27_checklist_aprovado` (volume 2) | `07_10_checklist` (tecnologia) |
| `clapperboard` | `35_video_claquete` (volume 2) | `clapperboard` (terceiro volume) |
| `cloud` | `06_nuvem` (volume 2) | `02_07_nuvem` (tecnologia), `cloud` (original) |
| `code_window` | `01_06_ide_codigo` (tecnologia) | `code_window` (terceiro volume) |
| `compressed_files` | `48_arquivos_compactados` (volume 2) | `08_05_arquivo_compactado` (tecnologia) |
| `computer` | `01_computador_desktop` (volume 2) | `01_01_desktop_servidor` (tecnologia) |
| `database` | `05_banco_de_dados` (volume 2) | `02_04_banco_dados` (tecnologia), `database` (original) |
| `favorite` | `69_favorito` (volume 2) | `favorite` (original) |
| `folder` | `01_10_pasta` (tecnologia) | `folder` (original) |
| `globe` | `globe` (original) | `07_04_internet` (tecnologia, cortado) |
| `json_file` | `json_file` (terceiro volume) | `08_09_json` (tecnologia, cortado) |
| `key` | `key` (original) | `06_06_chave` (tecnologia, cortado) |
| `laptop` | `02_notebook` (volume 2) | `01_02_notebook` (tecnologia), `laptop` (original) |
| `lock` | `lock` (original) | `06_07_cadeado` (tecnologia, cortado) |
| `microphone` | `40_microfone` (volume 2) | `microphone` (terceiro volume) |
| `music` | `37_musica` (volume 2) | `music` (original) |
| `network` | `network` (original) | `07_01_rede_local` (tecnologia, cortado) |
| `picture` | `31_foto_paisagem` (volume 2) | `picture` (original) |
| `plug` | `power_plug` (terceiro volume) | `06_05_plugue` (tecnologia, cortado) |
| `printer` | `printer` (original) | `55_impressora` (volume 2, cortado) |
| `search` | `search` (original) | `07_08_pesquisa` (tecnologia, cortado) |
| `server` | `02_05_servidor` (tecnologia) | `server` (original) |
| `server_rack` | `04_servidor_rack` (volume 2) | `02_06_cluster_servidores` (tecnologia) |
| `settings` | `59_engrenagem` (volume 2) | `01_08_configuracoes` (tecnologia), `settings` (original) |
| `shield` | `shield` (original) | `06_09_escudo` (tecnologia, cortado) |
| `shield_check` | `61_protecao_ativa` (volume 2) | `06_08_seguranca` (tecnologia) |
| `smartphone` | `smartphone` (terceiro volume) | `08_03_smartphone` (tecnologia, cortado) |
| `tablet` | `08_04_tablet` (tecnologia) | `tablet` (terceiro volume) |
| `terminal` | `01_07_terminal` (tecnologia) | `terminal` (terceiro volume) |
| `tools` | `60_manutencao` (volume 2) | `01_09_ferramentas` (tecnologia) |
| `user` | `66_perfil_masculino` (volume 2) | `user` (original) |
| `users` | `68_grupo_de_usuarios` (volume 2) | `users` (original) |
| `video_camera` | `34_filmadora` (volume 2) | `video` (original) |
| `wifi` | `10_wifi` (volume 2) | `07_05_wifi` (tecnologia), `wifi` (original) |

**Terceiro volume recortado de novo (64 ícones).** Os arquivos individuais deste
pacote ocupavam só ~27% da tela de 512 px (a 16 px o desenho teria ~4 px) e vários
traziam lascas do ícone vizinho na borda. Eles foram recortados novamente do
`atlas.png` do próprio pacote (cerca de 150 px por ícone), separados por
componentes conexos, centralizados com 88% de ocupação e gravados em PNG de
256 px e ICO de 16 a 256 px.

**Fragmentos removidos (4 ícones).** `disc_download`, `compressed_files` e
`kafka` tinham uma faixa fina de outro ícone na borda inferior;
`sync` tinha um pedaço de cadeado ao lado das setas (removido e o desenho
recentralizado). Os demais arquivos foram copiados sem alteração.

**Segundo lote.** Os ícones de redes sociais entraram como vieram (o `.ico`
original e o quadro de 256 px dele como `.png`). Emojis e gestos foram
recentralizados com 88% de ocupação, como o terceiro volume. Quatro arquivos
eram o mesmo desenho de um ícone que já existia e viraram apelidos:
`21_GitHub` → `github`, `22_GitLab` → `gitlab`, `31_Threads_Alternate` →
`threads` e `29_robo` → `robot`. Os nomes seguem o desenho: o arquivo
`10_apontando_esquerda` é `hand_point_left`.

**Auditoria de integridade.** Todos os ícones passaram por inspeção ampliada em
fundo magenta e por medidas automáticas. A mais confiável é a da borda dura: um
contorno desenhado sempre tem antisserrilhado, então um trecho reto em que o alfa
salta de 0 para 255 de um pixel para o outro é sinal de corte. Também foram
medidos trechos retos longos nos limites do desenho, contorno escuro nas bordas e
componentes soltos. Ficou de fora todo ícone com desenho cortado, incompleto,
borrado ou com sobra do vizinho; eles voltam quando forem gerados de novo.

Onze ícones defeituosos tinham uma duplicata íntegra que fora unificada a eles;
a duplicata passou a ser o ícone e o arquivo defeituoso ficou só como apelido:
`lock`, `key`, `search`, `shield`, `globe`, `network` e `printer` (agora do
pacote original, com SVG) e `json_file`, `smartphone`, `plug` e `bug` (agora do
terceiro volume).

Retirados por defeito (arquivo de origem entre parênteses):

- tecnologia, topo cortado: `ssl_certificate` (`06_10_certificado_ssl`), `soap`
  (`06_04_soap`), `ini_file` (`08_11_ini`), `yaml_file` (`08_10_yaml`), `zip_file`
  (`08_06_arquivo_zip`), `graphql` (`06_03_graphql`), `postgresql`
  (`05_05_postgresql`), `sql_server` (`05_07_sql_server`), `cloud_global`
  (`07_06_nuvem_global`), `api` (`06_01_api`), `patch` (`07_11_correcao`) e
  `jwt` (`06_11_jwt`);
- tecnologia, linha de texto duplicada/borrada: `text_file`
  (`01_05_arquivo_texto`), `log_file` (`08_07_log`) e `sql_file` (`08_08_sql`), os
  dois últimos também com o topo cortado;
- volume 2: `firewall_secure` (`64_firewall_seguro`, ponta do escudo cortada) e
  `org_chart` (`80_organograma`, conector cortado à esquerda);
- terceiro volume: `smartwatch` (pulseira cortada no alto), `color_wheel` (base
  cortada; na cartela ela encostava no carimbo) e `route` (`route_pins`, trajeto
  incompleto);
- redes sociais (versão revisada): todos menos `09_Threads`. `17_Vimeo` tem uma
  lasca do vizinho à esquerda; os demais têm faixas do vizinho, cortes ou as
  duas coisas;
- emojis (versão revisada), lado ou topo cortado: `05_apaixonado`,
  `06_oculos_escuros`, `10_chorando`, `11_bravo`, `12_dormindo`, `18_confuso`,
  `22_tonto`, `23_carinhoso`, `24_sorriso_malicioso`, `27_anjinho`, `29_robo` e
  `30_coco_sorridente` (os que já estavam na biblioteca, como `emoji_poop`, são
  da primeira versão e estão íntegros; `emoji_confused` saiu);
- gestos (versão revisada), corte: `05_acenando`, `08_apontando_baixo`,
  `09_apontando_direita`, `14_aperto_de_maos`, `15_dedos_cruzados`,
  `17_toque_de_punhos`, `21_saudacao_vulcana`, `26_duas_maos_levantadas` e
  `30_pare` (`hand_fist_bump` saiu; `hand_fingers_crossed` e `hand_vulcan` são da
  primeira versão e estão íntegros).

**Tamanho visual uniforme.** Os ícones cujo desenho ocupava menos de 84% do quadro
(39 do pacote original, com margens transparentes largas, e `mongodb`) foram
ampliados para ocupar 88%, como os demais — senão pareciam menores que os vizinhos
no mesmo botão, lista ou barra. PNG e ICO foram refeitos a partir do PNG de 512 px;
no SVG só o `viewBox` mudou (o desenho vetorial é o mesmo).

**Tipos de arquivo (66 ícones).** O pacote de extensões (áudio, banco de dados,
compactados, documentos, executáveis, imagens, programação e vídeo) entrou com os
nomes `<extensão>_file` (`pdf_file`, `xlsx_file`, `7z_file` → `SEVEN_ZIP_FILE`) e a
extensão como apelido quando ela não era nome nem apelido de outro ícone (`cpp`,
`java`, `js` e `ts` continuam sendo os logotipos; `icons::for_extension` resolve
a extensão para o arquivo).

- *Repetidos.* Nenhum ícone do pacote repete outro do próprio pacote (`jpeg`/`jpg`,
  `doc`/`docx`, `ppt`/`pptx`, `xls`/`xlsx` e `ts`/`tsx` são desenhos diferentes, com
  a etiqueta de cada extensão). Cinco tinham o mesmo significado de ícones que já
  existiam: `css_file`, `html_file`, `javascript_file`, `json_file` e `xml_file`.
  Como nos lotes anteriores, ficou a versão mais detalhada (a de 512 px), que é
  também a da mesma família dos outros 61: os cinco mantêm nome, constante e
  apelidos e passaram para a categoria `FileTypes`; o desenho antigo (256 px)
  saiu. O apelido `rar`, que era de `compressed_files` (os volumes empilhados),
  passou para o ícone do arquivo `.rar`. `zip_folder` (pasta com zíper) e
  `zip_file` (arquivo `.zip`) são coisas diferentes e ficaram os dois.
- *Cortes reparados (27 ícones).* As folhas de onde o pacote foi recortado
  deixaram bordas retas em que o contorno some de uma vez (a medida de borda
  dura da auditoria acima). Em 17 ícones o corte tirava a lateral da página e a
  ponta da etiqueta (de ~2 a ~15 px): `ai`, `avi`, `bmp`, `cs`, `html`, `jpg`,
  `m4a`, `py`, `yaml` e `zip` (lado direito), `mkv`, `psd`, `webm` e `webp` (lado
  esquerdo), e só a ponta da etiqueta em `apk`, `bat`, `exe`, `msi`, `tar` e
  `xlsx`. Como a página e a etiqueta são simétricas, o lado cortado foi refeito
  espelhando o lado intacto em torno do eixo do desenho, com uma transição de
  5 px; no alto, o canto dobrado continua a diagonal da dobra (deslocamento
  subpixel da última linha intacta, unida ao contorno vertical no canto) e, do
  lado esquerdo, um arco tangente à borda ajustado às linhas intactas do canto
  arredondado. Em `css`, `dll`, `gif`, `html`, `jpeg`, `jpg`, `json`, `png`,
  `webp` e `xml` o rodapé estava aparado reto 2 px abaixo do contorno; ganhou
  duas linhas de antisserrilhado. Os reparados foram recentralizados pelo
  contorno e o `.ico` deles foi refeito do PNG (16 a 256 px, quadros PNG); os
  outros 39 `.ico` e PNGs vieram do pacote sem alteração. Todos foram conferidos
  ampliados em fundo magenta e numa `ListView` a 16, 32 e 48 px.
- *Categoria do M4A.* O pacote punha `M4A` em "Vídeo"; aqui todos os tipos estão
  na mesma categoria, então isso não muda nada.

**Nomes.** Em inglês, minúsculos e com `_`, como o resto da API; os nomes de
arquivo originais (por exemplo `05_banco_de_dados`) são apelidos aceitos por
`icons::find`.

## Redes sociais fornecidas em 16/11/2026

Esta seção registra a primeira importação XP. A atualização com bisel descrita
abaixo substituiu 22 desses desenhos e acrescentou 18 plataformas.

O arquivo `pacote_icones_redes_sociais_xp.zip` foi incorporado por solicitação
explícita do responsável: os 40 desenhos estão disponíveis em `IconCategory::Social`.
São 39 novos assets e o reaproveitamento de `THREADS`, cujo `.ico` é idêntico ao
`09_Threads.ico` recebido. O catálogo passa a ter 374 ícones.

Os logotipos existentes `GITHUB` e `GITLAB` permanecem intactos na categoria
`Technologies`; os desenhos recebidos são `GITHUB_SOCIAL` e `GITLAB_SOCIAL`.
`THREADS_ALTERNATE` mantém a variante roxa separada de `THREADS`.
Os apelidos `21_github`, `22_gitlab` e `31_threads_alternate` agora encontram os
desenhos correspondentes deste pacote. Os demais nomes originais, sem `.ico`,
também podem ser usados em `icons::find`.

Os ICOs foram copiados sem alterações e já contêm os sete tamanhos, de 16 a 256 px.
O PNG é o quadro de 256 px do próprio ICO, sem duplicar os bytes.
A parte 7 recebeu 15 novos ícones até seu limite; os outros 24 estão na nova parte 8.
A importação não adiciona código nem dependências de terceiros.

A inspeção visual confirmou os cortes e fragmentos nas bordas de diversos arquivos
recebidos, já descritos no histórico da auditoria acima. Eles foram preservados
conforme o pacote solicitado, sem reparos ou alegação de aprovação visual.
A lista de origem, destinos e SHA-256 está em [social/ORIGEM.md](social/ORIGEM.md);
o texto original acompanha em [social/LEIA-ME.txt](social/LEIA-ME.txt).
[Prancha atual das redes sociais](social/preview.png).

## Atualização de redes sociais com bisel 3D

O novo arquivo fornecido `arqrustui-social-icons-bevel-3d.zip` trouxe 40 desenhos:
22 substituem plataformas existentes e 18 são novos. A categoria Social tem agora
58 ícones, e o catálogo completo tem 392. Os 18 desenhos anteriores sem versão
nesse arquivo continuam disponíveis. Nomes e constantes existentes foram mantidos;
`facebook_messenger` é um apelido de `MESSENGER`.

ICO multirresolução, PNG RGBA de 512 px e SVG foram preservados byte a byte nos
mesmos subpacotes dos ícones substituídos. Os novos ficam na parte 8. Os ícones
e o logo padrão de janela que não pertencem a esse lote foram preservados.

O lote inclui arte Font Awesome Free sob CC BY 4.0. Créditos, licença recebida e
hashes acompanham os subpacotes de dados afetados; o código Rust próprio segue
MIT, sem implementação ou dependência de terceiros. Consulte
[ORIGEM.md](social/ORIGEM.md) e [ICON_LICENSES.md](../../docs/ICON_LICENSES.md).
O histórico XP está em [ORIGEM_XP_2026-11-16.md](social/ORIGEM_XP_2026-11-16.md).

Os ICOs e PNGs são usados normalmente pelos controles. As fontes SVG de BeReal,
KakaoTalk e Lemon8 contêm texto, fora do subconjunto suportado pelo renderizador;
esses três desenhos devem usar ICO/PNG quando for necessário preservar o texto.

## Acrescentar um ícone

1. Escolha a crate de dados: a última (`crates/arqrustui-icons-data8`) enquanto
   ela tiver espaço; um ícone já publicado nunca muda de crate. Se não couber,
   crie a próxima (`-data9`, copiando a estrutura de uma existente) e
   acrescente-a em `[workspace.dependencies]` no `Cargo.toml` da raiz, como
   dependência de `crates/arqrustui-icons` e no `pub use` do `src/lib.rs` dela.
2. Grave `nome.ico` (de preferência com 16, 24, 32, 48, 64, 128 e 256 px) em
   `ico/` dessa crate, `nome.png` em `png/` (dispensável se for igual ao quadro de
   256 px do `.ico`) e, se houver, `nome.svg` em `svg/`.
3. No `src/lib.rs` da crate, acrescente
   `pub const NOME: Bytes = (include_bytes!("../ico/nome.ico"), Some(include_bytes!("../png/nome.png")), None);`
   (`None` no PNG para usar o quadro do `.ico`; `Some(include_bytes!("../svg/nome.svg"))`
   no último campo se houver SVG).
4. Acrescente uma linha `icon!(NOME, "nome", Categoria, ["apelido"]);` em
   `src/icons/catalog.rs` e inclua `&NOME` em `ALL`, mantendo a ordem alfabética.
5. Se for um tipo de arquivo, acrescente as extensões em `EXTENSIONS` (no fim de
   `catalog.rs`, em ordem alfabética).
6. Rode `cargo test`: os testes conferem nomes únicos, apelidos sem ambiguidade,
   os sete tamanhos do `.ico` e que cada ícone carrega em `.ico` e `.png`. O job
   de tamanho do CI (ou `cargo package --workspace`) confere que cada crate
   continua abaixo do limite do crates.io.

Para ícones da própria aplicação, sem alterar a biblioteca, use
`IconAsset::new("nome", include_bytes!(..), include_bytes!(..), None)`.

Ao regenerar `preview.png`, mantenha o formato atual (12 ícones por linha,
agrupados por `IconCategory`, com o nome da constante).
