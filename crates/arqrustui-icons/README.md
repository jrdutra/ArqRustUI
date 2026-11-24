# arqrustui-icons

*English: the bytes of the 392-icon pack of [ArqRustUI](https://docs.rs/arqrustui)
(`.ico` 16–256 px, `.png`, 79 `.svg`), split into eight data crates of up to 9 MB
to fit crates.io's 10 MiB package limit. Do not depend on it directly: use the icons
through `arqrustui::icons` (the `icons` feature of `arqrustui`, on by default).
See `README.en.md` in the ArqRustUI repository.*

Os bytes do pacote de 392 ícones da [ArqRustUI](https://docs.rs/arqrustui): cada ícone
em `.ico` (16, 24, 32, 48, 64, 128 e 256 px, quadros PNG com alfa), `.png` (512 px em
boa parte deles) e, em 79, também `.svg`.

**Não use esta crate diretamente.** Os ícones chegam prontos — com nome, categoria,
apelidos, busca por nome ou extensão e conversão para `HICON` e imagem — em
`arqrustui::icons`, pelo recurso `icons` do crate `arqrustui` (ligado por padrão):

```toml
[dependencies]
arqrustui = "0.1"
```

```rust,ignore
use arqrustui::prelude::*;
let icone = icons::for_path("relatório.pdf").unwrap_or(&icons::FILE);
```

O pacote tem cerca de 66 MB e o crates.io aceita até 10 MB por crate; por isso os
dados ficam em oito crates de até 9 MB (`arqrustui-icons-data1` a
`arqrustui-icons-data7`), que esta reúne. Cada constante é `(.ico, .png, .svg)`; `None`
no `.png` indica que o PNG é o próprio quadro de 256 px do `.ico`, guardado uma vez só.

Origem dos ícones, tratamento aplicado e como acrescentar ícones: `assets/icons/README.md`
no repositório da ArqRustUI.

Código Rust: MIT. O lote social com bisel inclui arte Font Awesome Free sob
CC BY 4.0; os subpacotes de dados afetados acompanham a licença recebida,
THIRD_PARTY_NOTICES.md e SOCIAL_ICONS_ORIGIN.md. As marcas são de seus titulares.
