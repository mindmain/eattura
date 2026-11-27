import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { bundleDirectories, checkVersions, collect, installers, parseTag, protoModule, stage, updaterBundles, validateTag } from './release.mjs';

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

test('release configuration versions only the desktop and enables signed updater artifacts', () => {
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
  assert.equal(tauri.bundle.createUpdaterArtifacts, true);
  assert.ok(tauri.plugins?.updater?.pubkey);
  assert.equal(tauri.plugins.updater.requireSignedVersion, true);
  assert.deepEqual(tauri.plugins.updater.endpoints, [
    'https://github.com/mindmain/eattura/releases/latest/download/latest.json',
  ]);
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

test('stages installers and updater signatures and writes latest.json plus checksums', context => {
  const root = fixture(context);
  for (const [target, extensions] of Object.entries(installers)) {
    const bundle = join(root, 'target', target, 'release/bundle');
    mkdirSync(bundle, { recursive: true });
    const allExtensions = new Set([
      ...extensions,
      ...updaterBundles[target].map(entry => entry.extension),
    ]);
    for (const extension of allExtensions) {
      const directory = join(bundle, bundleDirectories[extension]);
      mkdirSync(directory, { recursive: true });
      writeFileSync(join(directory, `eattura${extension}`), 'installer');
    }
    for (const { extension } of updaterBundles[target]) {
      const directory = join(bundle, bundleDirectories[extension]);
      writeFileSync(join(directory, `eattura${extension}.sig`), `sig-${target}-${extension}`);
    }
    if (target === 'x86_64-unknown-linux-gnu') {
      const appDir = join(bundle, 'appimage/eattura.AppDir');
      mkdirSync(appDir, { recursive: true });
      symlinkSync('usr/share/icons/hicolor/128x128/apps/eattura.png', join(appDir, '.DirIcon'));
    }
    writeFileSync(join(bundle, 'ignored.zip'), 'not an installer');
    const staged = stage(root, target);
    assert.ok(staged.length >= extensions.length + updaterBundles[target].length);
  }
  const pubDate = '2026-11-25T10:00:00.000Z';
  const checksums = collect(root, {
    tag: 'v0.1.0',
    repository: 'mindmain/eattura',
    notes: 'First desktop release',
    pubDate,
  });
  const manifest = JSON.parse(readFileSync(join(root, 'release-artifacts/latest.json'), 'utf8'));
  assert.equal(manifest.version, '0.1.0');
  assert.equal(manifest.notes, 'First desktop release');
  assert.equal(manifest.pub_date, pubDate);
  assert.deepEqual(Object.keys(manifest.platforms).sort(), [
    'darwin-aarch64',
    'darwin-aarch64-app',
    'darwin-x86_64',
    'darwin-x86_64-app',
    'linux-x86_64',
    'linux-x86_64-appimage',
    'linux-x86_64-deb',
    'windows-x86_64',
    'windows-x86_64-msi',
    'windows-x86_64-nsis',
  ]);
  assert.equal(
    manifest.platforms['darwin-aarch64'].url,
    'https://github.com/mindmain/eattura/releases/download/v0.1.0/aarch64-apple-darwin-eattura.app.tar.gz',
  );
  assert.equal(manifest.platforms['darwin-aarch64'].signature, 'sig-aarch64-apple-darwin-.app.tar.gz');
  assert.equal(checksums.trim().split('\n').length, 15);
  assert.equal(readFileSync(join(root, 'release-artifacts/SHA256SUMS'), 'utf8'), checksums);
  assert.equal(collect(root, { tag: 'v0.1.0', repository: 'mindmain/eattura', notes: 'First desktop release', pubDate }), checksums);

  writeFileSync(join(root, 'release-artifacts/unexpected.zip'), 'not an installer');
  assert.throws(() => collect(root, { tag: 'v0.1.0' }), /Unexpected release asset/);
  rmSync(join(root, 'release-artifacts/unexpected.zip'));
  rmSync(join(root, 'release-artifacts/x86_64-pc-windows-msvc-eattura.msi.sig'));
  assert.throws(() => collect(root, { tag: 'v0.1.0' }), /Missing updater signature/);
  rmSync(join(root, 'release-artifacts/x86_64-pc-windows-msvc-eattura.msi'));
  assert.throws(() => collect(root, { tag: 'v0.1.0' }), /Missing release installer/);
});

test('rejects missing, empty and unexpected artifacts', context => {
  const root = fixture(context);
  const target = 'aarch64-apple-darwin';
  const bundle = join(root, 'target', target, 'release/bundle');
  mkdirSync(bundle, { recursive: true });
  assert.throws(() => stage(root, target), /Missing/);
  mkdirSync(join(bundle, 'dmg'));
  mkdirSync(join(bundle, 'macos'));
  writeFileSync(join(bundle, 'dmg/empty.dmg'), '');
  writeFileSync(join(bundle, 'macos/eattura.app.tar.gz'), 'archive');
  writeFileSync(join(bundle, 'macos/eattura.app.tar.gz.sig'), 'sig');
  assert.throws(() => stage(root, target), /Empty/);
  rmSync(join(bundle, 'dmg/empty.dmg'));
  symlinkSync(join(bundle, 'macos/eattura.app.tar.gz'), join(bundle, 'dmg/linked.dmg'));
  assert.throws(() => stage(root, target), /Unexpected symbolic link/);
  assert.throws(() => stage(root, 'unknown'), /Unknown/);
});
