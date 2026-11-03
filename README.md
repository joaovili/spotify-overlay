# Spotify Overlay

Painel flutuante para controlar o Spotify no **macOS** e no **Windows**. Fica sempre no
topo, mostra o que esta tocando e troca de faixa sem tirar o foco do app em uso.

- **Rust nativo com egui/OpenGL**: sem Electron, sem WebView, sem conta Premium e sem API web.
- **macOS**: Spotify via AppleScript ou Spotifast via CLI local.
- **Windows**: sessao SMTC do Windows (Spotify desktop ou Spotifast).
- Tres tamanhos: mini (158x38), medio (376x58) e grande (440x148).
- Atalhos globais, bandeja/menu bar, tres posicoes nos cantos, arrastar e `config.json`.
- O mesmo `config.json` das versoes anteriores, no mesmo caminho.

Versao atual: **2.0.0**.

## Baixar

- macOS: `https://github.com/joaovili/spotify-overlay/releases/latest/download/SpotifyOverlay-macOS.zip`
- Windows: `https://github.com/joaovili/spotify-overlay/releases/latest/download/SpotifyOverlay.exe`

O app nao e assinado. No macOS, libere em **Ajustes do Sistema > Privacidade e
Seguranca**. No Windows, use **Mais informacoes > Executar assim mesmo**.

## Estrutura

```
crates/core      config, layout, relogio de reproducao, atalhos e textos (Rust puro, testado)
crates/media     backends dos players: AppleScript/Spotifast (macOS) e SMTC (Windows)
src-app          app nativo eframe/egui: janela, tray, atalhos, autostart, single-instance
assets           icones e Info.plist do bundle macOS
landing          landing page publicada no GitHub Pages
scripts          empacotamento do .app
```

O codigo e o mesmo nas duas plataformas; o binario e compilado por plataforma e o backend
de midia e escolhido em tempo de compilacao. O macOS usa um `NSPanel` nao ativante e o Windows usa
`WS_EX_NOACTIVATE`/`WS_EX_TOOLWINDOW`, entao clicar no painel nao muda o foco.

## Desenvolvimento

Requisitos: Rust stable, macOS 13+ ou Windows 10/11.

```bash
cargo run                        # roda o app
cargo run -- --settings          # abre direto nas configuracoes
cargo run -- --dump-player       # imprime o estado lido do player
cargo run -- --selftest-core     # checagens rapidas sem GUI
cargo test                       # testes do core e do backend de midia
cargo build --release
```

Ferramentas de desenvolvimento de UI (nao usadas em producao):

```bash
cargo run -- --screenshot /tmp/overlay.png
cargo run -- --settings-screenshot /tmp/settings.png
```

## Empacotar

### macOS

```bash
bash scripts/package-macos.sh native      # arm64 desta maquina
bash scripts/package-macos.sh universal   # arm64 + x86_64 (release)
```

O app sai em `dist/Spotify Overlay.app`, com `LSUIElement`, `NSAppleEventsUsageDescription`
e o icone.

### Windows

```bash
cargo build --release
```

O exe portatil fica em `target\release\spotify-overlay.exe`; o icone e a versao sao
embutidos pelo `build.rs`.

## Configuracao

`~/Library/Application Support/SpotifyOverlay/config.json` no macOS e
`%APPDATA%\SpotifyOverlay\config.json` no Windows (ou ao lado do `.exe`, quando a pasta e
gravavel). As chaves sao as mesmas das versoes anteriores:

```json
{
  "size": "medium",
  "position": "top-right",
  "customX": 1500,
  "customY": 24,
  "hotkeySkip": "Ctrl+Alt+Right",
  "hotkeyPrev": "Ctrl+Alt+Left",
  "hotkeyToggle": "Ctrl+Alt+H",
  "alwaysOnTop": true,
  "closeHidesToTray": true,
  "startWithWindows": false,
  "player": "auto"
}
```

- `size`: `mini`, `medium` ou `large`.
- `position`: `top-right`, `top-left`, `bottom-right`, `bottom-left` ou `custom`.
- `player`: `auto`, `spotify` ou `spotifast`. No `auto`, vence quem estiver tocando;
  depois Spotifast, depois Spotify.
- `startWithWindows`: no macOS significa iniciar no login.
- Atalhos vazios (`""`) desligam o atalho.

## Plataformas

### macOS

- `NSAppleScript` roda no proprio processo (sem `osascript` por poll) e o macOS pede
  permissao de **Automacao** na primeira leitura.
- Com `player=spotifast`, usa `Spotifast now-playing --raw` e os verbos `next`,
  `previous` e `play-pause`.
- A capa vem de `i.scdn.co`; o download/decodificacao roda em segundo plano.
- `NSPanel` nao ativante, nivel flutuante e presenca em todos os espacos.
- Icone na barra de menus; o app nao aparece no Dock.

### Windows

- Usa `GlobalSystemMediaTransportControlsSessionManager` (SMTC). Sem .NET, PowerShell,
  login ou subprocessos.
- `WS_EX_NOACTIVATE` + `WS_EX_TOOLWINDOW`: nao rouba foco e nao entra no Alt+Tab.
- Desenha direto no OpenGL do sistema (egui/glow), sem WebView.

## Desempenho (macOS, release arm64)

- Binario: ~4 MB; `.app`: ~4 MB; zip universal: ~4 MB.
- RSS em idle com o painel aberto: ~90 MB.
- A versao Tauri/WebView media ~135 MB; a Swift/AppKit antiga, ~25 MB.

O objetivo foi sair do WebView mantendo uma UI nativa em Rust. Se o requisito for o menor
RSS possivel, o proximo passo seria desenhar com CoreGraphics em vez de egui/OpenGL.

## Release

Tags `v*` disparam o workflow que compila macOS universal e Windows, publica
`SpotifyOverlay-macOS.zip`, `SpotifyOverlay.exe`, `SpotifyOverlay.zip` e os arquivos
`.sha256`. A landing e publicada pelo GitHub Pages a partir de `landing/`.

## Repositorios antigos

As versoes Swift/AppKit, C#/WinForms e a landing antiga foram arquivadas em
`joaovili/spotify-overlay-macos`, `joaovili/spotify-overlay-windows` e
`joaovili/spotify-overlay-landing`.

## Licenca

MIT. Nao afiliado ao Spotify.
