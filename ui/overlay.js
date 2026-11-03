(() => {
  const tauri = window.__TAURI__ || {};
  const invoke = tauri.core ? tauri.core.invoke : async () => undefined;
  const listen = tauri.event ? tauri.event.listen : async () => () => {};
  const currentWindow = () => (tauri.window ? tauri.window.getCurrentWindow() : null);

  const panel = document.getElementById("panel");
  const title = document.getElementById("title");
  const artist = document.getElementById("artist");
  const album = document.getElementById("album");
  const cover = document.getElementById("cover");
  const fill = document.getElementById("fill");
  const time = document.getElementById("time");

  let playing = false;
  let duration = 0;
  let basePosition = 0;
  let baseStamp = performance.now();
  let lastArtworkKey = "";
  let lastTitle = "";
  let lastArtist = "";

  const permissionText = "Permita o controle do Spotify em Ajustes > Privacidade e Seguranca > Automacao";

  function mmss(seconds) {
    const total = Math.max(0, Math.floor(seconds || 0));
    return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
  }

  function currentPosition() {
    if (!playing) return basePosition;
    return Math.min(duration, basePosition + (performance.now() - baseStamp) / 1000);
  }

  function paintProgress() {
    if (!duration || duration <= 0) {
      fill.style.width = "0%";
      time.textContent = "";
      return;
    }
    const position = currentPosition();
    const fraction = Math.min(1, Math.max(0, position / duration));
    fill.style.width = `${(fraction * 100).toFixed(2)}%`;
    time.textContent = `${mmss(position)} / ${mmss(duration)}`;
  }

  function applyConfig(config) {
    if (!config) return;
    panel.dataset.size = config.size || "medium";
  }

  function updateState(snapshot) {
    if (!snapshot) return;

    const sameTrack = snapshot.title === lastTitle && snapshot.artist === lastArtist;
    lastTitle = snapshot.title || "";
    lastArtist = snapshot.artist || "";

    playing = Boolean(snapshot.playing);
    duration = Number(snapshot.duration) || 0;
    const incoming = Number(snapshot.position) || 0;
    // O clock do Rust ja e monotonico; ainda assim evita um salto para tras
    // quando a interface recebe duas leituras muito proximas.
    basePosition = sameTrack ? Math.max(incoming, currentPosition() - 0.35) : incoming;
    baseStamp = performance.now();
    panel.classList.toggle("playing", playing);

    title.textContent = snapshot.title || "Spotify";
    if (snapshot.isPermissionDenied) {
      artist.textContent = permissionText;
      artist.classList.add("permission");
    } else {
      artist.textContent = snapshot.artist || "";
      artist.classList.remove("permission");
    }
    album.textContent = snapshot.album || "";

    if (snapshot.artworkKey) {
      if (snapshot.artwork) {
        cover.src = snapshot.artwork;
        cover.classList.add("visible");
        lastArtworkKey = snapshot.artworkKey;
      } else if (snapshot.artworkKey !== lastArtworkKey && cover.src) {
        // Nao chegou imagem nova para esta chave: melhor esconder do que mostrar a antiga.
        cover.classList.remove("visible");
        cover.removeAttribute("src");
        lastArtworkKey = snapshot.artworkKey;
      }
    } else {
      cover.classList.remove("visible");
      cover.removeAttribute("src");
      lastArtworkKey = "";
    }

    paintProgress();
  }

  async function readConfig() {
    try {
      applyConfig(await invoke("get_config"));
    } catch (_) {}
  }

  async function readSnapshot() {
    try {
      const snapshot = await invoke("get_snapshot");
      if (snapshot) updateState(snapshot);
    } catch (_) {}
  }

  function watchDragEnd() {
    const win = currentWindow();
    if (!win) return;
    setTimeout(async () => {
      let last = null;
      let stable = 0;
      const timer = setInterval(async () => {
        try {
          const position = await win.outerPosition();
          if (last && position.x === last.x && position.y === last.y) {
            stable += 1;
          } else {
            stable = 0;
            last = position;
          }
          if (stable >= 5) {
            clearInterval(timer);
            try {
              await invoke("save_position");
            } catch (_) {}
          }
        } catch (_) {
          clearInterval(timer);
        }
      }, 120);
    }, 220);
  }

  panel.addEventListener("mousedown", (event) => {
    if (event.button !== 0) return;
    if (event.target.closest("button")) return;
    event.preventDefault();
    const win = currentWindow();
    if (!win) return;
    try {
      win.startDragging();
      watchDragEnd();
    } catch (_) {}
  });

  panel.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    invoke("popup_menu");
  });

  document.querySelectorAll("[data-control]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      invoke("control", { command: button.dataset.control });
    });
  });

  document.querySelectorAll("[data-action]").forEach((button) => {
    button.addEventListener("click", (event) => {
      event.stopPropagation();
      const action = button.dataset.action;
      if (action === "settings") invoke("open_settings");
      if (action === "hide") invoke("close_overlay");
    });
  });

  listen("state", (event) => updateState(event.payload));
  listen("config", (event) => applyConfig(event.payload));

  setInterval(paintProgress, 250);
  readConfig();
  readSnapshot();
})();
