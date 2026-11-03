# Spotify Overlay

Painel flutuante para controlar o Spotify no **macOS** e no **Windows**. Fica sempre no
topo, mostra o que esta tocando e troca de faixa sem tirar o foco do app em uso.

- **macOS**: Spotify via AppleScript ou Spotifast via CLI local.
- **Windows**: sessao SMTC do Windows (Spotify desktop ou Spotifast).
- **Sem login, sem token, sem conta Premium e sem API web.**
- Tres tamanhos: mini (158x38), medio (376x58) e grande (440x148).
- Atalhos globais, bandeja/menu bar, tres posicoes nos cantos, arrastar e config.json.

Versao atual: **1.2.0**.

## Baixar

- macOS: `https://github.com/joaovili/spotify-overlay/releases/latest/download/SpotifyOverlay-macOS.zip`
- Windows: `https://github.com/joaovili/spotify-overlay/releases/latest/download/SpotifyOverlay.exe`

O app nao e assinado. No macOS, libere em **Ajustes do Sistema > Privacidade e
Seguranca**. No Windows, use **Mais informacoes > Executar assim mesmo**.

## Como este repositorio esta organizado

```
crates/core      config, layout, relogio de reproducao, atalhos e textos (Rust puro, testado)
crates/media     backends dos players: AppleScript/Spotifast (macOS) e SMTC (Windows)
src-tauri        shell Tauri: janela, bandeja, atalhos globais, autostart, single-instance
ui               overlay e configuracoes em HTML/CSS/JS puro (sem framework)
landing          landing page publicada no GitHub Pages
scripts          geracao dos icones
```

O mesmo binario Tauri atende as duas plataformas: o backend de midia e escolhido em tempo
de compilacao e a interface e a mesma. O resultado fica pequeno (~3 MB por plataforma)
e usa o WebView do sistema.

## Desenvolvimento

Requisitos: Rust stable, Node 20+, macOS 13+ ou Windows 10/11.

```bash
npm install
npm run dev                      # roda o app em modo dev
npm run build                    # gera o bundle da plataforma atual
cargo test                       # testes do core e do backend de midia
cargo run -- --selftest-core     # checagens rapidas sem GUI
cargo run -- --dump-player       # imprime o estado lido do player
cargo run -- --settings          # abre direto nas configuracoes
```

Para gerar os icones novamente:

```bash
swift scripts/IconGen.swift build/Icon.iconset
npx tauri icon build/Icon.iconset/icon_512x512@2x.png
```

## Configuracao

O arquivo fica em `~/Library/Application Support/SpotifyOverlay/config.json` no macOS e em
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

- Le e controla o Spotify pelo dicionario AppleScript do app. Na primeira vez o macOS pede
  permissao de **Automacao**.
- Com `player=spotifast`, usa `Spotifast now-playing --raw` e os verbos `next`,
  `previous` e `play-pause`.
- A capa vem de `i.scdn.co`; a interface carrega a imagem direto no WebView.
- A janela e um `NSPanel` nao ativante, entao clicar no painel nao muda o app em foco.
- O icone fica na barra de menus; o app nao aparece no Dock.

### Windows

- Usa `GlobalSystemMediaTransportControlsSessionManager` (SMTC). Nao precisa de .NET,
  PowerShell, login ou subprocessos.
- A janela usa `WS_EX_NOACTIVATE` e `WS_EX_TOOLWINDOW`: nao rouba foco e nao entra no
  Alt+Tab.
- O WebView2 ja vem no Windows 10/11.

## Release

Tags `v*` disparam o workflow que compila macOS universal e Windows, publica
`SpotifyOverlay-macOS.zip`, `SpotifyOverlay.exe`, `SpotifyOverlay.zip` e os arquivos
`.sha256`. A landing e publicada pelo GitHub Pages a partir de `landing/`.

## Repositorios antigos

As versoes Swift/AppKit, C#/WinForms e a landing antiga foram arquivadas em
`joaovili/spotify-overlay-macos`, `joaovili/spotify-overlay-windows` e
`joaovili/spotify-overlay-landing`. O codigo deste repositorio e uma reescrita em Rust/Tauri.

## Licenca

MIT. Nao afiliado ao Spotify.
