export type ControlId = 'up' | 'down' | 'left' | 'right' | 'select' | 'start' | 'b' | 'a';

// Measured in the unmodified 3721 × 1870 Nintendo controller photograph.
export const controllerControls = [
  { id: 'up', key: 'W', nes: 'D-pad Up', action: 'Jump', x: 705, y: 944, w: 180, h: 180 },
  { id: 'down', key: 'S', nes: 'D-pad Down', action: 'Menu down', x: 705, y: 1330, w: 180, h: 180 },
  { id: 'left', key: 'A', nes: 'D-pad Left', action: 'Move left', x: 510, y: 1140, w: 180, h: 180 },
  { id: 'right', key: 'D', nes: 'D-pad Right', action: 'Move right', x: 910, y: 1140, w: 180, h: 180 },
  { id: 'select', key: "'", nes: 'Select', action: 'Go back', x: 1505, y: 1315, w: 270, h: 110 },
  { id: 'start', key: 'Enter', nes: 'Start', action: 'Confirm', x: 1956, y: 1315, w: 270, h: 110 },
  { id: 'b', key: 'K', nes: 'NES B', action: 'Special', x: 2570, y: 1320, w: 295, h: 295 },
  { id: 'a', key: 'J', nes: 'NES A', action: 'Jab', x: 3028, y: 1320, w: 295, h: 295 },
] satisfies { id: ControlId; key: string; nes: string; action: string; x: number; y: number; w: number; h: number }[];
