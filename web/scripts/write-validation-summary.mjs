import { createHash } from 'node:crypto';
import { readFile, readdir, rename, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../../', import.meta.url));
const reportDirectory = path.join(root, 'out/reports/web');
const report = JSON.parse(await readFile(path.join(reportDirectory, 'playwright.json'), 'utf8'));
const { expected, unexpected, skipped, flaky } = report.stats;
if (!(expected > 0) || unexpected !== 0 || skipped !== 0 || flaky !== 0) {
  throw new Error('Refusing a PASS summary for incomplete or unsuccessful browser tests');
}
const screenshots = (await readdir(path.join(reportDirectory, 'screens')))
  .filter(name => name.endsWith('.png')).sort().map(name => `out/reports/web/screens/${name}`);
if (!screenshots.length) throw new Error('No screenshots to report');
const digest = async relative => createHash('sha256').update(await readFile(path.join(root, relative))).digest('hex');
const summary = {
  status: 'PASS',
  rom_sha256: await digest('out/smash.nes'),
  site_entry_sha256: await digest('web/dist/index.html'),
  playwright_report: 'out/reports/web/playwright.json',
  screenshots,
};
const destination = path.join(reportDirectory, 'summary.json');
// Write atomically; a stopped export must not leave an empty success report.
await writeFile(`${destination}.tmp`, JSON.stringify(summary, null, 2) + '\n');
await rename(`${destination}.tmp`, destination);
console.log(`[web-test] PASS: out/reports/web/summary.json (${expected} tests, ${screenshots.length} screenshots)`);
