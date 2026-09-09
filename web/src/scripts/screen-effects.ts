export function initScreenEffects() {
  const root = document.documentElement;
  const storageKey = 'smash-screen-fx';
  const setEffects = (enabled: boolean) => {
    root.dataset.screenFx = enabled ? 'on' : 'off';
    document.querySelectorAll<HTMLElement>('[data-rental-roster]').forEach(board => { board.dataset.screenFx = root.dataset.screenFx; });
    document.querySelectorAll<HTMLButtonElement>('[data-screen-effects-toggle]').forEach(toggle => {
      toggle.setAttribute('aria-pressed', String(enabled));
      const label = toggle.querySelector('[data-screen-fx-label]');
      if (label) label.textContent = enabled ? 'ON' : 'OFF';
    });
    window.dispatchEvent(new CustomEvent('smash:screen-effects', { detail: { enabled } }));
  };
  let enabled = true;
  try { enabled = localStorage.getItem(storageKey) !== 'off'; } catch {}
  setEffects(enabled);
  document.addEventListener('click', event => {
    if (!(event.target instanceof Element) || !event.target.closest('[data-screen-effects-toggle]')) return;
    const next = root.dataset.screenFx !== 'on';
    setEffects(next);
    try { localStorage.setItem(storageKey, next ? 'on' : 'off'); } catch {}
  });
  window.addEventListener('storage', event => {
    if (event.key === storageKey) setEffects(event.newValue !== 'off');
  });
}
