# Composição de conteúdo nativo

## Barra de status e altura da janela

`StatusBarLayout::new(content, &status)` recebe qualquer implementação de `Layout` e uma `StatusBar` existente. Conteúdo e barra devem ser filhos da mesma janela. O layout mede a altura real da barra a cada distribuição; o padding inferior do conteúdo define a margem antes do rodapé.

Associe uma cópia com `window.set_layout(layout.clone())`. `layout.minimum_width(380).fixed_height(true)` configura os limites usados por `fit_height(&window)`. Sem `fixed_height`, a área flexível continua podendo crescer. Chame `fit_height` depois de mostrar ou ocultar resultados. A biblioteca não define mensagens nem ícones de status.

`fit_height` redimensiona apenas janelas no estado normal. Se estiver maximizada, minimizada ou em tela cheia, preserva sua geometria e atualiza os limites e a distribuição do conteúdo. Após restaurar, uma nova chamada ajusta a altura ao conteúdo atual. Isso evita encolher uma janela que o Windows ainda considera maximizada e mantém o rodapé na base da área cliente.

## Rolagem vertical

`ScrollViewport::new(&parent)?` cria um painel visível, um painel de conteúdo e uma barra vertical nativa. Posicione `viewport()` no layout e crie os controles como filhos de `content()`.

`layout_content(|width| height)` mede a altura em função da largura disponível, repetindo a medição com a barra descontada se necessário. Para essa medição, a altura não deve diminuir quando a largura diminui. Para conteúdo fixo, use `set_content_height(height)`. Ao redimensionar uma composição responsiva, a aplicação reaplica sua medição no `viewport().on_resize`.

A biblioteca controla a visibilidade da barra, a página, os limites, a posição e a roda do mouse no viewport. `scroll_wheel(delta)` pode receber eventos encaminhados pela janela quando o foco estiver em outro controle. `ensure_visible(rect, margin)` recebe um retângulo relativo ao conteúdo, útil ao navegar pelo teclado. Os recursos permanecem vivos até a destruição nativa, mesmo quando handles temporários são descartados.

## Texto por posição Unicode

`StyledText::push_mapped(text, |index, character| format)` agrupa caracteres adjacentes de mesmo estilo. Os índices contam valores escalares Unicode, não bytes nem unidades UTF-16. A aplicação fornece somente texto e estilos; agrupamento, escape e serialização ficam na biblioteca.

Exemplo executável independente: `cargo run --example composition`. Testes nativos de limites, foco, filtragem, redimensionamento, vida útil e ajuste do rodapé: `tests/composition.rs`.
