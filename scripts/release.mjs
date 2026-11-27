import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { basename, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const installers = {
  'aarch64-apple-darwin': ['.dmg'],
  'x86_64-apple-darwin': ['.dmg'],
  'x86_64-unknown-linux-gnu': ['.deb', '.AppImage'],
  'x86_64-pc-windows-msvc': ['.exe', '.msi'],
};

export const bundleDirectories = {
  '.dmg': 'dmg',
  '.app.tar.gz': 'macos',
  '.deb': 'deb',
  '.AppImage': 'appimage',
  '.exe': 'nsis',
  '.msi': 'msi',
};

export const updaterBundles = {
  'aarch64-apple-darwin': [
    { extension: '.app.tar.gz', platforms: ['darwin-aarch64', 'darwin-aarch64-app'] },
  ],
  'x86_64-apple-darwin': [
    { extension: '.app.tar.gz', platforms: ['darwin-x86_64', 'darwin-x86_64-app'] },
  ],
  'x86_64-unknown-linux-gnu': [
    { extension: '.AppImage', platforms: ['linux-x86_64', 'linux-x86_64-appimage'] },
    { extension: '.deb', platforms: ['linux-x86_64-deb'] },
  ],
  'x86_64-pc-windows-msvc': [
    { extension: '.exe', platforms: ['windows-x86_64', 'windows-x86_64-nsis'] },
    { extension: '.msi', platforms: ['windows-x86_64-msi'] },
  ],
};

export function parseTag(tag) {
  if (typeof tag !== 'string' || tag.trim() !== tag
      || !/^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(tag)) {
    throw new Error('Expected a stable app tag in vMAJOR.MINOR.PATCH format');
  }
  return tag.slice(1);
}

function parseRepository(repository) {
  if (typeof repository !== 'string' || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
    throw new Error('Expected a GitHub repository in OWNER/REPO format');
  }
  return repository;
}

function git(root, ...args) {
  return execFileSync('git', args, { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function packageVersion(text, name) {
  const sections = text.split(/^\[\[?package\]?\]\s*$/m);
  const section = name === undefined ? sections[1] : sections.find(value =>
    new RegExp(`^name = "${name}"$`, 'm').test(value));
  const version = section?.match(/^version = "([^"]+)"$/m)?.[1];
  if (!version) throw new Error(`Missing explicit package version: ${name ?? 'app'}`);
  return version;
}

export function checkVersions(root, version, read = path => readFileSync(join(root, path), 'utf8')) {
  const versions = {
    'eattura/package.json': JSON.parse(read('eattura/package.json')).version,
    'eattura/src-tauri/tauri.conf.json': JSON.parse(read('eattura/src-tauri/tauri.conf.json')).version,
    'eattura/src-tauri/Cargo.toml': packageVersion(read('eattura/src-tauri/Cargo.toml')),
    'Cargo.lock': packageVersion(read('Cargo.lock'), 'eattura'),
  };
  const expected = version ?? versions['eattura/package.json'];
  parseTag(`v${expected}`);
  for (const [file, actual] of Object.entries(versions)) {
    if (actual !== expected) throw new Error(`${file}: version ${actual} does not match ${expected}`);
  }
  return expected;
}

export function validateTag(root, tag, expectedCommit) {
  const version = parseTag(tag);
  const commit = git(root, 'rev-parse', '--verify', `refs/tags/${tag}^{commit}`);
  if (expectedCommit && commit !== expectedCommit) throw new Error('Release tag moved after validation');
  try {
    git(root, 'merge-base', '--is-ancestor', commit, 'refs/remotes/origin/main');
  } catch {
    throw new Error('Release tag must reference a commit belonging to origin/main');
  }
  checkVersions(root, version, path => git(root, 'show', `${commit}:${path}`));
  return { tag, version, commit };
}

export function protoModule(lock) {
  const commit = lock.match(/name: buf\.build\/bufbuild\/protovalidate\s+commit: ([a-f0-9]{32})\b/)?.[1];
  if (!commit) throw new Error('Missing pinned protovalidate commit in proto/buf.lock');
  return `buf.build/bufbuild/protovalidate:${commit}`;
}

function topLevelBundleFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Unexpected symbolic link: ${path}`);
    return entry.isDirectory() ? [] : [path];
  });
}

function expectedExtensions(target) {
  const extensions = new Set(installers[target]);
  for (const updater of updaterBundles[target] ?? []) {
    extensions.add(updater.extension);
  }
  return [...extensions];
}

export function stage(root, target) {
  if (!installers[target]) throw new Error('Unknown build target');
  const updaterExtensions = new Set((updaterBundles[target] ?? []).map(entry => entry.extension));
  const files = expectedExtensions(target).flatMap(extension => {
    const directory = join(root, 'target', target, 'release', 'bundle', bundleDirectories[extension]);
    const matches = existsSync(directory) ? topLevelBundleFiles(directory).filter(path => path.endsWith(extension)) : [];
    if (matches.length === 0) throw new Error(`Missing ${target} bundle: ${extension}`);
    if (matches.length > 1) throw new Error(`Multiple ${target} bundles matched ${extension}`);
    if (!updaterExtensions.has(extension)) return matches;
    const signature = `${matches[0]}.sig`;
    if (!existsSync(signature)) throw new Error(`Missing ${target} updater signature: ${extension}.sig`);
    return [matches[0], signature];
  });
  const destination = join(root, 'release-artifacts');
  mkdirSync(destination, { recursive: true });
  const names = new Set();
  for (const file of files) {
    const rawName = basename(file);
    if (!/^[A-Za-z0-9._+-]+$/.test(rawName)) throw new Error(`Unsafe release asset filename: ${rawName}`);
    const name = `${target}-${rawName}`;
    if (names.has(name)) throw new Error(`Duplicate installer: ${name}`);
    names.add(name);
    if (readFileSync(file).length === 0) throw new Error(`Empty installer: ${name}`);
    copyFileSync(file, join(destination, name));
  }
  return [...names].sort();
}

function allowedAssetExtensions(target) {
  const allowed = [...expectedExtensions(target)];
  for (const updater of updaterBundles[target] ?? []) {
    allowed.push(`${updater.extension}.sig`);
  }
  return allowed.sort((a, b) => b.length - a.length);
}

export function collect(root, options = {}) {
  const tag = options.tag ?? process.env.RELEASE_TAG;
  const version = parseTag(tag);
  const repository = parseRepository(options.repository ?? process.env.GH_REPO ?? 'mindmain/eattura');
  const notes = (options.notes ?? process.env.RELEASE_NOTES ?? '').trim();
  const pubDate = options.pubDate ?? process.env.RELEASE_PUB_DATE ?? new Date().toISOString();
  if (Number.isNaN(Date.parse(pubDate))) throw new Error(`Invalid release publication date: ${pubDate}`);

  const directory = join(root, 'release-artifacts');
  const stagedFiles = readdirSync(directory)
    .filter(name => name !== 'SHA256SUMS' && name !== 'latest.json')
    .sort();

  for (const name of stagedFiles) {
    const valid = Object.keys(installers).some(target =>
      name.startsWith(`${target}-`)
      && allowedAssetExtensions(target).some(extension => name.endsWith(extension)));
    if (!valid) throw new Error(`Unexpected release asset: ${name}`);
  }

  for (const [target, extensions] of Object.entries(installers)) {
    for (const extension of extensions) {
      if (!stagedFiles.some(name => name.startsWith(`${target}-`) && name.endsWith(extension))) {
        throw new Error(`Missing release installer: ${target} ${extension}`);
      }
    }
  }

  const platforms = {};
  for (const [target, entries] of Object.entries(updaterBundles)) {
    for (const { extension, platforms: keys } of entries) {
      const asset = stagedFiles.find(name => name.startsWith(`${target}-`) && name.endsWith(extension));
      if (!asset) throw new Error(`Missing updater bundle: ${target} ${extension}`);
      const sigAsset = `${asset}.sig`;
      if (!stagedFiles.includes(sigAsset)) throw new Error(`Missing updater signature: ${sigAsset}`);
      const signature = readFileSync(join(directory, sigAsset), 'utf8').trim();
      if (!signature) throw new Error(`Empty updater signature: ${sigAsset}`);
      const url = `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(asset)}`;
      for (const key of keys) {
        platforms[key] = { signature, url };
      }
    }
  }

  const manifest = {
    version,
    ...(notes ? { notes } : {}),
    pub_date: new Date(pubDate).toISOString(),
    platforms,
  };
  writeFileSync(join(directory, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`);

  const hashedFiles = [...stagedFiles, 'latest.json'].sort();
  const checksums = hashedFiles.map(name => {
    const content = readFileSync(join(directory, name));
    if (content.length === 0) throw new Error(`Empty release asset: ${name}`);
    return `${createHash('sha256').update(content).digest('hex')}  ${name}\n`;
  }).join('');
  writeFileSync(join(directory, 'SHA256SUMS'), checksums);
  return checksums;
}

