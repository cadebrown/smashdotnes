import { expect, test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const screens=fileURLToPath(new URL('../../out/reports/web/screens/',import.meta.url));

test('chosen cartridge presentation covers primary routes and keeps footer effects preferences', async ({page})=>{
  const errors=[];
  page.on('pageerror',error=>errors.push(error.message));
  page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  await mkdir(screens,{recursive:true});
  const routes=[['home','/'],['play','/play/'],['book','/book/'],['docs','/docs/'],['journal','/blog/'],['article','/blog/one-rom-many-observers/'],['build','/docs/build/'],['missing','/404.html']];
  for(const [name,url] of routes){
    await page.goto(url);
    await expect(page.locator('html')).toHaveAttribute('data-art-theme','b');
    await expect(page.getByRole('heading',{level:1})).toBeVisible();
    const primary=page.getByRole('navigation',{name:'Primary navigation'});
    await expect(primary.getByRole('link')).toHaveText(['Play','Book','Docs','Blog']);
    await expect(primary.getByRole('button',{name:'Screen effects'})).toHaveCount(0);
    await expect(page.locator('[data-site-footer]')).toHaveCount(1);
    await expect(page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'})).toHaveCount(1);
    for(const width of [1440,1280,1201,1100,900,799,390,320]){
      await page.setViewportSize({width,height:1000});
      expect(await page.evaluate(()=>document.documentElement.scrollWidth-innerWidth),`${name} at ${width}px`).toBeLessThanOrEqual(1);
      const header=page.locator('header.header');
      if(await header.count()) {
        const escapedControls=await header.evaluate(element=>{
          const frame=element.getBoundingClientRect();
          return [...element.querySelectorAll('a,button,select')].filter(control=>{
            const bounds=control.getBoundingClientRect();
            return bounds.width>0 && bounds.height>0 && (bounds.top<frame.top-1 || bounds.bottom>frame.bottom+1 || bounds.left<frame.left-1 || bounds.right>frame.right+1);
          }).map(control=>control.getAttribute('aria-label')||control.textContent.trim());
        });
        expect(escapedControls,`${name} header controls at ${width}px`).toEqual([]);
      }
      if((name==='home' && [1440,390].includes(width)) || (name==='book' && width===390)) await page.screenshot({path:path.join(screens,`cartridge-${name}-${width}.png`)});
    }
  }
  await page.goto('/');
  const toggle=page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'});
  await toggle.click();
  await expect(page.locator('html')).toHaveAttribute('data-screen-fx','off');
  await page.goto('/book/');
  await expect(page.locator('.sidebar-pane')).toHaveCount(0);
  const contents=page.locator('aside.book-nav details[data-book-contents]');
  await page.setViewportSize({width:1440,height:1000});
  await expect(contents).toHaveAttribute('open','');
  await page.setViewportSize({width:390,height:1000});
  await expect(contents).not.toHaveAttribute('open','');
  await contents.locator('summary').click();
  await contents.locator('[data-chapter-link="first-match"]').click();
  await expect(page).toHaveURL(/\/book\/#first-match$/);
  await expect(contents).not.toHaveAttribute('open','');
  await page.setViewportSize({width:1440,height:1000});
  await expect(page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'})).toHaveAttribute('aria-pressed','false');
  await page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'}).click();
  await page.goto('/blog/');
  await expect(page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'})).toHaveAttribute('aria-pressed','true');
  expect(errors).toEqual([]);
});
