// babel.config.js — required for expo-router + reanimated.
//
// `react-native-reanimated/plugin` MUST be the LAST entry. APP-SHELL.md
// flags this may flip to `react-native-worklets/plugin` once the worklets
// migration lands in the pinned Reanimated; revisit on each SDK bump.

module.exports = function (api) {
  api.cache(true);
  return {
    presets: ['babel-preset-expo'],
    plugins: ['react-native-reanimated/plugin'],
  };
};
