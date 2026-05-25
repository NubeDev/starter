// babel.config.js — required for expo-router + reanimated.
//
// `react-native-worklets/plugin` MUST be the LAST entry. Reanimated 4.x
// moved the worklet runtime out into a separate package; using the old
// `react-native-reanimated/plugin` against RN-Reanimated ≥4 leaves the
// worklets TurboModule uninstalled and any caller that touches it
// (moti, the new RN gesture-handler worklet bridges, …) crashes at
// runtime with `installTurboModule called with 1 arguments`.
//
// `babel-plugin-inline-import` inlines `import sql from './foo.sql'` as a
// string literal at babel transform time, so the migrations index can keep
// SQL in standalone `.sql` files (grep-able, side-by-side with the agent
// migrations folder) without Metro ever needing a `.sql` asset loader.

module.exports = function (api) {
  api.cache(true);
  return {
    presets: ['babel-preset-expo'],
    plugins: [
      ['babel-plugin-inline-import', { extensions: ['.sql'] }],
      'react-native-worklets/plugin',
    ],
  };
};
