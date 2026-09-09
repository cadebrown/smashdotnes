import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, rm, stat, unlink, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { compareImages, fileHash, inspectPngDirectory, validateScenes } from './manual-capture-check.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const web = path.join(root, 'web');
const refresh = path.join(root, 'out/reports/manual-refresh');
const captures = path.join(refresh, 'captures');
const candidateFile = path.join(refresh, 'candidate.json');
const galleryFile = path.join(refresh, 'index.html');
const annotationFile = path.join(web, 'src/data/manual-scenes.json');
const driverFile = path.join(root, 'nessy/tools/emulator/manual_capture.lua');
const romFile = path.join(root, 'out/smash.nes');
const publicRom = path.join(web, 'public/rom/smash.nes');
const lockFile = path.join(web, 'src/data/manual-capture-lock.json');
const publicCaptures = path.join(web, 'public/assets/captures');
const snaps = path.join(root, 'out/snaps');

const digest = value => createHash('sha256').update(value).digest('hex');
const exists = async file => stat(file).then(() => true, () => false);
const readJson = async file => JSON.parse(await readFile(file, 'utf8'));
const writeJson = async (file, value) => writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
const relative = file => path.relative(root, file).split(path.sep).join('/');

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', code => code === 0 ? resolve() : reject(new Error(`${command} ${args.join(' ')} failed with exit ${code}`)));
  });
}

async function sourceFacts() {
  const annotationsRaw = await readFile(annotationFile);
  return {
    annotationsRaw,
    scenes: JSON.parse(annotationsRaw),
    romSha256: await fileHash(romFile),
    driverSha256: await fileHash(driverFile),
    annotationSha256: digest(annotationsRaw),
  };
}

async function gitFacts() {
  const git = args => new Promise((resolve, reject) => {
    const child = spawn('git', args, { cwd: root });
    let text = ''; child.stdout.on('data', chunk => text += chunk); child.on('error', reject); child.on('exit', code => code ? reject(new Error('git status failed')) : resolve(text));
  });
  const [revision, branch, status] = await Promise.all([git(['rev-parse', 'HEAD']), git(['branch', '--show-current']), git(['status', '--porcelain=v1'])]);
  return { revision: revision.trim(), branch: branch.trim() || 'detached', dirty: Boolean(status.trim()) };
}

function captureState(candidate) {
  if (typeof candidate.captureStateRaw !== 'string' || candidate.captureState?.sha256 !== digest(candidate.captureStateRaw)) {
    throw new Error('Candidate capture-state.json changed or is missing. Run make manual-refresh.');
  }
  try { return JSON.parse(candidate.captureStateRaw); } catch { throw new Error('Candidate capture-state.json is invalid JSON. Run make manual-refresh.'); }
}

export function prepareReview(candidate, facts, images) {
  if (candidate.rom?.sha256 !== facts.romSha256) throw new Error('ROM changed since capture. Run make manual-refresh.');
  if (candidate.driver?.sha256 !== facts.driverSha256) throw new Error('Capture driver changed since capture. Run make manual-refresh.');
  compareImages(candidate.images, images, 'Candidate captures');
  captureState(candidate);
  validateScenes(facts.scenes, images);
  candidate.annotations = { sha256: facts.annotationSha256 };
  candidate.review = { inputDigest: digest(JSON.stringify({ rom: facts.romSha256, driver: facts.driverSha256, annotations: facts.annotationSha256, images, captureStateSha256: candidate.captureState.sha256 })), annotationSha256: facts.annotationSha256, captureStateSha256: candidate.captureState.sha256 };
  return candidate;
}

