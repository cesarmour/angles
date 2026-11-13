# Angles

Jogo didático de CQB (limpeza de cômodos) em primeira pessoa, para Mac. A ideia central: todo vão de porta é um ângulo de 180° e o jogo mede, ao vivo, quanto dele o jogador já abriu. O jogo é inteiro em inglês; os comentários do código são em português.

- Nome do jogo: **Angles**. "Kill House" é o nome da versão (o ícone continua KILL/HOUSE).
- Repositório: `github.com/cesarmour/angles` (público, conta pessoal).
- Site: https://angles-kz.vercel.app · Painel de uso: https://angles-kz.vercel.app/stats.html
- Versão publicada na data deste arquivo: 0.14.0 (assinada e notarizada pela Apple).

## Como rodar

```bash
cargo build --release                         # compila (binário: target/release/angles)
cargo test --release -- --test-threads=1      # testes sem janela
./bundle.sh                                   # monta e assina dist/Angles.app
```

- `cargo` fica em `~/.cargo/bin`. O Python do sistema pode falhar por licença do Xcode: usar `/opt/homebrew/bin/python3`.
- Testes sem janela cobrem: mapas válidos e rotas (`scenario.rs`), desenho do arco (`tactical.rs`), montagem de todos os sons (`audio.rs`), nomes de codinome (`net.rs`). Com `ANGLES_NET_TEST=1` roda também um teste real contra o servidor (tabela `__test`, que o servidor esconde).
- Abrir o jogo com roteiro (capturas e testes de tela): `./run.sh <roteiro>`, sempre mudo (`CQB_MUTE` já vem ligado). Roteiros em `src/bot.rs`.

## Estrutura

| Onde | O que é |
|---|---|
| `src/main.rs` | Montagem do app, ordem dos sistemas, pasta de recursos, migração de saves antigos |
| `src/state.rs` | Estado da partida (`Game`), jogador, alvos, "cabeça" dos invasores (`Brain`), eventos de som (`Sfx`) |
| `src/map.rs`, `src/maps/*.rs` | Regras de mapa e os seis mapas (um arquivo por mapa; `maps/mod.rs` lista todos) |
| `src/world.rs` | Geometria: paredes de OSB com montantes, portas, luminárias, materiais |
| `src/player.rs` | Movimento, câmeras (primeira pessoa, de cima, minimapa), arma em primeira pessoa |
| `src/combat.rs` | Tiros (inclusive através do OSB), linha de visada, reação e mira dos invasores |
| `src/scenario.rs` | Cenários, inteligência dos invasores (fatiar, recuar, aprender a porta, granada), ranking local |
| `src/tactical.rs` | O sistema de ângulo: cálculo do que foi aberto e o arco no topo da tela |
| `src/lessons.rs` | As seis lições do tutorial, objetivos, ponto de retomada, replay |
| `src/course.rs` | O curso (5 fases, 9 etapas), progresso, a voz do instrutor com legenda, a tela da frase histórica (`QUOTES`) e a contagem 3-2-1 |
| `src/chatter.rs` | Falas dos invasores, da refém e do operador, saindo de onde cada um está |
| `src/audio.rs` | Todos os efeitos (montados na abertura a partir de gravações) e as duas músicas |
| `src/operator.rs` | Bonecos 3D: animações dos invasores e reféns, corpo do jogador, retrato do menu |
| `src/ui.rs` | Menu, telas, HUD |
| `src/net.rs` | Ranking mundial e contagem de uso (envio em segundo plano) |
| `src/bot.rs` | Roteiros automáticos para testes e capturas |
| `site/` | `template.html` + `build.py` geram `index.html` (site) e `artifact.html`; `server/` é o servidor do ranking |
| `tools/`, `tools3d/` | Scripts de preparo de texturas, música, ícone e conversão de modelos no Blender |

## Recursos

