import scenes from './manual-scenes.json';

export type ManualScene = keyof typeof scenes;
export type ManualAnnotation = {
  label: string;
  note: string;
  /** Arrow destination in the untouched 256 × 240 source frame. */
  x: number;
  y: number;
  badgeX: number;
  badgeY: number;
  /** Short handwriting on the image; the full instruction stays beside it. */
  tag?: string;
  tagX?: number;
  tagY?: number;
};
export type ManualAnnotationScene = {
  image: string;
  alt: string;
  title: string;
  caption: string;
  annotations: ManualAnnotation[];
};

// Shared by the Book and the capture refresh/review script.
export const manualAnnotations: Record<ManualScene, ManualAnnotationScene> = scenes;
