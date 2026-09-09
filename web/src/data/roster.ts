// Bounds describe painted pixels inside the original 32 × 40 source frames.
// Crop the presentation window, never stretch or resample the source artwork.
export const roster = [
  { id:'mario', name:'Mario', move:'Fireball', pose:8, frames:14, x:0, y:13, width:21, height:27, scale:6, smallScale:4, ink:'#e47859', anchor:'mario--close-then-cover-the-approach' },
  { id:'simon', name:'Simon', move:'Whip & axe', pose:8, frames:14, x:0, y:10, width:16, height:29, scale:6, smallScale:4, ink:'#d6ac58', anchor:'simon--make-the-extra-reach-count' },
  { id:'tommy', name:'Tommy', move:'Flying kick', pose:8, frames:14, x:5, y:0, width:24, height:38, scale:5, smallScale:3, ink:'#b3cbb9', anchor:'tommy--commit-to-the-kick' },
  { id:'rizer', name:'Rizer', move:'Rifle shot', pose:11, frames:15, x:0, y:8, width:25, height:32, scale:6, smallScale:4, ink:'#d5815c', anchor:'rizer--keep-the-shot-moving' },
] as const;
