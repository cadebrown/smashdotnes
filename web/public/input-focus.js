// EmulatorJS listens for keyup inside its own parent. Focus can leave that
// element before a held key is released, so retain the actual mapped controls.
window.installKeyboardFocusRelease = function installKeyboardFocusRelease(emulator) {
  const parent = emulator.elements.parent;
  const heldKeys = new Map();

  function releaseKey(keyCode) {
    const controls = heldKeys.get(keyCode) ?? [];
    heldKeys.delete(keyCode);
    for (const [player, button] of controls) {
      emulator.gameManager.simulateInput(player, button, 0);
    }
  }

  function releaseAll() {
    for (const keyCode of heldKeys.keys()) releaseKey(keyCode);
  }

  parent.addEventListener("keydown", (event) => {
    // The core marks accepted keyboard input with preventDefault(). This
    // listener follows its handler, excluding settings and disabled input.
    if (!event.defaultPrevented || heldKeys.has(event.keyCode)) return;
    const controls = [];
    for (let player = 0; player < 4; player += 1) {
      for (let button = 0; button < 30; button += 1) {
        // Save, load, and slot selection are one-shot commands. Speed and
        // rewind controls (27-29) still require a release, just like buttons.
        if (button >= 24 && button <= 26) continue;
        if (emulator.controls[player]?.[button]?.value === event.keyCode) {
          controls.push([player, button]);
        }
      }
    }
    if (controls.length) heldKeys.set(event.keyCode, controls);
  });
  parent.addEventListener("keyup", (event) => releaseKey(event.keyCode));
  parent.addEventListener("focusout", (event) => {
    if (!parent.contains(event.relatedTarget)) releaseAll();
  });
  window.addEventListener("blur", releaseAll);
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) releaseAll();
  });
};