async function candidateIsCurrent(candidate, { allowAnnotationDrift = false } = {}) {
  const facts = await sourceFacts();
  const images = await inspectPngDirectory(captures);
  const currentStateRaw = await readFile(path.join(captures, 'capture-state.json'), 'utf8');
  if (currentStateRaw !== candidate.captureStateRaw || digest(currentStateRaw) !== candidate.captureState?.sha256) {
    throw new Error('capture-state.json changed since capture. Run make manual-refresh.');
  }
  if (candidate.rom?.sha256 !== facts.romSha256) throw new Error('ROM changed since capture. Run make manual-refresh.');
  if (candidate.driver?.sha256 !== facts.driverSha256) throw new Error('Capture driver changed since capture. Run make manual-refresh.');
  compareImages(candidate.images, images, 'Candidate captures');
  captureState(candidate);
  validateScenes(facts.scenes, images);
  if (allowAnnotationDrift) prepareReview(candidate, facts, images);
  else if (candidate.annotations?.sha256 !== facts.annotationSha256) throw new Error('Manual annotations changed since review. Run make manual-review.');
  return { facts, images };
}

const escape = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const connector = ({ x, y, badgeX, badgeY }) => {
  const midX = (x + badgeX) / 2, midY = (y + badgeY) / 2;
  const bend = badgeX === x ? 10 : badgeY === y ? -10 : (badgeX < x ? -9 : 9);
  return `M ${badgeX} ${badgeY} Q ${midX + bend} ${midY - bend} ${x} ${y}`;
};

function renderGallery(candidate, scenes) {
  const imageByName = new Map(candidate.images.map(image => [image.file, image]));
  const ordered = Object.entries(scenes).map(([key, scene]) => [key, scene, imageByName.get(`${scene.image}.png`)]).filter(([, , image]) => image);
  const seen = new Set(ordered.map(([, , image]) => image.file));
  const stateByFile = new Map((captureState(candidate).captures ?? []).map(item => [item.file, item]));
  const tile = (name, image, scene, changed, index) => {
    const state = stateByFile.get(name);
    const marker = `manual-arrow-${index}`;
    const marks = (scene?.annotations ?? []).map(a => `<g><path d="${connector(a)}" marker-end="url(#${marker})"/><circle class="target" cx="${a.x}" cy="${a.y}" r="5"/><circle class="badge" cx="${a.badgeX}" cy="${a.badgeY}" r="8"/><text x="${a.badgeX}" y="${a.badgeY}" text-anchor="middle">${escape(a.label)}</text>${a.tag ? `<g class="tag" transform="translate(${a.tagX ?? a.badgeX},${a.tagY ?? a.badgeY + 17})"><rect x="-3" y="-11" width="${String(a.tag).length * 6.2 + 6}" height="16"/><text>${escape(a.tag)}</text></g>` : ''}</g>`).join('');
    const stateInfo = state ? `<p class="state"><b>Captured state:</b> ${escape(state.observed ?? 'no predicate text')} <span>Predicate observed before the display tick.</span></p>` : '<p class="state">No capture-state entry was recorded for this tile.</p>';
    return `<article class="tile${changed ? ' changed' : ''}"><h2>${escape(scene?.title ?? name)}${changed ? ' <small>changed</small>' : ''}</h2><div class="frame"><img src="captures/${encodeURIComponent(name)}" alt="${escape(scene?.alt ?? name)}"><svg viewBox="0 0 256 240" aria-label="Annotation overlay"><defs><marker id="${marker}" markerWidth="7" markerHeight="7" refX="5.5" refY="3.5" orient="auto" markerUnits="strokeWidth"><path d="M0,0 L7,3.5 L0,7" fill="none" stroke="currentColor" stroke-width="1.4"/></marker></defs>${marks}</svg></div>${scene ? `<p>${escape(scene.caption)}</p><ol>${scene.annotations.map(a => `<li><b>${escape(a.label)}</b> ${escape(a.note)}</li>`).join('')}</ol>` : '<p>Unused capture retained as a raw review tile.</p>'}${stateInfo}<code>${escape(image.sha256)}</code></article>`;
  };
  const approved = new Map((candidate.changedVsApproved ?? []).filter(item => !item.changed).map(item => [item.file, true]));
  return `<!doctype html><meta charset="utf-8"><title>Smash.NES manual capture review</title><style>@font-face{font-family:Caveat;font-style:normal;font-weight:600;src:url(caveat.woff2) format("woff2")}body{margin:2rem;background:#e9dfca;color:#20170f;font:16px system-ui}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(340px,1fr));gap:2rem}.tile{border:2px solid #4d3826;padding:1rem;background:#f9f1df}.changed{border-color:#bb3924}.frame{position:relative;max-width:512px}.frame img,.frame svg{display:block;width:100%;height:auto}.frame svg{position:absolute;inset:0;overflow:visible;color:#a3291d;pointer-events:none}.frame svg path{fill:none;stroke:currentColor;stroke-width:1.55;stroke-linecap:round;vector-effect:non-scaling-stroke}.frame .target,.frame .badge{fill:none;stroke:currentColor;stroke-width:1.5;vector-effect:non-scaling-stroke}.frame text{fill:currentColor;font:600 11px Caveat,cursive;dominant-baseline:middle}.frame .tag rect{fill:#e8dbb5;stroke:none;opacity:.95}.frame .tag text{font-size:13px;dominant-baseline:auto}.state{font-size:13px}.state span{color:#5f533d}small{color:#bb3924}code{font-size:11px;word-break:break-all}</style><h1>Manual capture review</h1><p>Prepared for local review. To revise annotations, edit <code>web/src/data/manual-scenes.json</code> and run <code>make manual-review</code>; this page itself does not save edits.</p><main>${ordered.map(([, scene, image], index) => tile(image.file, image, scene, !approved.has(image.file), index)).join('')}${candidate.images.filter(image => !seen.has(image.file)).map((image, index) => tile(image.file, image, null, !approved.has(image.file), ordered.length + index)).join('')}</main>`;
}