- `assets/models/`: `operator`, `player`, `threat` (invasor cáqui), `threat_b` (invasor camuflado), `hostage`, `walker`, armas, e `ph/` (poucos objetos do Poly Haven).
- `assets/voice/`: falas do instrutor (`<id>.mp3`, roteiro em `script.tsv`) e dos personagens (`ch_*.mp3`, roteiro em `tools/voices_characters.tsv`).
- `assets/sfx/`, `assets/music/`: efeitos e as duas faixas (menu e partida).
- `assets/local/`: modelos de uso pessoal. Fica fora do git e nunca entra no pacote publicado; `LOCAL=1 ./bundle.sh` monta uma versão pessoal em `dist/local/`.
- `3dmodels/`: fontes originais dos modelos e animações do Mixamo. Fora do git; é de lá que os scripts de `tools3d/` reconstroem os personagens.
- Licenças e autores: `CREDITS.md`. Todos os modelos em uso são CC BY ou CC0 (a refém é a "Carla" da Renderpeople, CC BY). A refém antiga, de licença não comercial, saiu na 0.14.

## Dados do jogador

`~/Library/Application Support/Angles/`: `course.txt` (progresso), `ranking.tsv` (ranking local), `callsign.txt` (codinome), `install.txt` (identificador sorteado da instalação). Um arquivo vazio chamado `offline` nessa pasta, ou `ANGLES_OFFLINE=1`, desliga o envio ao servidor.

## Servidor (ranking mundial e uso)

- Código em `site/server/`: funções `api/scores`, `api/event`, `api/stats` e a página `stats.html`.
- Roda no projeto `angles-kz` da Vercel, junto com o site. Dados num Blob privado (`boards.json`, `totals.json`, `stats.json` e um arquivo por partida em `raw/`).
- O jogo envia: pontuação, tempo e codinome de cada partida terminada, e avisos anônimos (abriu, começou etapa, terminou etapa). Nenhum dado pessoal.
- Não há proteção contra pontuação forjada além da validação de faixa e de nome.

## Publicar

1. Subir a versão em `bundle.sh` (duas linhas do `Info.plist`), `site/build.py` (`VERSION`), `README.md` e `Cargo.toml`.
2. `cargo build --release`, testes, `./bundle.sh`.
3. `ditto -c -k --keepParent Angles.app Angles-mac.zip` dentro de `dist/`.
4. Notarizar: `xcrun notarytool submit dist/Angles-mac.zip --keychain-profile killhouse-notary --wait`; depois `xcrun stapler staple dist/Angles.app` e refazer o zip.
5. Commit, push e `gh release create vX.Y.Z dist/Angles-mac.zip --repo cesarmour/angles`.
6. Site: acertar `SIZE` em `site/build.py`, rodar `python3 site/build.py`, publicar `index.html` junto com a pasta `site/img/` (e `site/server/` se mudou) no projeto `angles-kz`.
7. Conferir: baixar o zip do release e comparar com o local; `spctl -a -vv` no app extraído; comparar a página no ar com `site/index.html`.

## Capturas para o site

`CQB_NOHUD=1` esconde a interface; `CQB_NOREACT=1` deixa os invasores parados; `CQB_NODEATH=1` impede o fim da partida; `CQB_W`/`CQB_H` definem o tamanho da janela. Roteiros: `site2` (arco, vista de cima, portas), `hero` (invasores de perto, enquadrados à direita). As capturas saem em `shots/` (fora do git).

## Regras do projeto

- Não abrir a janela do jogo sem o dono pedir na hora; compilar, testar sem janela e empacotar pode.
- Trabalho de design: pesquisar referências e mostrar imagem antes de implementar.
- Publicar ou commitar só com o aval dele; nunca dizer que algo está no ar sem conferir o destino.
- Não usar modelos tirados de outros jogos. Modelos sem licença de redistribuição ficam só em `assets/local/`.
- Não trazer de volta: andares e escadas no mapa da casa, pintura no chão, gizmos de linha, medidores flutuantes nas portas, painel de métricas, legenda, radar no minimapa, a refém no site.
- Falas: uma voz de cada vez (`game.talk`) e uma mensagem de cada vez na tela (legenda, depois aviso, depois dica). A frase histórica aparece ao abrir o jogo e ao fim de cada partida; a contagem 3-2-1 só no começo da partida.
- Preferências já decididas: modo tático ligado por padrão; lanterna desligada por padrão; operador com a voz do instrutor, mais grave; site preto, estilo militar, com o sistema de ângulo em destaque.
