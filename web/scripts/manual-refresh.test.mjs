import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { compareImages, inspectPng, validateScenes } from './manual-capture-check.mjs';
import { checkApprovedManualAssets, prepareReview } from './refresh-manual.mjs';

const png = (width = 256, height = 240) => { const bytes = Buffer.alloc(24); Buffer.from([137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82]).copy(bytes); bytes.writeUInt32BE(width, 16); bytes.writeUInt32BE(height, 20); return bytes; };
const scene = image => ({ scene: { image, annotations: [{ label: 'A', x: 1, y: 2, badgeX: 3, badgeY: 4 }] } });

test('capture inspection rejects a wrong-sized pixel frame', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'manual-refresh-'));
  const file = path.join(directory, 'frame.png'); await writeFile(file, png(255));
  await assert.rejects(() => inspectPng(file), /255x240/);
});

test('approval comparison rejects tampered pixels', () => {
  const approved = [{ file: '01_title.png', sha256: 'good', width: 256, height: 240 }];
  assert.throws(() => compareImages(approved, [{ ...approved[0], sha256: 'tampered' }]), /changed/);
});

test('freshness check rejects tampered ROM and annotations', async () => {
  const fixture = await mkdtemp(path.join(os.tmpdir(), 'manual-approved-'));
  const paths = ['web/src/data', 'web/public/rom', 'web/public/assets/captures', 'nessy/tools/emulator'];
  await Promise.all(paths.map(directory => mkdir(path.join(fixture, directory), { recursive: true })));
  const annotation = JSON.stringify(scene('01_title'));
  const frame = png();
  const crypto = await import('node:crypto');
  const hash = value => crypto.createHash('sha256').update(value).digest('hex');
  await Promise.all([
    writeFile(path.join(fixture, 'web/src/data/manual-scenes.json'), annotation),
    writeFile(path.join(fixture, 'web/public/rom/smash.nes'), 'rom bytes'),
    writeFile(path.join(fixture, 'web/public/assets/captures/01_title.png'), frame),
    writeFile(path.join(fixture, 'nessy/tools/emulator/manual_capture.lua'), 'driver bytes'),
  ]);
  await writeFile(path.join(fixture, 'web/src/data/manual-capture-lock.json'), JSON.stringify({ version: 1, romSha256: hash('rom bytes'), driverSha256: hash('driver bytes'), annotationSha256: hash(annotation), images: [{ file: '01_title.png', sha256: hash(frame), width: 256, height: 240 }] }));
  await checkApprovedManualAssets({ projectRoot: fixture });
  await writeFile(path.join(fixture, 'web/public/rom/smash.nes'), 'tampered ROM');
  await assert.rejects(() => checkApprovedManualAssets({ projectRoot: fixture }), /Published ROM changed/);
  await writeFile(path.join(fixture, 'web/public/rom/smash.nes'), 'rom bytes');
  await writeFile(path.join(fixture, 'web/src/data/manual-scenes.json'), JSON.stringify(scene('01_title'), null, 2));
  await assert.rejects(() => checkApprovedManualAssets({ projectRoot: fixture }), /Manual annotations changed/);
});

test('scene validation rejects missing captures, changed annotations, and off-frame coordinates', () => {
  const image = [{ file: '01_title.png', sha256: 'x', width: 256, height: 240 }];
  assert.throws(() => validateScenes(scene('missing'), image), /missing capture/);
  const invalid = scene('01_title'); invalid.scene.annotations[0].x = 257;
  assert.throws(() => validateScenes(invalid, image), /outside/);
  const changed = scene('01_title'); changed.scene.annotations.push({ label: 'A', x: 1, y: 1, badgeX: 1, badgeY: 1 });
  assert.throws(() => validateScenes(changed, image), /repeats/);
});

test('a valid annotation edit can be re-reviewed while invalid coordinates and state tampering cannot', () => {
  const images = [{ file: '01_title.png', sha256: 'pixels', width: 256, height: 240 }];
  const state = '{"captures":[]}';
  const candidate = { rom: { sha256: 'rom' }, driver: { sha256: 'driver' }, annotations: { sha256: 'old annotation bytes' }, images, captureState: { sha256: createHash('sha256').update(state).digest('hex') }, captureStateRaw: state };
  const edited = scene('01_title'); edited.scene.caption = 'A revised, valid note.';
  prepareReview(candidate, { romSha256: 'rom', driverSha256: 'driver', annotationSha256: 'new annotation bytes', scenes: edited }, images);
  assert.equal(candidate.annotations.sha256, 'new annotation bytes');
  const offFrame = scene('01_title'); offFrame.scene.annotations[0].badgeY = 241;
  assert.throws(() => prepareReview({ ...candidate }, { romSha256: 'rom', driverSha256: 'driver', annotationSha256: 'another annotation bytes', scenes: offFrame }, images), /outside/);
  assert.throws(() => prepareReview({ ...candidate, captureStateRaw: '{"captures":["tampered"]}' }, { romSha256: 'rom', driverSha256: 'driver', annotationSha256: 'new annotation bytes', scenes: edited }, images), /capture-state/);
});
