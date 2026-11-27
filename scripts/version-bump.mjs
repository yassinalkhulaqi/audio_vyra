// Update every version string in the workspace.
// Usage: node scripts/version-bump.mjs 0.2.0
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, '..');
const version = process.argv[2];

if (!/^\d+\.\d+\.\d+$/.test(version || '')) {
  console.error('usage: node scripts/version-bump.mjs <major.minor.patch>');
  process.exit(1);
}

const files = [
  'apps/desktop/src-tauri/tauri.conf.json',
  'crates/aurivo-core/Cargo.toml',
  'crates/aurivo-dsp/Cargo.toml',
  'crates/aurivo-audio-engine/Cargo.toml',
  'apps/desktop/src-tauri/Cargo.toml',
];

for (const relative of files) {
  const path = join(root, relative);
  const text = readFileSync(path, 'utf8');
  const updated = text.replace(/version\s*=\s*"\d+\.\d+\.\d+"/g, `version = "${version}"`)
    .replace(/"version":\s*"\d+\.\d+\.\d+"/g, `"version": "${version}"`);
  if (updated === text) {
    console.warn(`no version found in ${relative}`);
    continue;
  }
  writeFileSync(path, updated);
  console.log(`updated ${relative}`);
}

console.log(`\nNext steps:\n  1. cargo check --workspace\n  2. update CHANGELOG.md\n  3. git commit -am "chore: release v${version}"\n  4. git tag -a v${version} -m "AURIVO ${version}"`);
