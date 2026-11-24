# ArqRustUI — guia da biblioteca

*English: [BIBLIOTECA.en.md](BIBLIOTECA.en.md); visão geral em inglês: [README.en.md](README.en.md).*

## 1. Identidade e objetivo

- Diretório: `ArqRustUI`; crate/import: `arqrustui`.
- Versão: 0.1.0; edição 2021; Rust mínimo declarado: 1.80.
- Plataforma: GUI nativa para Windows 10/11. O crate usa `cfg(windows)`; compilar em outro sistema não valida os recursos de interface.
- Objetivo: fornecer janelas, controles, layouts, desenho e integração de interface reutilizáveis por qualquer aplicação Rust.

Implementação própria em Rust, `std` e FFI manual para APIs/DLLs nativas do Windows. Não utiliza crates de GUI ou bindings de terceiros. As APIs do sistema operacional são um requisito de execução, não dependências de crates externos.

## 2. Recursos implementados

| Grupo | Recursos públicos |
|---|---|
| Aplicação e janelas | `Application`, `Window`, diálogos próprios, múltiplas janelas, DPI, foco e eventos. |
| Ícone padrão das janelas | Logo `icons::DEFAULT_WINDOW_ICON` aplicado automaticamente a cada `Window`, inclusive diálogos próprios e sem o recurso opcional `icons`. `WindowBuilder::icon` e `Window::set_icon` substituem o padrão e continuam prevalecendo nas atualizações de DPI. |
| Controles | Botões, labels, campos de texto, seleção, calendários, listas, árvores, tabelas, abas (destacáveis), `DataGrid`, `PropertyGrid` e os desenhados `ToggleSwitch`, `NumberBox`, `RatingControl`, `Card`, `Expander`, `BreadcrumbBar`, `Chart`, `PrintPreview`, `DockHost`/`DockPane` e `TitleBar` (janela sem moldura). |
| Layout | Stack, Grid, Flow, Dock, Anchor, Split, `StatusBarLayout` e `ScrollViewport`. |
| Menus e comandos | `MenuBar`, `MenuStrip`, `Menu`, contexto, atalhos, `CommandBar`, `ToolBar` e `StatusBar`. |
| Desenho e temas | Canvas, fontes, cores, GDI/GDI+ (com vetores em ponto flutuante e gradientes), SVG (`SvgImage`), Direct2D/DirectWrite, animações (`motion`) e temas configuráveis. |
| Texto formatado | `RichTextBox` (com listas, tabelas e imagens), `TextFormat`, `StyledText`, RTF e `apply_highlights` para intervalos sem substituir o texto nem inserir etapas de desfazer. |
| Conteúdo de botões | Ícones, posição relativa e alinhamento horizontal por `content_alignment(TextAlign)`, com margem e truncamento. |
| Sistema | Clipboard (DIBV5, PNG, metarquivos, monitoramento), Shell, bandeja, notificações (com ativador COM), impressão com intervalo e pré-visualização, arrastar/soltar, gestos de toque, IME com lista de candidatos própria, acessibilidade (modelo UIA para controles desenhados) e internacionalização (também dos textos internos), conforme suporte documentado. |
| Trabalho assíncrono | Dispatcher, temporizadores e helpers de tarefas, respeitando a thread de UI. |
| Ícones de redes sociais | 58 desenhos em `IconCategory::Social`, catálogo completo com 392 ícones. O lote com bisel substitui 22 desenhos e acrescenta 18 plataformas, com ICO, PNG de 512 px e SVG; as outras 18 redes anteriores permanecem. Constantes existentes preservadas; origem, créditos e limites SVG em `assets/icons/social/ORIGEM.md`. |

O mapa de APIs fica em [AGENTS.md](AGENTS.md); a lista detalhada e o estado de cada recurso ficam em [COMPONENTS.md](docs/COMPONENTS.md) e [FEATURE_MATRIX.md](docs/FEATURE_MATRIX.md). Não interpretar esta síntese como garantia de que cada recurso esteja completo em todos os ambientes.

## 3. Como utilizar

No `Cargo.toml` de um consumidor em diretório irmão:

```toml
[dependencies]
arqrustui = { path = "../ArqRustUI" }
```

Exemplo com a API pública:

```rust
#![windows_subsystem = "windows"]
use arqrustui::prelude::*;

fn main() -> GuiResult<()> {
    let app = Application::new()?;
    let window = Window::builder().title("Exemplo ArqRustUI").size(640, 360).build()?;
    let button = Button::builder().text("Abrir").icon(&icons::FOLDER_OPEN)
        .content_alignment(TextAlign::Left).parent(&window).build()?;
    let layout = StackLayout::vertical().padding(Thickness::all(16));
    layout.add_with(&button, LayoutParams::new().height(48));
    window.set_layout(layout);
    window.show();
    app.run()
}
```

O caminho é relativo ao manifesto do consumidor; não há descoberta automática de dependências.

## 4. Dependências e organização

