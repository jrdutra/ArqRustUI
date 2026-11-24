# Controles personalizados e integração com Win32

*English: [CUSTOM_CONTROLS.en.md](CUSTOM_CONTROLS.en.md).*

Há três níveis de extensão, do mais simples ao mais baixo nível. Use sempre o mais alto que resolver o problema.

## 1. Composição na aplicação (sem alterar a biblioteca)

Um controle personalizado é normalmente um `Panel` com desenho e eventos próprios:

```rust
use std::cell::Cell;
use std::rc::Rc;
use arqrustui::prelude::*;

/// Indicador circular de progresso — exemplo de controle feito na aplicação.
#[derive(Clone)]
pub struct Gauge {
    panel: Panel,
    value: Rc<Cell<f32>>,
}

impl Gauge {
    pub fn new(parent: &impl Widget, bounds: Rect) -> GuiResult<Gauge> {
        let panel = Panel::builder().bounds(bounds.x, bounds.y, bounds.width, bounds.height).parent(parent).build()?;
        let value = Rc::new(Cell::new(0.0f32));
        let shown = value.clone();
        panel.on_paint(move |canvas| {
            let size = canvas.size();
            let area = Rect::new(4, 4, size.width - 8, size.height - 8);
            canvas.ellipse(area, Some(&Pen::new(Color::GRAY, 2)), None);
            let filled = (area.height as f32 * shown.get()) as i32;
            canvas.clip_rect(Rect::new(area.x, area.bottom() - filled, area.width, filled));
            canvas.ellipse(area, None, Some(Color::BLUE));
        });
        // Leitores de tela passam a enxergar o controle como barra de progresso.
        accessibility::set_custom_role(&panel, AccessibleRole::ProgressBar, "Progresso")?;
        Ok(Gauge { panel, value })
    }

    pub fn set_value(&self, value: f32) {
        self.value.set(value.clamp(0.0, 1.0));
        self.panel.invalidate(); // agenda o redesenho
    }

    /// Expõe o painel para layouts, posicionamento e eventos.
    pub fn widget(&self) -> &Panel {
        &self.panel
    }
}
```

Pontos de atenção:

- **Estado** em `Rc<Cell<_>>`/`Rc<RefCell<_>>`, compartilhado entre o tipo e as closures.
- **Redesenho**: altere o estado e chame `invalidate()`; nunca desenhe fora de `on_paint`.
- **Entrada**: `Panel::builder().focusable(true)` para receber teclado; use `on_mouse_down`, `on_mouse_move`, `on_key_down`…
- **Acessibilidade**: `accessibility::set_custom_role` + `set_accessible_name`.
- Para animação ou vetores pesados, troque `on_paint` por `Direct2DSurface::attach`.
- **Ícones**: `canvas.draw_icon(&icons::WARNING, Point::new(x, y), 16)` desenha na resolução do DPI da superfície; para imagens grandes, `icons::X.image()` + `canvas.with_gdiplus(|g| g.draw_image(..))`.

## 2. Novo controle dentro da biblioteca

Para envolver uma classe nativa ainda não coberta (ou criar um composto reutilizável), siga o padrão dos arquivos em `src/controls/`:

```rust
// src/controls/meu_controle.rs
use super::common::{builder_common, widget_type, ControlOptions};
use crate::core::native;
use crate::core::widget::Reflected;
use crate::core::{Event, EventKind, GuiResult, HandlerId};

widget_type!(
    /// Documentação pública do controle (obrigatória: `#![warn(missing_docs)]`).
    MeuControle
);

/// Construtor de [`MeuControle`].
#[derive(Clone, Debug)]
pub struct MeuControleBuilder {
    opts: ControlOptions,
    // opções específicas…
}

impl MeuControleBuilder {
    builder_common!(); // text, position, size, bounds, parent, enabled, visible, tab_stop, font, tooltip

