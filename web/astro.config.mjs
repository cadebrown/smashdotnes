import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import learningExtensions from './src/extensions/learning.mjs';

export default defineConfig({
  site: 'https://smash.cade.io',
  publicDir: './.public',
  trailingSlash: 'always',
  output: 'static',
  integrations: [starlight({
    title: 'SMASH.NES',
    favicon: '/assets/favicon.svg',
    disable404Route: true,
    description: 'Play the game. Take apart the cartridge.',
    plugins: [learningExtensions()],
    credits: false,
    routeMiddleware: './src/route-middleware.ts',
    sidebar: [
      { label: 'Book', items: [{ autogenerate: { directory: 'book' } }] },
      { label: 'Docs', items: [{ autogenerate: { directory: 'docs' } }] },
    ],
  })],
});
