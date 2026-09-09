export const artThemes = [
  { id: 'a', name: 'Fight Poster', description: 'Current poster baseline.' },
  { id: 'b', name: 'Rental Cartridge', description: 'Scuffed plastic and tile type.' },
  { id: 'c', name: 'Xerox Brawl', description: 'Photocopied fight flyer.' },
  { id: 'd', name: 'After Hours', description: 'Faded arcade marquee.' },
] as const;

export type ArtThemeId = (typeof artThemes)[number]['id'];
