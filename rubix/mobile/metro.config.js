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
const { withNativeWind } = require('nativewind/metro');
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

// pnpm + React Native dedup. Several workspace packages pull in a newer
// react-native (0.85.x) as a peer-resolver, but this app pins 0.81.5
// (the version Expo SDK 54 ships). With `unstable_enableSymlinks` on,
// Metro will happily chase symlinks into the .pnpm store and load the
// wrong RN copy — codegen then crashes on new event types
// (e.g. VirtualViewNativeComponent `onModeChange`). Pin the canonical
// RN install (and its tightly-coupled siblings) to the project's
// node_modules so every resolver lands on the same realpath.
const pin = (name) =>
  path.resolve(projectRoot, 'node_modules', name);
config.resolver.extraNodeModules = {
  ...(config.resolver.extraNodeModules || {}),
  'react-native': pin('react-native'),
  react: pin('react'),
  'react-dom': pin('react-dom'),
  'expo': pin('expo'),
  'expo-router': pin('expo-router'),
  // Same story as react-native: pnpm hoists both worklets 0.5.1 (the
  // Expo-SDK-54-pinned version, which matches Expo Go's native side) and
  // 0.8.3 (pulled in by workspace deps that peer-resolve against the
  // newer react-native@0.85). 0.8.3's JS calls TurboModule methods with
  // signatures Expo Go's bundled native side rejects. Pin to the
  // project-local 0.5.1 install.
  'react-native-worklets': pin('react-native-worklets'),
  'react-native-reanimated': pin('react-native-reanimated'),
};

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
  // Hard-block any react-native copy in the pnpm store that ISN'T 0.81.x.
  // Some workspace packages declare a newer RN peer (0.85.x) and pnpm
  // hoists both — Metro then walks into the 0.85 internals and codegen
  // crashes on event types the pinned codegen (0.81.5) doesn't know.
  /\/node_modules\/\.pnpm\/react-native@(?!0\.81\.)[^/]+\/.*/,
  // Same defense for the worklets store entries: only 0.5.x matches Expo
  // Go's bundled native side; pnpm also stages 0.8.x copies (peered with
  // the rogue RN 0.85). Block them so Metro can never resolve them.
  /\/node_modules\/\.pnpm\/react-native-worklets@(?!0\.5\.)[^/]+\/.*/,
  // And the matching Reanimated copies. Expo SDK 54 pins 4.1.x; pnpm also
  // stages 4.3.x peered with worklets 0.8.x. Loading two Reanimateds at
  // once triggers `[Reanimated] Another instance of Reanimated was
  // detected. Previous: 4.1.7, current: 4.3.1.`
  /\/node_modules\/\.pnpm\/react-native-reanimated@(?!4\.1\.)[^/]+\/.*/,
  // Same defense for react itself. Workspace packages declare `react: ^19`
  // and `react: ^18 || ^19`; pnpm hoists both 19.2.6 and 18.3.1 into the
  // store alongside the 19.1.0 we pin. If Metro ever resolves through the
  // wrong realpath we get "Invalid hook call / more than one copy of React".
  /\/node_modules\/\.pnpm\/react@(?!19\.1\.)[^/]+\/.*/,
  /\/node_modules\/\.pnpm\/react-dom@(?!19\.1\.)[^/]+\/.*/,
];

module.exports = withNativeWind(config, { input: './global.css' });
