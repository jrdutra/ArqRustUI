# Origem atual dos ícones de redes sociais

Atualização em 16/11/2026 pelo arquivo fornecido `arqrustui-social-icons-bevel-3d.zip`.
SHA-256 do ZIP: `979439b5c64cb2cba2ae5cce3cbb5b5d8ede4983b7f75efc81e8dc32297e1abf`.

22 desenhos existentes substituídos e 18 novos; o catálogo tem 392 assets,
58 na categoria Social. As outras 18 redes sociais permanecem da importação XP,
registrada em [ORIGEM_XP_2026-11-16.md](ORIGEM_XP_2026-11-16.md).

ICO, PNG 512x512 e SVG do novo lote foram preservados byte a byte. `facebook_messenger`
é exposto pela constante existente `MESSENGER` e por esse novo apelido, sem quebrar a API.

Arte aplicável: Font Awesome Free, Fonticons, Inc., Copyright 2024, CC BY 4.0.
O pacote recebido acrescenta cores, bisel, extrusão e gradientes. Veja
[licença recebida](FONT_AWESOME_LICENSE.txt) e [texto original](README_BEVEL_ORIGINAL.txt).
As marcas pertencem aos respectivos titulares. Nenhuma fonte ou implementação
de terceiros foi incorporada. O código Rust da biblioteca continua sob MIT.

O renderizador SVG próprio não suporta texto; BeReal, KakaoTalk e Lemon8 contêm
elementos textuais. Seus ICOs e PNGs fornecidos são completos e usados normalmente
nos controles; a renderização vetorial dessas três fontes mantém essa limitação.

