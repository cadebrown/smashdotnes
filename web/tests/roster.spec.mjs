import { expect, test } from '@playwright/test';
import path from 'node:path';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));

for (const [scope, route] of [['workshop','/design/?theme=b&study=fighters'], ['home','/']]) {
test(`${scope} roster balances actual sprite pixels and preserves controllable motion`, async ({ page }) => {
  const prefix = scope === 'home' ? 'home-' : '';
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await mkdir(screens, { recursive:true });
  await page.goto(route);
  if(scope==='workshop') await expect(page.getByRole('tab', { name:'Fighter roster' })).toHaveAttribute('aria-selected', 'true');
  else await expect(page.locator('#game')).toHaveCount(0);
  const board = page.locator(scope === 'home' ? '[data-home-arena]' : '[data-rental-roster]');
  if (scope === 'home') await board.evaluate(async element => {
    await Promise.all(element.getAnimations({subtree:true}).map(animation => animation.finished));
  });
  const measurements = [];
  for (const width of [1440, 900, 600, 390, 320]) {
    await page.setViewportSize({ width, height:1000 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `page at ${width}px`).toBeLessThanOrEqual(1);
    const fighters = await board.locator('[data-roster-fighter]').evaluateAll(async cards => Promise.all(cards.map(async card => {
      const pose = card.querySelector('.roster-pose');
      const stage = card.querySelector('.sprite-stage');
      const css = getComputedStyle(pose);
      const image = new Image(); image.src = css.backgroundImage.slice(5,-2); await image.decode();
      const frame = Number(css.getPropertyValue('--pose'));
      const canvas = document.createElement('canvas'); canvas.width=32; canvas.height=40;
      const context = canvas.getContext('2d'); context.drawImage(image,frame*32,0,32,40,0,0,32,40);
      const pixels = context.getImageData(0,0,32,40).data;
      let left=32, top=40, right=0, bottom=0;
      for(let y=0;y<40;y++) for(let x=0;x<32;x++) if(pixels[(y*32+x)*4+3]) {left=Math.min(left,x);top=Math.min(top,y);right=Math.max(right,x+1);bottom=Math.max(bottom,y+1);}
      const scale = parseFloat(css.backgroundSize) / image.naturalWidth;
      const frameRect=pose.getBoundingClientRect(), stageRect=stage.getBoundingClientRect();
      const inkLeft=frameRect.left+parseFloat(css.backgroundPositionX)+(frame*32+left)*scale;
      const inkTop=frameRect.top+parseFloat(css.backgroundPositionY)+top*scale;
      const inkWidth=(right-left)*scale, inkHeight=(bottom-top)*scale;
      return {id:card.dataset.rosterFighter,scale,inkWidth,inkHeight,centerError:Math.abs(inkLeft+inkWidth/2-(stageRect.left+stageRect.width/2)),footError:Math.abs(inkTop+inkHeight-(stageRect.bottom-parseFloat(getComputedStyle(stage).paddingBottom))),clipped:inkLeft<frameRect.left-.5||inkTop<frameRect.top-.5||inkLeft+inkWidth>frameRect.right+.5||inkTop+inkHeight>frameRect.bottom+.5,cardFits:inkWidth<=card.getBoundingClientRect().width,feet:inkTop+inkHeight};
    })));
    measurements.push({viewport:width,fighters});
    for (const fighter of fighters) {
      expect(Number.isInteger(fighter.scale), `${fighter.id} whole-pixel scale at ${width}`).toBe(true);
      expect(fighter.centerError, `${fighter.id} centered painted pixels at ${width}`).toBeLessThanOrEqual(.6);
      expect(fighter.footError, `${fighter.id} feet on baseline at ${width}`).toBeLessThanOrEqual(.6);
      expect(fighter.clipped, `${fighter.id} artwork preserved at ${width}`).toBe(false);
      expect(fighter.cardFits, `${fighter.id} fits its column at ${width}`).toBe(true);
    }
    expect(Math.max(...fighters.map(f=>f.inkHeight))/Math.min(...fighters.map(f=>f.inkHeight))).toBeLessThan(1.25);
    if(width>600) expect(Math.max(...fighters.map(f=>f.feet))-Math.min(...fighters.map(f=>f.feet))).toBeLessThanOrEqual(.6);
    if([1440,390].includes(width)) {
      await board.screenshot({path:path.join(screens,`${prefix}rental-screen-${width}.png`)});
    }
  }
  await writeFile(new URL(`../../out/reports/web/${prefix}roster-layout.json`,import.meta.url),JSON.stringify(measurements,null,2));
  await page.setViewportSize({width:1440,height:1000});
  const toggle=(scope==='home' ? page.locator('[data-site-footer]') : board).getByRole('button',{name:'Screen effects'});
  await toggle.click();
  await expect(toggle).toHaveAttribute('aria-pressed','false');
  if (scope === 'workshop') await expect(board.locator('.screen-effects')).toBeHidden();
  else {
    await expect(board.locator('.arena-motion').first()).toBeHidden();
    expect(await board.locator('.roster-pose').first().evaluate(el => getComputedStyle(el).animationName)).toBe('none');
  }
  await page.reload();
  await expect(toggle).toHaveAttribute('aria-pressed','false');
  if(scope==='workshop') await expect(page.getByRole('tab',{name:'Fighter roster'})).toHaveAttribute('aria-selected','true');
  await toggle.press('Space');
  await expect(toggle).toHaveAttribute('aria-pressed','true');
  if (scope === 'workshop') {
    const screen=board.locator('.roster-screen');
    await screen.hover({position:{x:60,y:160}});
    await expect.poll(()=>screen.evaluate(el=>el.style.getPropertyValue('--light-x'))).not.toBe('');
    await page.emulateMedia({reducedMotion:'reduce'});
    await expect.poll(()=>board.locator('.screen-grain').evaluate(el=>getComputedStyle(el).animationName)).toBe('none');
    await expect(board.locator('.screen-roll')).toBeHidden();
    await screen.hover({position:{x:200,y:160}});
    expect(await screen.evaluate(el=>el.style.getPropertyValue('--light-x'))).toBe('');
  } else {
    await board.locator('[data-roster-fighter=mario]').focus();
    await expect.poll(() => board.locator('.arena-motion i').first().evaluate(el => getComputedStyle(el).animationName)).toBe('speed-line');
    await page.emulateMedia({reducedMotion:'reduce'});
    const animations = await board.evaluate(element => element.getAnimations({subtree:true}).filter(animation => animation.playState === 'running').length);
    expect(animations).toBe(0);
    expect(await board.locator('.roster-pose').first().evaluate(el => getComputedStyle(el).animationName)).toBe('none');
  }
  await page.screenshot({path:path.join(screens,`${prefix}rental-reduced-motion.png`)});
  await board.getByRole('link',{name:/Mario Fireball/i}).click();
  await expect(page).toHaveURL(/\/book\/#mario--close-then-cover-the-approach/);
  await expect(page.locator('#mario--close-then-cover-the-approach')).toBeVisible();
  expect(errors).toEqual([]);
});
}