async function review() {
  const candidate = await readJson(candidateFile);
  const { facts } = await candidateIsCurrent(candidate, { allowAnnotationDrift: true });
  await cp(path.join(web, 'node_modules/@fontsource/caveat/files/caveat-latin-600-normal.woff2'), path.join(refresh, 'caveat.woff2'));
  await writeFile(galleryFile, renderGallery(candidate, facts.scenes));
  await writeJson(candidateFile, candidate);
  console.log(`Manual review ready: ${relative(galleryFile)}`);
}

async function refreshManual() {
  await rm(refresh, { recursive: true, force: true });
  await mkdir(captures, { recursive: true });
  await run('make', ['build']);
  const captureInputs = { romSha256: await fileHash(romFile), driverSha256: await fileHash(driverFile) };
  await run('cargo', ['run', '--quiet', '--package', 'nessy', '--', 'emulator', 'capture', '--script', 'nessy/tools/emulator/manual_capture.lua', '--output-dir', 'out/reports/manual-refresh/captures', '--report', 'out/reports/manual-refresh/capture.txt', '--timeout-seconds', '60']);
  await run('cargo', ['run', '--quiet', '--package', 'nessy', '--', 'test', 'screenshots', '--dir', 'out/reports/manual-refresh/captures']);
  const { annotationsRaw, scenes, romSha256, driverSha256, annotationSha256 } = await sourceFacts();
  if (captureInputs.romSha256 !== romSha256 || captureInputs.driverSha256 !== driverSha256) throw new Error('ROM or capture driver changed during capture; retry make manual-refresh.');
  const images = await inspectPngDirectory(captures);
  validateScenes(scenes, images);
  const approved = await exists(lockFile) ? await readJson(lockFile) : { images: [] };
  const prior = new Map((approved.images ?? []).map(image => [image.file, image.sha256]));
  const current = new Map(images.map(image => [image.file, image.sha256]));
  const changedVsApproved = [...new Set([...prior.keys(), ...current.keys()])].sort().map(file => ({ file, changed: prior.get(file) !== current.get(file) }));
  const captureStateRaw = await readFile(path.join(captures, 'capture-state.json'), 'utf8');
  JSON.parse(captureStateRaw);
  const candidate = { version: 1, rom: { sha256: romSha256 }, driver: { sha256: driverSha256 }, git: await gitFacts(), annotations: { sha256: annotationSha256 }, images, captureState: { sha256: digest(captureStateRaw) }, captureStateRaw, changedVsApproved };
  // Keep the digest material explicit: annotations are reviewed as bytes, not reformatted JSON.
  if (digest(annotationsRaw) !== annotationSha256) throw new Error('Annotation digest changed while refreshing; retry the refresh.');
  await writeJson(candidateFile, candidate);
  await review();
}