| Origem | Constante | Parte | Ação | SHA-256 ICO | SHA-256 PNG | SHA-256 SVG |
|---|---|---|---|---|---|---|
| baidu_tieba | `BAIDU_TIEBA` | arqrustui-icons-data7 | Substituído | `0f90e29136acfe44a1b6c6bdf746ccc969247393d834b80f07975e5a31d566cd` | `8b62d499df3a81f8cdd760acb6ed81889c833cf1f3a0f7b617b0efd869db1690` | `e259e2ea40d8349c3db06183c69ff0b4d163c0e6590b4e74a3c05145b7d869ce` |
| bereal | `BEREAL` | arqrustui-icons-data8 | Novo | `665779e1f4dd14414c3212dd0ca861d452180377d138a88531d90727aa373247` | `d6da0f08d46bbe5dbc8a169c11aeec9e92fee99ba7cf46410056bacae2f80b57` | `23b10608922746105b601d38829785f6a2b85c11a1fe8aae5b7fa33fc0c433ba` |
| bilibili | `BILIBILI` | arqrustui-icons-data8 | Novo | `f86f7ab38b42d48095870f2cfedf5309ff657c4498a1b6e61874dde4c85d3e46` | `afcbd4b92b77ccec55084f23a92be047d37d0e318ec4884d3aec3f5563bee3bc` | `70d6c7726c0ea4f80516aa159819392c40f865b59744ebb4dc857c2c645b0e48` |
| bluesky | `BLUESKY` | arqrustui-icons-data7 | Substituído | `9079fdbffe0a943df5de26879578f08c9cdd87e1911da82826b5917e679443ac` | `9bd9eb4cfaabbfdff2559a27ccf9485e9d2b0db1fdc5e123ec69b750e51f51be` | `b1817a70c4cc1e2dff1a48bf62d115a5461149a130ce3174861c808380b6ffec` |
| discord | `DISCORD` | arqrustui-icons-data7 | Substituído | `4e875c33393a2f50cde16a9021817c478604e10b7c784d03cf45fc852ce0365e` | `d8d2d4df4d68af6149caa061b77a522f423b5f8299590a1c41d7c73b4ad97567` | `6eebbfc7cffe2d143460aac07d4c9d206998ea1c0f1b40604c7fc25177a98ea2` |
| douyin | `DOUYIN` | arqrustui-icons-data8 | Novo | `45fbcd7a759d4e333f0974518a47a2d20455ef295e98e1d3e860ce2cc28e7b3d` | `33189b0bbaddd3b551f787e44009209f61432763845ce7c6af7a68f649447e83` | `db0365776fd8226e9e19b1a78953e397d0f7bd5aa70f07cf62a107e00cb2f401` |
| facebook | `FACEBOOK` | arqrustui-icons-data7 | Substituído | `3209377f8e9ff95f7e7cc1f225a140629348fa7344dfe5e1309c0e6e799ba8fc` | `7d660714943fd4434957c9ae82aa14e1e22c0d9a5124836d039957d8ff5a6521` | `4dc62feeede47663fe9f649992b5c7a41cb584c2f1057bf2907b054054ff3367` |
| instagram | `INSTAGRAM` | arqrustui-icons-data7 | Substituído | `5d602148b75565a582697c248086633d9343422b884ef8f18c764a01acc1cc0c` | `c19794c705a59a3033437ed68624945bf0310a21ddfcda0e4b6d7fb697617091` | `23f0422ae102d4700b9e979fd72d16505b4be2b2276ed3a4c1c611a1f09fd32e` |
| kakaotalk | `KAKAOTALK` | arqrustui-icons-data8 | Novo | `9d94b31846fce1bac7e0a9bcb2ac1b8766a76f4aea87c8c56669a652f07247a5` | `5963402d67247f16273df2b6e672f059ded8bb310492e76ce93f039c1bc34e34` | `ef0c414cad181f1eef56f00d76493efc32cbc252dc39b80eaef071ff28149178` |
| kick | `KICK` | arqrustui-icons-data8 | Novo | `8390df46397711166abfa68518fe120a3a07a94cde38140cf2906768824c253f` | `247bb0635ef75a5489bf517347094b13533ee23515334650b6d3b39f60ff28b6` | `80efdcf51438731a350fb50fcf145bd22f7a99003253ef40b79d0fe81769307c` |
| kuaishou_kwai | `KUAISHOU_KWAI` | arqrustui-icons-data8 | Novo | `d66e6f7fc69e208e20c9a0ce5c16c6e678bfc108bdbde6798eb916223981c1bc` | `f185aa9f158c3bad354be09ef8f89dde7cfb95b887ac24f660449f424171d4aa` | `439a29dda457527f3c0ae7f63ff1e3eb4fe21976c6b18a07f64459565c91bab1` |
| lemon8 | `LEMON8` | arqrustui-icons-data8 | Novo | `7cd5d62afb4e54ab6e7083f010d2a69b136586bfee545780e914579282a8f4df` | `a54f0d56bf461b5451ceb57363916be8941eaf6c1411f5d59a9e66615d3c350a` | `4ba720a50d7a8adb49081d3b5f1d2402ba503fbcc6b59eefaf8603df2d0ef0c6` |
| likee | `LIKEE` | arqrustui-icons-data8 | Novo | `169536177570b8ced04ed6e4a67c218fd906133b160f89283f4e34056c3ff7e3` | `ad8bf28fa06a216fb381bc16a46530c1ddfce8cc0072ac327f4aa44b782fb539` | `a396e73766b65e9fef3b7a98634b9c8f0d7f41c03fb54c3bf10c98fa90132756` |
| line | `LINE` | arqrustui-icons-data7 | Substituído | `075f91a3e7936d485a4b2b5527abf241e5e37df2d4d983ae4e0157ef2be61c47` | `def27f7346a245fd2d4a18cf59fdec52ad3b6bbf9949f6412b46c5bd83f4b6da` | `3e14fa6127c62bbc6780e3ad224b6bbbf8a470c539c0878d973a0c10a1e93916` |
| linkedin | `LINKEDIN` | arqrustui-icons-data7 | Substituído | `55da258a9e05016d13ef6a915c4fdbe54bf5da92d68abf433cb72ec0b7288efe` | `aa2e85d29baa385401498c6b2ef370113275bbf2388a40fe2c0657538a29a015` | `883d8c97ea106b7d0c93e9318c51f2d0320e640ecf81bc72e45808310dcaa1c7` |
| mastodon | `MASTODON` | arqrustui-icons-data7 | Substituído | `0a458eb56f41fdc280bd5ea417aa60e24dc9a4550333b00cf053a031cac2fba3` | `d309fa9b55bf11f2326ef66242e2479982c3f113d3fb3d28795c0dc9ada545d6` | `5b02229d9e3fc104f3ce2bc44c6bdda73e430dc06d00157b836717ed08fd34ec` |
| facebook_messenger | `MESSENGER` | arqrustui-icons-data8 | Substituído | `6265c978faf40b8adc1a2909561ce6fd49e8c73d4d1176677a05e0d18f30b23e` | `fb5da86d644f897a570593bc90ce9d238e25c8220bce37e85f2c7a329cc396f5` | `02920477a74e61bf467bbd20aa26aa0b7a308231c77b0d48e957169ad5fe1357` |
| nextdoor | `NEXTDOOR` | arqrustui-icons-data8 | Novo | `9bdb961526840715dae17a5312717bd5d8f8c449c3ee045ba481246c84675a8c` | `df208dc2d5f4f2d6578ffa76ea2dbe362554fd73f0efc8ec1b8576964306bf08` | `561bc7a32579ce6e70f2d82530d549db065a8d438044b3e242312f01a1f9d5cd` |
| odnoklassniki | `ODNOKLASSNIKI` | arqrustui-icons-data8 | Novo | `13d93a07322ca8d3130ac722a632acbcaeeb336e4a4cf1e75566841ab413eb45` | `adfc2301b79d956c8e95fc55d75f9b6fc25df3feea43ff8cd35a742e058f86cd` | `6ff7c090e777a0a58efe62ae13229ab8e1c36852ae32892bdee0e460922922c1` |
| pinterest | `PINTEREST` | arqrustui-icons-data8 | Substituído | `e2db8af8f793d7f505ee491936c0c87440411de24eef1c2481430ed887ed7a04` | `9108f7ac26bca59159becb5bda6347507ea192cfd6180a4ff03b7369f45edecc` | `44efc2487554f6bf5ace68469a11036848f43df11e230828e4f94083bd57ba19` |
| qq | `QQ` | arqrustui-icons-data8 | Novo | `f67cbb345adc9fcffc76ff3a27afcb4c29fe39acbae4f95025f05ecd5e1b3930` | `ec3098caefd4ebdabfbbe343674b60c46823a1006dee4b5735776d30b69b65e6` | `9de62ba2da1890f3133ca7c28027f1ae9b641f747447e082de49985a80c61b11` |
| quora | `QUORA` | arqrustui-icons-data8 | Novo | `722fbea18ac51a66644851da0250f3a6299723e6d2882ba20fed80b854175003` | `a265c74dbff22f07c75f432e14a40e7dff4b195fa988599df918d324e7bbf3aa` | `401b25fba52fc3cf531c0d0f4f780b884b1eea45dbdf5ae8977c2615b2912cdc` |
| reddit | `REDDIT` | arqrustui-icons-data8 | Substituído | `32668071e284828109c4cbfade7b6e3a04f3ce2f7685f057f450eb5858aee380` | `5315a72c2411dd5b6edd08f5954aa4741b8f1d6d27ac53c89690c6fab7b0f533` | `97b8bcb78091fbd1ab79f984fd7c4ac1d00576aa1067842354bf167634344c12` |
| rumble | `RUMBLE` | arqrustui-icons-data8 | Novo | `78e1d8efba150228380010eedf6575357eaf5a1dfe33bcaa3114b15d7c525f1d` | `c2c3ccf0065384c2ea5dabc5adc9909e950959503b39d18db00ec19e3b106b1b` | `874114ff153346a2ece86e01ed2c6d34364be5baac4b4781052c15b470d717b9` |
| signal | `SIGNAL` | arqrustui-icons-data8 | Substituído | `09b1d80a835297fdeca4d98cc652b0a7675689e01bc30c44569417354f98560b` | `5ec0a94f5263774a8908c6f374b840c096d3c6df6514acc59fd109a62bf26070` | `b220c64e32ddc9ec99633d905e8fa4845055c7bf0d5650c6885da1b1a29799d9` |
| snapchat | `SNAPCHAT` | arqrustui-icons-data8 | Substituído | `0b6ddc7b986792996838016475c0e5e3e1b044edfe879fd41d31a05ca3d56783` | `bbf7d2de1fa8bda3c9fec5913b38e50b8c3a734b10d8e05dc8f32bdef1d7f64b` | `2f3922c51e74dc9db61c9dffd314a73ed88522eba62fdbbb254020f560102d3e` |
| telegram | `TELEGRAM` | arqrustui-icons-data8 | Substituído | `9519b7482387c62ab1b2cfef383f723faafbc0412222c91f2dba0632fe0cef00` | `244b7bb54af954bd564993b7775c788920609633b55bf575e6b9a188962730bd` | `6fe3c480835b67bafb082e81f3eb316f7e9b027d90149c34b314152536461a39` |
| threads | `THREADS` | arqrustui-icons-data6 | Substituído | `6c4f6af4ff4aa7f76746bf2ca216157dfb7946d6b53d73aae7bdc87a59d8748a` | `2407ec75ccc8b5589bbb36901d54ae24b4397da671a227965ec4f782c3bdc143` | `cd758193bdf0cfafe4330de57595a2cd72e0f1cd5d63149b64dac1d9c4eddf99` |
| tiktok | `TIKTOK` | arqrustui-icons-data8 | Substituído | `7e5a8cc4226428d0ff2b8b61b9c8f9606bda41f2567a60f760448665175526e5` | `90ad715a326e746eb7869e222ec9cd9aacd16f88ef1a3f07f08561f7f93d2ec8` | `91b382ac0cd89a51fd2958f60306441f7364afb8d9b25d08dc85e48464ab44fe` |
| tumblr | `TUMBLR` | arqrustui-icons-data8 | Novo | `f6d2adc2e192e76ccb503260b80db01932e742a46128e3371a52ec10e9774c89` | `30cf0be3affae4dd8d6bde6e3f0746e3ed3cd0fa780011e191a0f03a34f27bb1` | `4dc5d195e74e3c795741e30e45e83016a51f53c957a01911fb5629f082200f29` |
| twitch | `TWITCH` | arqrustui-icons-data8 | Substituído | `8e4f0a1d70f84491c6a99fb7af1a76164e85a12123bd75eaa18427dd7bc95930` | `d29239650b4d8f794daf7c836757718369a9623135ea90c026b147d7e39bc921` | `d4be4e68b409885dd016a241ec22346028e5575f0efc17f37e4a17f3d059d726` |
| viber | `VIBER` | arqrustui-icons-data8 | Substituído | `d1924221cf80c85834c3d10c286f20df89dbe21401fba819d3e2f26eb161f89d` | `737f0920000357c95b86a58273626b858f184d4afca1d55a34fe11e3f8281c29` | `d21b9b4afb7031aeaa36c00378f7fd32a41a66bb2789d062a5d84237da9f2702` |
| vk_vkontakte | `VK_VKONTAKTE` | arqrustui-icons-data8 | Novo | `c1ccf876ce254212c3aa3ce7f43286a1a379039ebc7bb13d5da4869870ebf959` | `77d9a1472f8de37f1b055d315f4b9456e9632120081039ab1cee66c967559342` | `5a3e28e5a9c67e39af8a3f7860f6373bd6a34bf1403b81cba22ad5e262d4354b` |
| wechat | `WECHAT` | arqrustui-icons-data8 | Substituído | `a5e1131ebf92bc26f09e05dec5673d2b5bb7816f2d2878930a028d83d735268a` | `1d7faffd2e722cc43dadb9ecab027e7ba36fd6bcce3d71ddb80df15c2ddcf8d4` | `667cf6c8dc38216fbd296c322df7efc04592824ff750be181216e7056b99ff7c` |
| weibo | `WEIBO` | arqrustui-icons-data8 | Novo | `de036f370f42ef9951ebbdcc55b6f57a24d940c1f2ba9ad0e0d3e9336d749412` | `ddd79d8b48d9ea350add7e424bf1d2d9b8c0d7fe85f0348196bfa63977a01fff` | `41fde1293e47737c74897196139d4f7e91361106f68ea83a87e8d4ff803f35f4` |
| whatsapp | `WHATSAPP` | arqrustui-icons-data8 | Substituído | `b8356ac119ec2acb01299306781a0b23f563594e6b12fa442b70fef76cc581aa` | `03cd228242391bcc4cdd2706ad1d6ebeed6fa6c05589ea75e88805d9f71fc47f` | `f1aebee3e98a3ab8a643ac6c76fbf14d0a521f81cbca1369aad4deb4fd32d4a0` |
| x_twitter | `X_TWITTER` | arqrustui-icons-data8 | Substituído | `e6a98b2eca0441a8a7086af7b42c8c7d2813583e1108ac3041af080d263e29c3` | `c1500db0e06d79b10ef280b79cd7e2a5635a75569f3d01906baa922686789743` | `7f3df2648ddeb0eea2303aa9bd8010c5e16461ca746b9c8c4c68d553dd4309cc` |
| xiaohongshu_rednote | `XIAOHONGSHU_REDNOTE` | arqrustui-icons-data8 | Novo | `85dd607abf6c1af99425f03444a0df1b0a7f5ce33bd8531208f2df8d837bb208` | `3b14826f17db1fed778910c23ae1664e5143184e059ac9fb9f2add69a4715173` | `2820368e86e85edef2d59b2b7478f479d1c627e6d672173858e9b6c1b556c5b9` |
| youtube | `YOUTUBE` | arqrustui-icons-data8 | Substituído | `411be8a00e25b9188c469bea03df1045dae53ee3cda10b1ee1cf0875577cea7b` | `84e1e345aafa428715acd9d3a8cf1ede9cd3b9e26ebc7bc56493f09acb9a5226` | `24116fa90e11a0c84dac25a03f1d0c07054dc03e636c98abc93c987187f01cae` |
| zhihu | `ZHIHU` | arqrustui-icons-data8 | Novo | `9e57de1a213f36356dd1d51ddd7334e3eae3646a3061840124b772508ef0eb1c` | `f9118d4093180a813f0e8ee63e096dad9f1c593616318f2486ed618994d4f376` | `79aee5c972e42a6e6720d5c29a490d29c70d835b0d04e2161f4a3e892f68a228` |
