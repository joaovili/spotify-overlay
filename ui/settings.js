(() => {
  const tauri = window.__TAURI__ || {};
  const invoke = tauri.core ? tauri.core.invoke : async () => undefined;
  const currentWindow = () => (tauri.window ? tauri.window.getCurrentWindow() : null);
  const isMac = /Mac/i.test(navigator.userAgent);

  const $ = (id) => document.getElementById(id);
  const size = $("size");
  const position = $("position");
  const customPosition = $("custom-position");
  const customX = $("customX");
  const customY = $("customY");
  const player = $("player");
  const alwaysOnTop = $("alwaysOnTop");
  const closeHidesToTray = $("closeHidesToTray");
  const startWithWindows = $("startWithWindows");
  const status = $("status");
  const rows = Array.from(document.querySelectorAll(".hotkey"));

  let config = null;
  let recording = null;

  if (isMac) $("startup-label").textContent = "Iniciar no login";
  else $("startup-label").textContent = "Iniciar com o Windows";

  const keyMap = {
    ArrowLeft: "Left",
    ArrowRight: "Right",
    ArrowUp: "Up",
    ArrowDown: "Down",
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Home: "Home",
    End: "End",
    PageUp: "PgUp",
    PageDown: "PgDn",
    Delete: "Delete",
    Backspace: "Backspace",
    MediaTrackNext: "MediaNextTrack",
    MediaTrackPrevious: "MediaPrevTrack",
    MediaPlayPause: "MediaPlayPause",
    Minus: "-",
    Equal: "=",
    BracketLeft: "[",
    BracketRight: "]",
    Backslash: "\\",
    Semicolon: ";",
    Quote: "'",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Backquote: "`",
  };

  function keyName(event) {
    const code = event.code || "";
    if (/^Key[A-Z]$/.test(code)) return code.slice(3);
    if (/^Digit[0-9]$/.test(code)) return code.slice(5);
    if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
    if (keyMap[code]) return keyMap[code];
    if (event.key && event.key.length === 1) {
      const character = event.key.toUpperCase();
      if (/[A-Z0-9]/.test(character)) return character;
    }
    return null;
  }

  function stopRecording(applyValue) {
    if (!recording) return;
    const { row, input, previous } = recording;
    row.classList.remove("capturing");
    if (!applyValue) input.value = previous;
    recording = null;
    invoke("set_hotkey_capture", { capturing: false });
  }

  function startRecording(row) {
    if (recording) stopRecording(false);
    const input = row.querySelector("input");
    recording = { row, input, previous: input.value };
    row.classList.add("capturing");
    input.value = "Pressione...";
    status.textContent = "";
    invoke("set_hotkey_capture", { capturing: true });
  }

  window.addEventListener(
    "keydown",
    (event) => {
      if (!recording) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        stopRecording(false);
        return;
      }

      if (["Control", "Alt", "Shift", "Meta"].includes(event.key)) {
        event.preventDefault();
        return;
      }

      const modifiers = [];
      if (event.ctrlKey) modifiers.push("Ctrl");
      if (event.altKey) modifiers.push("Alt");
      if (event.shiftKey) modifiers.push("Shift");
      if (event.metaKey) modifiers.push(isMac ? "Cmd" : "Win");

      if (modifiers.length === 0) {
        status.textContent = "Use ao menos um modificador (Ctrl, Alt, Shift ou Cmd).";
        event.preventDefault();
        return;
      }

      const key = keyName(event);
      if (!key) return;

      event.preventDefault();
      event.stopPropagation();
      recording.input.value = [...modifiers, key].join("+");
      stopRecording(true);
    },
    true,
  );

  rows.forEach((row) => {
    row.querySelector(".record").addEventListener("click", () => startRecording(row));
    row.querySelector(".clear").addEventListener("click", () => {
      if (recording && recording.row === row) stopRecording(false);
      row.querySelector("input").value = "";
    });
  });

  position.addEventListener("change", () => {
    customPosition.classList.toggle("hidden", position.value !== "custom");
  });

  function fill(next) {
    config = next;
    size.value = next.size || "medium";
    position.value = next.position || "top-right";
    player.value = next.player || "auto";
    customX.value = next.customX ?? 1500;
    customY.value = next.customY ?? 24;
    alwaysOnTop.checked = Boolean(next.alwaysOnTop);
    closeHidesToTray.checked = Boolean(next.closeHidesToTray);
    startWithWindows.checked = Boolean(next.startWithWindows);
    customPosition.classList.toggle("hidden", position.value !== "custom");
    rows.forEach((row) => {
      row.querySelector("input").value = next[row.dataset.key] || "";
    });
  }

  function collect() {
    const next = {
      size: size.value,
      position: position.value,
      customX: Number.parseInt(customX.value, 10) || 0,
      customY: Number.parseInt(customY.value, 10) || 0,
      hotkeySkip: "",
      hotkeyPrev: "",
      hotkeyToggle: "",
      alwaysOnTop: alwaysOnTop.checked,
      closeHidesToTray: closeHidesToTray.checked,
      startWithWindows: startWithWindows.checked,
      player: player.value,
    };
    rows.forEach((row) => {
      next[row.dataset.key] = row.querySelector("input").value.trim();
    });
    return next;
  }

  async function save() {
    if (recording) stopRecording(false);
    status.textContent = "";
    try {
      await invoke("save_config", { config: collect() });
      const win = currentWindow();
      if (win) win.close();
    } catch (error) {
      status.textContent = String(error);
    }
  }

  $("save").addEventListener("click", save);
  $("cancel").addEventListener("click", () => {
    if (recording) stopRecording(false);
    const win = currentWindow();
    if (win) win.close();
  });

  invoke("get_config")
    .then(fill)
    .catch((error) => {
      status.textContent = String(error);
    });
})();
