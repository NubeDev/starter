// metro.config.js — Expo + pnpm-workspace recipe per
// rubix/docs/scope/mobile/APP-SHELL.md §Metro:
//
//   1. watchFolders → repo root, so workspace packages reload on edit.
//   2. resolver.nodeModulesPaths → both rubix/mobile/node_modules and the
//      root node_modules, so symlinked workspace deps resolve.
//   3. resolver.unstable_enableSymlinks → true (Expo SDK 54 default, set
//      explicitly). Metro follows pnpm's symlinks into the .pnpm store and
//      naturally dedupes React via the symlink target, which is the
//      modern, pnpm-safe replacement for the old
//      `disableHierarchicalLookup: true` recipe. That older flag broke
//      subpath imports like `expo-router/build/qualified-entry`, because
//      `entry-classic.js` lives inside the .pnpm sandbox and can't walk
//      back up to its own package without hierarchical lookup.

const { getDefaultConfig } = require('expo/metro-config');
const path = require('path');

const projectRoot = __dirname;
// rubix/mobile/ → rubix/ → repo root.
const workspaceRoot = path.resolve(projectRoot, '../..');

const config = getDefaultConfig(projectRoot);

config.watchFolders = [workspaceRoot];

config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, 'node_modules'),
  path.resolve(workspaceRoot, 'node_modules'),
];

config.resolver.unstable_enableSymlinks = true;
config.resolver.unstable_enablePackageExports = true;

// NodeNext/ESM `.js` extension rewrite for workspace TS packages.
// Several `@nube/starter-ui-*` packages set `"type": "module"` and ship
// TypeScript source with NodeNext-style relative imports that include
// the `.js` extension (e.g. `import "./render-page.js"` resolving to
// `render-page.tsx`). Metro with `unstable_enablePackageExports` takes
// the `.js` literally, so we intercept failed resolutions of relative
// `.js`/`.jsx` specifiers and retry against `.ts`/`.tsx`.
const upstreamResolveRequest = config.resolver.resolveRequest;
config.resolver.resolveRequest = (context, moduleName, platform) => {
  const tryResolve = (name) => {
    if (upstreamResolveRequest) {
      return upstreamResolveRequest(context, name, platform);
    }
    return context.resolveRequest(context, name, platform);
  };
  if (
    (moduleName.startsWith('./') || moduleName.startsWith('../')) &&
    /\.jsx?$/.test(moduleName)
  ) {
    try {
      return tryResolve(moduleName);
    } catch (err) {
      const tsName = moduleName.replace(/\.jsx$/, '.tsx').replace(/\.js$/, '.ts');
      try {
        return tryResolve(tsName);
      } catch {
        const tsxName = moduleName.replace(/\.jsx?$/, '.tsx');
        try {
          return tryResolve(tsxName);
        } catch {
          throw err;
        }
      }
    }
  }
  return tryResolve(moduleName);
};

// Exclude noisy / churn-heavy dirs that crash Metro's file watcher when files
// vanish mid-scan (Rust target/, pnpm install temp dirs, vcs metadata, run
// artifacts). NOTE: do NOT blanket-blocklist `/build/` or `/dist/` here —
// many npm packages (expo-router, react-native internals, …) publish their
// compiled JS under `build/` and the resolver will refuse those files too.
// Only Rust's repo-root `target/` and pnpm install temp dirs are real
// hazards on this workspace.
config.resolver.blockList = [
  /\/target\/.*/,
  /\/node_modules\/\.pnpm\/.*_tmp_.*/,
  /\/\.git\/.*/,
  /\/runs\/.*/,
];

module.exports = config;
