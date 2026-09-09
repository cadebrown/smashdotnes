import { defineRouteMiddleware } from '@astrojs/starlight/route-data';

export const onRequest = defineRouteMiddleware(({ locals, url }) => {
  const route = locals.starlightRoute;
  const section = url.pathname.split('/')[1];
  const label = section === 'book' ? 'Book' : 'Docs';
  const group = route.sidebar.find(entry => entry.type === 'group' && entry.label === label);
  const entries = group?.type === 'group' ? group.entries : [];
  // Keep a reader within the section they opened, including previous/next links.
  route.sidebar = entries;
  route.hasSidebar = section !== 'blog';
  const links = entries.filter(entry => entry.type === 'link');
  const index = links.findIndex(entry => entry.isCurrent);
  route.pagination = { prev: index > 0 ? links[index - 1] : undefined, next: index >= 0 ? links[index + 1] : undefined };
  if (section === 'blog') { route.sidebar = []; route.pagination = { prev: undefined, next: undefined }; route.toc = undefined; }
  if (url.pathname === '/book/') { route.sidebar = []; route.hasSidebar = false; route.pagination = { prev: undefined, next: undefined }; route.toc = undefined; }
});
