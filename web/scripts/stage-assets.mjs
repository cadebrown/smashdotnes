import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { checkApprovedManualAssets } from './refresh-manual.mjs';
const root = fileURLToPath(new URL('../', import.meta.url));
const output = path.join(root, '.public');
await checkApprovedManualAssets();
// This is a generated staging tree; removed assets must not survive the next build.
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
for (const directory of ['assets', 'rom']) {
  await cp(path.join(root, 'public', directory), path.join(output, directory), { recursive: true });
}
await cp(path.join(root, 'public/assets/captures'), path.join(output, 'snaps'), { recursive: true });
await mkdir(path.join(output, 'info'), { recursive: true });
await cp(path.join(root, 'public/info/build.json'), path.join(output, 'info/build.json'));
await cp(path.join(root, 'public/input-focus.js'), path.join(output, 'input-focus.js'));
await mkdir(path.join(output, 'assets/fighters'), { recursive: true });
for (const fighter of ['mario','simon','tommy','rizer']) {
  await cp(path.join(root, '..', 'src/chars', fighter, 'sheet.png'), path.join(output, 'assets/fighters', `${fighter}.png`));
}
const metadata = JSON.parse(await readFile(path.join(output, 'info/build.json'), 'utf8'));
if (Object.hasOwn(metadata.git ?? {}, 'remote')) throw new Error('Build metadata contains a forbidden field');
const rom = await readFile(path.join(output, 'rom/smash.nes'));
if (!rom.subarray(0, 4).equals(Buffer.from([0x4e, 0x45, 0x53, 0x1a]))) {
  throw new Error('Expected an iNES ROM. Run make web-sync before building the site.');
}
await writeFile(path.join(output, 'robots.txt'), 'User-agent: *\nAllow: /\n');
console.log('Staged runtime, native captures, fighter sheets, and build metadata.');