    /// Cria o controle.
    pub fn build(self) -> GuiResult<MeuControle> {
        // kind, classe nativa, estilo, estilo estendido, focável, id opcional
        let core = self.opts.create("MeuControle", "CLASSE_NATIVA", 0, 0, true, None)?;
        // Notificações que o Windows envia ao pai chegam aqui, já refletidas:
        core.set_reflect(|core, reflected| match reflected {
            Reflected::Command { code, .. } if *code == 0 => {
                core.emit(&Event::Click);
                Some(0)
            }
            Reflected::Notify { code, lparam } => {
                let _ = (code, lparam); // converter `lparam` para a estrutura NM* em um bloco unsafe documentado
                None
            }
            _ => None,
        });
        Ok(MeuControle { core })
    }
}

impl MeuControle {
    /// Inicia a construção (informe o tamanho padrão).
    pub fn builder() -> MeuControleBuilder {
        MeuControleBuilder { opts: ControlOptions::new(120, 24) }
    }

    /// Propriedades usam os helpers de `core::native` em vez de `unsafe` direto.
    pub fn valor(&self) -> i32 {
        native::send(self.core.hwnd(), 0x0400, 0, 0) as i32
    }

    /// Eventos tipados encaminham para `core.on`.
    pub fn on_value_changed(&self, f: impl Fn(i32) + 'static) -> HandlerId {
        let this = self.clone();
        self.core.on(EventKind::ValueChanged, move |_| f(this.valor()))
    }
}
```

Lista de verificação ao adicionar um controle:

1. Constantes, estruturas e funções novas vão para o arquivo da DLL correspondente em `src/ffi/`, com os nomes da documentação da Microsoft; acrescente o `size_of` das estruturas novas em `tests/ffi_layout.rs`.
2. Arquivo em `src/controls/`, `pub mod` + `pub use` em `src/controls/mod.rs` (e no `prelude` de `src/lib.rs`, se for de uso comum).
3. Todo item público documentado; todo bloco `unsafe` com `// SAFETY:`.
4. Coordenadas da API em unidades lógicas (`to_pixels`/`to_logical` com `core.dpi()`).
5. Se o controle tiver cores próprias (como ListView/TreeView), trate o `kind` em `themes::style_control`.
6. Teste de integração em `tests/controls.rs` e um exemplo ou uso no `examples/demo.rs`.
7. Atualize `docs/COMPONENTS.md`, `docs/FEATURE_MATRIX.md` e `AGENTS.md` (e as versões `.en.md` deles).
8. **Genérico**: nada de regras de negócio ou nomes de aplicações consumidoras.
9. Se o controle exibe ícones, aceite `&impl IconSource` e reutilize `WidgetIcon` ou `ManagedImages` de `controls/icon_support.rs` (DPI e ciclo de vida já tratados).

## 3. Mensagens brutas e FFI (API avançada)

Quando falta um recurso pontual, a aplicação pode descer um nível sem modificar a biblioteca:

```rust
use arqrustui::ffi::user32::{SendMessageW, WM_SETREDRAW};

// Suspende o redesenho durante uma carga grande.
// SAFETY: mensagem sem ponteiros para uma janela viva desta thread.
unsafe { SendMessageW(list.handle() as _, WM_SETREDRAW, 0, 0) };
/* … inserir milhares de linhas … */
// SAFETY: idem.
unsafe { SendMessageW(list.handle() as _, WM_SETREDRAW, 1, 0) };
list.invalidate();
```

- `widget.handle()` devolve o `HWND` como `isize` (zero se destruído).
- `widget.on_raw_message(|msg| …)` intercepta mensagens antes da biblioteca (um gancho por widget).
- `arqrustui::ffi` expõe as declarações já existentes; `ffi::dynamic::Library` carrega DLLs e símbolos opcionais.

Se o recurso for de interesse geral, o caminho correto é promovê-lo a uma API segura da biblioteca (nível 2) em vez de espalhar `unsafe` pelas aplicações.