export async function checkApprovedManualAssets({ projectRoot = root } = {}) {
  const paths = projectRoot === root ? { lockFile, annotationFile, driverFile, publicRom, publicCaptures } : {
    lockFile: path.join(projectRoot, 'web/src/data/manual-capture-lock.json'), annotationFile: path.join(projectRoot, 'web/src/data/manual-scenes.json'), driverFile: path.join(projectRoot, 'nessy/tools/emulator/manual_capture.lua'), publicRom: path.join(projectRoot, 'web/public/rom/smash.nes'), publicCaptures: path.join(projectRoot, 'web/public/assets/captures'),
  };
  if (!await exists(paths.lockFile)) throw new Error('Manual captures are not approved. Run make manual-refresh, inspect the review gallery, then run make manual-accept.');
  const lock = await readJson(paths.lockFile);
  const [romSha256, driverSha256, annotationsRaw, images] = await Promise.all([fileHash(paths.publicRom), fileHash(paths.driverFile), readFile(paths.annotationFile), inspectPngDirectory(paths.publicCaptures)]);
  if (lock.romSha256 !== romSha256) throw new Error('Published ROM changed since manual capture approval. Run make manual-refresh and make manual-accept after make web-sync.');
  if (lock.driverSha256 !== driverSha256) throw new Error('Manual capture driver changed. Run make manual-refresh and make manual-accept.');
  if (lock.annotationSha256 !== digest(annotationsRaw)) throw new Error('Manual annotations changed. Run make manual-review and make manual-accept.');
  compareImages(lock.images, images, 'Approved manual captures');
  validateScenes(JSON.parse(annotationsRaw), images);
  return lock;
}

async function accept() {
  const candidate = await readJson(candidateFile);
  const { facts, images } = await candidateIsCurrent(candidate);
  if (!candidate.review?.inputDigest) throw new Error('Candidate has not been reviewed. Run make manual-review before accepting it.');
  const inputDigest = digest(JSON.stringify({ rom: facts.romSha256, driver: facts.driverSha256, annotations: facts.annotationSha256, images, captureStateSha256: candidate.captureState?.sha256 }));
  if (candidate.review.inputDigest !== inputDigest) throw new Error('Candidate changed after review. Run make manual-review again.');
  if (!await exists(publicRom) || await fileHash(publicRom) !== facts.romSha256) throw new Error('Published ROM differs from the reviewed ROM. Run make web-sync, then make manual-accept.');
  await mkdir(publicCaptures, { recursive: true }); await mkdir(snaps, { recursive: true });
  const previous = await exists(lockFile) ? await readJson(lockFile) : { images: [] };
  const currentNames = new Set(images.map(image => image.file));
  for (const image of previous.images ?? []) if (!currentNames.has(image.file)) {
    await Promise.all([publicCaptures, snaps].map(async directory => {
      const stale = path.join(directory, image.file); if (await exists(stale)) await unlink(stale);
    }));
  }
  for (const image of images) { await cp(path.join(captures, image.file), path.join(publicCaptures, image.file)); await cp(path.join(captures, image.file), path.join(snaps, image.file)); }
  await writeJson(lockFile, { version: 1, romSha256: facts.romSha256, driverSha256: facts.driverSha256, annotationSha256: facts.annotationSha256, images });
  await checkApprovedManualAssets();
  console.log('Manual captures accepted into web/public/assets/captures and out/snaps.');
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const mode = process.argv[2] ?? '--check';
  try {
    if (mode === '--refresh') await refreshManual(); else if (mode === '--review') await review(); else if (mode === '--accept') await accept(); else if (mode === '--check') { await checkApprovedManualAssets(); console.log('Manual capture approval is current.'); } else throw new Error(`Unknown mode ${mode}. Use --refresh, --review, --accept, or --check.`);
  } catch (error) { console.error(`manual refresh: ${error.message}`); process.exitCode = 1; }
}