**Dependências de crates: nenhuma de terceiros nem de outras bibliotecas ArqRust.** A única entrada em `dependencies` é a crate própria `arqrustui-icons` (opcional, recurso `icons`), que fica dentro desta pasta, em `crates/`, junto com as oito crates de dados `arqrustui-icons-data1` a `-data8`: são os bytes do pacote de ícones da própria ArqRustUI, divididos em partes de até 9 MB só para caber no limite de 10 MiB por pacote do crates.io. Não são outra biblioteca da suíte nem código de terceiros, não têm código além de `include_bytes!` e não dependem de nada. `dev-dependencies` e `build-dependencies` estão vazias; `std` não precisa ser declarada no manifesto. Para isso, `ArqRustUI/Cargo.toml` é também um workspace Cargo com os membros `crates/*` (a raiz da suíte continua sem workspace).

Se uma evolução realmente precisar de outra biblioteca da suíte, ela deve estar em uma pasta irmã e ser declarada exclusivamente por `path = "../ArqRustX"`. O exemplo é uma regra de organização, não uma dependência a adicionar agora. É proibido usar crates.io, outros registries, Git, caminhos absolutos, dependências aninhadas ou código de terceiros copiado para uma pasta local. A restrição abrange dependências opcionais, por plataforma, de testes/build e transitivas. Não criar ciclos.

## 5. Estrutura interna

```text
ArqRustUI/
├── Cargo.toml
├── BIBLIOTECA.md
├── README.md
├── AGENTS.md
├── rustfmt.toml
├── src/
│   ├── lib.rs             API, módulos e prelude
│   ├── core/              aplicação, janelas, eventos, handles e erros
│   ├── controls/          componentes e composição genérica
│   ├── ffi/               declarações nativas do Windows
│   ├── layout/            algoritmos e composições de layout
│   ├── graphics/          desenho, fontes e imagens
│   ├── menus/             menus nativos e desenhados
│   └── outros módulos     dialogs, themes, system, input, accessibility, i18n, icons
├── tests/                 testes unitários/nativos de integração
├── examples/              programas independentes
├── docs/                  arquitetura, componentes, eventos, compatibilidade e segurança
├── crates/                crates próprias do pacote de ícones (fachada + 7 de dados)
└── assets/icons/          origem, tratamento e prancha do pacote de ícones
```

`BIBLIOTECA.md` é o guia padrão. `README.md` complementa a apresentação e o manifesto Cargo aponta para este guia. Cada um desses documentos tem uma versão em inglês ao lado (`*.en.md`).

## 6. Erros, limites e compatibilidade

- Exige Windows para executar e verificar a interface. Componentes visuais pertencem à thread de UI e não são `Send`/`Sync`.
- `Application` deve existir antes da criação dos controles. Usar builders, handles `Clone`, callbacks e `GuiResult` pela API pública.
- Falhas são explícitas; recursos indisponíveis podem retornar `GuiError::Unsupported`. Consultar limitações por versão do Windows e por ambiente.
- `apply_highlights` recebe intervalos UTF-16 do RichEdit: um par CRLF conta como uma quebra, e caracteres fora do BMP usam duas unidades. Parsers da suíte retornam bytes UTF-8; o consumidor precisa converter as posições corretamente.
- O recurso não define gramáticas ou paletas de linguagens; recebe texto, intervalos e estilos fornecidos pelo consumidor.
- Tema escuro, notificações, arrastar/soltar, acessibilidade e outros recursos têm ressalvas de validação e plataforma. Consultar [COMPATIBILITY.md](docs/COMPATIBILITY.md) e a matriz, sem inferir aprovação completa a partir de compilação bem-sucedida.
- Testes de GUI podem depender de ambiente gráfico, APIs e associações do sistema. Registrar os resultados executados, inclusive falhas; contagens históricas da documentação não substituem execução atual.

## 7. Validação

Executar dentro de `ArqRustUI/`:

```powershell
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline
cargo build --offline --release
cargo metadata --offline --format-version 1
```

Os comandos validam esta biblioteca isoladamente. Verificar também os manifestos e a origem dos pacotes: estar no cache/offline não torna um crate de terceiros permitido. Não declarar testes aprovados sem executá-los; registrar limitações do ambiente e falhas reais.

## 8. Orientações para IA e manutenção

- Rustdoc em inglês: módulos, APIs, contratos `Safety`, macros e exemplos
  publicados. Os guias mantêm versões em português e inglês.
- Ler o [AGENTS.md local](AGENTS.md) inteiro antes de alterar ou consumir a API de interface.
- Preservar generalidade: nenhum parser, formato, nome, fluxo ou regra específica de uma aplicação consumidora dentro desta biblioteca.
- Não adicionar frameworks ou bindings de GUI. Integração nativa é implementada em Rust com FFI manual; manter `unsafe` isolado e justificado, ownership/RAII, tratamento de erros e APIs públicas documentadas.
- Usar somente `std` e, se houver necessidade demonstrada, APIs públicas de uma biblioteca própria da suíte em diretório irmão. Não adicionar dependências fictícias ou criar ciclos.
- Ao estender recursos genéricos, atualizar este guia, AGENTS.md, documentação de componentes/matriz (versões em português e `.en.md`), exemplos e testes independentes.
- Validar a biblioteca isoladamente em Windows. Relatar alterações e resultados separadamente de qualquer aplicação consumidora.