function main() {
  const root = process.cwd();
  const [command, argument, expectedCommit] = process.argv.slice(2);
  switch (command) {
    case 'validate': {
      const result = validateTag(root, argument, expectedCommit);
      if (process.env.GITHUB_OUTPUT) {
        appendFileSync(process.env.GITHUB_OUTPUT, Object.entries(result).map(([key, value]) => `${key}=${value}\n`).join(''));
      }
      console.log(JSON.stringify(result));
      break;
    }
    case 'check':
      console.log(checkVersions(root, argument ? parseTag(argument) : undefined));
      break;
    case 'proto': {
      const module = protoModule(readFileSync(join(root, 'proto/buf.lock'), 'utf8'));
      const destination = join(root, 'proto/vendor');
      if (existsSync(destination)) throw new Error('proto/vendor already exists; remove it before exporting pinned definitions');
      execFileSync('buf', ['export', module, '--output', destination], { stdio: 'inherit' });
      break;
    }
    case 'stage':
      console.log(stage(root, argument).join('\n'));
      break;
    case 'collect':
      console.log(collect(root, argument ? { tag: argument } : {}));
      break;
    default:
      throw new Error('Usage: node scripts/release.mjs <validate TAG [COMMIT]|check [TAG]|proto|stage TARGET|collect [TAG]>');
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
