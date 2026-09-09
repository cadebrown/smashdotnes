import { getCollection } from 'astro:content';

export async function sectionEntries(section: 'book' | 'docs' | 'blog') {
  const entries = await getCollection('docs', entry => entry.id.startsWith(`${section}/`));
  return entries.sort((a, b) => section === 'blog'
    ? (b.data.date?.valueOf() ?? 0) - (a.data.date?.valueOf() ?? 0) || b.id.localeCompare(a.id)
    : (a.data.sidebar.order ?? 99) - (b.data.sidebar.order ?? 99));
}

export const formatDate = (date: Date) => new Intl.DateTimeFormat('en', {
  month: 'short', day: 'numeric', year: 'numeric', timeZone: 'UTC',
}).format(date);
