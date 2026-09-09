export function initRentalRoster(board: HTMLElement) {
  const screen = board.querySelector<HTMLElement>('.roster-screen');
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  if (!screen) return;
  let pendingFrame = 0;
  let x = 50;
  let y = 30;
  const resetLight = () => {
    cancelAnimationFrame(pendingFrame);
    pendingFrame = 0;
    screen.style.removeProperty('--light-x');
    screen.style.removeProperty('--light-y');
  };
  window.addEventListener('smash:screen-effects', () => {
    if (document.documentElement.dataset.screenFx === 'off') resetLight();
  });
  screen.addEventListener('pointermove', (event) => {
    if (event.pointerType !== 'mouse' || reducedMotion.matches || document.documentElement.dataset.screenFx !== 'on' || document.documentElement.dataset.artTheme !== 'b') return;
    const bounds = screen.getBoundingClientRect();
    x = Math.max(0, Math.min(100, (event.clientX - bounds.left) / bounds.width * 100));
    y = Math.max(0, Math.min(100, (event.clientY - bounds.top) / bounds.height * 100));
    if (!pendingFrame) pendingFrame = requestAnimationFrame(() => {
      screen.style.setProperty('--light-x', `${x.toFixed(1)}%`);
      screen.style.setProperty('--light-y', `${y.toFixed(1)}%`);
      pendingFrame = 0;
    });
  });
  screen.addEventListener('pointerleave', resetLight);
  reducedMotion.addEventListener('change', resetLight);
}
