import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { bundleDirectories, checkVersions, collect, installers, parseTag, protoModule, stage, validateTag } from './release.mjs';

function fixture(context) {
  const directory = mkdtempSync(join(tmpdir(), 'eattura-release-'));
  context.after(() => rmSync(directory, { recursive: true, force: true }));
  return directory;
}

function versions(root, version = '0.1.0') {
  mkdirSync(join(root, 'eattura/src-tauri'), { recursive: true });
  writeFileSync(join(root, 'eattura/package.json'), JSON.stringify({ version }));
  writeFileSync(join(root, 'eattura/src-tauri/tauri.conf.json'), JSON.stringify({ version }));
  writeFileSync(join(root, 'eattura/src-tauri/Cargo.toml'), `[package]\nname = "eattura"\nversion = "${version}"\n`);
  writeFileSync(join(root, 'Cargo.lock'), `version = 4\n[[package]]\nname = "common"\nversion = "9.0.0"\n[[package]]\nname = "eattura"\nversion = "${version}"\n`);
}

test('accepts only canonical stable app tags', () => {
  for (const tag of ['v0.1.0', 'v1.20.300', 'v0.0.0']) assert.equal(parseTag(tag), tag.slice(1));
  for (const tag of ['', undefined, '0.1.0', 'V0.1.0', 'v01.1.0', 'v0.01.0', 'v0.1.00', 'v1.2.3-rc.1', 'v1.2.3+build', 'v1.2.3\n', 'v1.2.3/evil']) {
    assert.throws(() => parseTag(tag));
  }
});

test('release configuration versions only the desktop and disables updater artifacts', () => {
  const config = JSON.parse(readFileSync(new URL('../release-please-config.json', import.meta.url), 'utf8'));
  assert.deepEqual(Object.keys(config.packages), ['eattura']);
  const app = config.packages.eattura;
  assert.equal(app['initial-version'], '0.1.0');
  assert.equal(app['include-component-in-tag'], false);
  assert.equal(app['include-v-in-tag'], true);
  assert.equal(app.draft, true);
  assert.equal(app['force-tag-creation'], true);
  assert.equal(app['release-as'], undefined);
  assert.deepEqual(app['extra-files'].map(file => file.path), [
    'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', '/Cargo.lock',
  ]);
  assert.equal(app['extra-files'][2].jsonpath, "$.package[?(@.name.value == 'eattura')].version");
  const tauri = JSON.parse(readFileSync(new URL('../eattura/src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
  assert.equal(tauri.bundle.createUpdaterArtifacts, false);
});

test('checks only the app versions and rejects manifest drift', context => {
  const root = fixture(context);
  versions(root);
  assert.equal(checkVersions(root, '0.1.0'), '0.1.0');
  assert.throws(() => checkVersions(root, '0.2.0'));
  writeFileSync(join(root, 'eattura/src-tauri/Cargo.toml'), '[package]\nversion.workspace = true\n');
  assert.throws(() => checkVersions(root, '0.1.0'), /explicit package version/);
});

test('validates tag ancestry, tagged versions and tag immutability', context => {
  const root = fixture(context);
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  git('init', '-b', 'main');
  git('config', 'user.name', 'Release Test');
  git('config', 'user.email', 'release-test@example.invalid');
  git('config', 'commit.gpgsign', 'false');
  git('config', 'tag.gpgsign', 'false');
  versions(root);
  git('add', '.');
  git('commit', '-m', 'feat: initial app');
  const commit = git('rev-parse', 'HEAD');
  git('update-ref', 'refs/remotes/origin/main', commit);
  git('tag', 'v0.1.0');
  assert.equal(validateTag(root, 'v0.1.0').commit, commit);
  assert.throws(() => validateTag(root, 'v0.1.0', '0'.repeat(40)), /moved/);
  git('tag', '-a', 'v0.1.1', '-m', 'Wrong version');
  assert.throws(() => validateTag(root, 'v0.1.1'), /does not match/);
  git('checkout', '-b', 'feature');
  versions(root, '0.2.0');
  git('add', '.');
  git('commit', '-m', 'feat: unmerged change');
  git('tag', '-a', 'v0.2.0', '-m', 'App release');
  assert.throws(() => validateTag(root, 'v0.2.0'), /origin\/main/);
  git('update-ref', 'refs/remotes/origin/main', git('rev-parse', 'HEAD'));
  assert.equal(validateTag(root, 'v0.2.0').version, '0.2.0');
  assert.equal(validateTag(root, 'v0.1.0').version, '0.1.0');
});

test('exports the locked protobuf module rather than a floating version', () => {
  const commit = '80ab13bee0bf4272b6161a72bf7034e0';
  assert.equal(protoModule(`deps:\n  - name: buf.build/bufbuild/protovalidate\n    commit: ${commit}\n`), `buf.build/bufbuild/protovalidate:${commit}`);
  assert.throws(() => protoModule('deps: []'));
});

test('stages all platform installers and hashes only complete releases', context => {
  const root = fixture(context);
  for (const [target, extensions] of Object.entries(installers)) {
    const bundle = join(root, 'target', target, 'release/bundle');
    mkdirSync(bundle, { recursive: true });
    for (const extension of extensions) {
      const directory = join(bundle, bundleDirectories[extension]);
      mkdirSync(directory);
      writeFileSync(join(directory, `eattura${extension}`), 'installer');
    }
    writeFileSync(join(bundle, 'ignored.zip'), 'not an installer');
    assert.equal(stage(root, target).length, extensions.length);
  }
  const checksums = collect(root);
  assert.equal(checksums.trim().split('\n').length, 6);
  assert.match(checksums, /^9c0d294c05fc1d88d698034609bb81c0c69196327594e4c69d2915c80fd9850c  /);
  assert.equal(readFileSync(join(root, 'release-artifacts/SHA256SUMS'), 'utf8'), checksums);
  assert.equal(collect(root), checksums);
  writeFileSync(join(root, 'release-artifacts/unexpected.zip'), 'not an installer');
  assert.throws(() => collect(root), /Unexpected release asset/);
  rmSync(join(root, 'release-artifacts/unexpected.zip'));
  rmSync(join(root, 'release-artifacts/x86_64-pc-windows-msvc-eattura.msi'));
  assert.throws(() => collect(root), /Missing release installer/);
});

test('rejects missing, empty and unexpected artifacts', context => {
  const root = fixture(context);
  const target = 'aarch64-apple-darwin';
  const bundle = join(root, 'target', target, 'release/bundle');
  mkdirSync(bundle, { recursive: true });
  assert.throws(() => stage(root, target), /Missing/);
  mkdirSync(join(bundle, 'dmg'));
  writeFileSync(join(bundle, 'dmg/empty.dmg'), '');
  assert.throws(() => stage(root, target), /Empty/);
  assert.throws(() => stage(root, 'unknown'), /Unknown/);
});
