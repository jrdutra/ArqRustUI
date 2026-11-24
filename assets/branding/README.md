# Logo das janelas ArqRustUI

Origem: arquivo `RustArq_Logo_PNG_SVG_ICO.zip` fornecido pelo responsável em
16/11/2026. Os arquivos foram copiados sem alterações; somente seus nomes locais
foram padronizados para `arqrust-logo`.

SHA-256 do ZIP: `d024515b934ef2cb5fec7902974ee15b6b8e8dd49b9685bbafccf013817507f5`.

| Arquivo original | Arquivo local | SHA-256 |
|---|---|---|
| RustArq_Logo.ico | arqrust-logo.ico | `9eaf9b26c7bc15b24f10cfb92cd9acef2e6a6f208c18e90167b7dac63df3cedc` |
| RustArq_Logo.png | arqrust-logo.png | `dc608ca6c28b10c887087e0d8f6715db5d0ad3361234e6c52cb912f264db9841` |
| RustArq_Logo.svg | arqrust-logo.svg | `8906a574ee82f6077270c8695dd1498b2d66ad4055efa6b8aff050c578cb66b1` |

O ICO de 131.515 bytes tem oito quadros: 16, 24, 32, 48, 64, 96, 128 e 256 px.
Somente ele é embutido e distribuído com a biblioteca. PNG e SVG originais ficam
como fontes gráficas neste diretório. O PNG usado em `DEFAULT_WINDOW_ICON` é
o quadro de 256 px do ICO, para evitar embutir outra cópia maior.

O logo não pertence ao catálogo opcional de 374 ícones. Ele continua disponível
com `default-features = false` e pode ser substituído pelo programador via
`WindowBuilder::icon` ou `Window::set_icon`. Diálogos próprios usam a mesma regra.
As janelas e os ícones de conteúdo dos diálogos fornecidos pelo sistema operacional
continuam sujeitos às APIs nativas; esta alteração define o padrão de `Window`.
