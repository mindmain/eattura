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
  '.deb': 'deb',
  '.AppImage': 'appimage',
  '.exe': 'nsis',
  '.msi': 'msi',
};

export function parseTag(tag) {
  if (typeof tag !== 'string' || tag.trim() !== tag
      || !/^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(tag)) {
    throw new Error('Expected a stable app tag in vMAJOR.MINOR.PATCH format');
  }
  return tag.slice(1);
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

function filesIn(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`Unexpected symbolic link: ${path}`);
    return entry.isDirectory() ? filesIn(path) : [path];
  });
}

export function stage(root, target) {
  const extensions = installers[target];
  if (!extensions) throw new Error('Unknown build target');
  const files = extensions.flatMap(extension => {
    const directory = join(root, 'target', target, 'release', 'bundle', bundleDirectories[extension]);
    const matches = existsSync(directory) ? filesIn(directory).filter(path => path.endsWith(extension)) : [];
    if (matches.length === 0) throw new Error(`Missing ${target} installer: ${extension}`);
    return matches;
  });
  const destination = join(root, 'release-artifacts');
  mkdirSync(destination, { recursive: true });
  const names = new Set();
  for (const file of files) {
    const name = `${target}-${basename(file)}`;
    if (names.has(name)) throw new Error(`Duplicate installer: ${name}`);
    names.add(name);
    if (readFileSync(file).length === 0) throw new Error(`Empty installer: ${name}`);
    copyFileSync(file, join(destination, name));
  }
  return [...names].sort();
}

export function collect(root) {
  const directory = join(root, 'release-artifacts');
  const files = readdirSync(directory).filter(name => name !== 'SHA256SUMS').sort();
  for (const [target, extensions] of Object.entries(installers)) {
    for (const extension of extensions) {
      if (!files.some(name => name.startsWith(`${target}-`) && name.endsWith(extension))) {
        throw new Error(`Missing release installer: ${target} ${extension}`);
      }
    }
  }
  const checksums = files.map(name => {
    if (!Object.entries(installers).some(([target, extensions]) =>
      name.startsWith(`${target}-`) && extensions.some(extension => name.endsWith(extension)))) {
      throw new Error(`Unexpected release asset: ${name}`);
    }
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
      console.log(collect(root));
      break;
    default:
      throw new Error('Usage: node scripts/release.mjs <validate TAG [COMMIT]|check [TAG]|proto|stage TARGET|collect>');
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
