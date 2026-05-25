// metro.config.js — Expo + pnpm-workspace recipe per
// rubix/docs/scope/mobile/APP-SHELL.md §Metro:
//
//   1. watchFolders → repo root, so workspace packages reload on edit.
//   2. resolver.nodeModulesPaths → both rubix/mobile/node_modules and the
//      root node_modules, so symlinked workspace deps resolve.
//   3. resolver.disableHierarchicalLookup → true, to avoid resolving the
//      same React twice (a hard requirement with React 19 + new arch).

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

config.resolver.disableHierarchicalLookup = true;

module.exports = config;
