import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';

export const FRAME_WIDTH = 256;
export const FRAME_HEIGHT = 240;

export const sha256 = value => createHash('sha256').update(value).digest('hex');
export const fileHash = async file => sha256(await readFile(file));

export function pngDimensions(bytes, name = 'PNG') {
  if (bytes.length < 24 || !bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
    throw new Error(`${name} is not a PNG`);
  }
  if (bytes.subarray(12, 16).toString('ascii') !== 'IHDR') throw new Error(`${name} has no IHDR chunk`);
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

export async function inspectPng(file) {
  const bytes = await readFile(file);
  const dimensions = pngDimensions(bytes, path.basename(file));
  if (dimensions.width !== FRAME_WIDTH || dimensions.height !== FRAME_HEIGHT) {
    throw new Error(`${path.basename(file)} is ${dimensions.width}x${dimensions.height}; expected ${FRAME_WIDTH}x${FRAME_HEIGHT}`);
  }
  return { file: path.basename(file), sha256: sha256(bytes), ...dimensions };
}

export async function inspectPngDirectory(directory) {
  const names = (await readdir(directory)).filter(name => name.endsWith('.png')).sort();
  if (!names.length) throw new Error(`No PNG captures found in ${directory}`);
  return Promise.all(names.map(name => inspectPng(path.join(directory, name))));
}

export function validateScenes(scenes, images) {
  if (!scenes || typeof scenes !== 'object' || Array.isArray(scenes)) throw new Error('Manual annotations must be an object keyed by scene.');
  const available = new Set(images.map(image => image.file));
  for (const [scene, value] of Object.entries(scenes)) {
    if (!value || typeof value !== 'object' || typeof value.image !== 'string') throw new Error(`Scene ${scene} has no image.`);
    if (!available.has(`${value.image}.png`)) throw new Error(`Scene ${scene} refers to missing capture ${value.image}.png.`);
    const labels = new Set();
    if (!Array.isArray(value.annotations)) throw new Error(`Scene ${scene} has no annotations array.`);
    for (const annotation of value.annotations) {
      if (!annotation || typeof annotation.label !== 'string' || !annotation.label) throw new Error(`Scene ${scene} has an annotation without a label.`);
      if (labels.has(annotation.label)) throw new Error(`Scene ${scene} repeats annotation label ${annotation.label}.`);
      labels.add(annotation.label);
      for (const key of ['x', 'badgeX']) {
        if (!Number.isFinite(annotation[key])) throw new Error(`Scene ${scene} annotation ${annotation.label} has no numeric ${key}.`);
      }
      for (const key of ['y', 'badgeY']) {
        if (!Number.isFinite(annotation[key])) throw new Error(`Scene ${scene} annotation ${annotation.label} has no numeric ${key}.`);
      }
      if (annotation.tag !== undefined && (!Number.isFinite(annotation.tagX) || !Number.isFinite(annotation.tagY))) {
        throw new Error(`Scene ${scene} annotation ${annotation.label} has a tag without tag coordinates.`);
      }
      for (const key of ['x', 'badgeX', 'tagX']) {
        if (annotation[key] !== undefined && (!Number.isFinite(annotation[key]) || annotation[key] < 0 || annotation[key] > FRAME_WIDTH)) {
          throw new Error(`Scene ${scene} annotation ${annotation.label} has ${key} outside 0..${FRAME_WIDTH}.`);
        }
      }
      for (const key of ['y', 'badgeY', 'tagY']) {
        if (annotation[key] !== undefined && (!Number.isFinite(annotation[key]) || annotation[key] < 0 || annotation[key] > FRAME_HEIGHT)) {
          throw new Error(`Scene ${scene} annotation ${annotation.label} has ${key} outside 0..${FRAME_HEIGHT}.`);
        }
      }
    }
  }
}

export function compareImages(expected, actual, label = 'captures') {
  const current = new Map(actual.map(image => [image.file, image]));
  const stale = [];
  for (const image of expected ?? []) {
    const found = current.get(image.file);
    if (!found || found.sha256 !== image.sha256 || found.width !== image.width || found.height !== image.height) stale.push(image.file);
  }
  if (stale.length || current.size !== (expected ?? []).length) {
    throw new Error(`${label} changed: ${[...new Set([...stale, ...[...current.keys()].filter(name => !(expected ?? []).some(image => image.file === name))])].join(', ') || 'image set differs'}`);
  }
}
