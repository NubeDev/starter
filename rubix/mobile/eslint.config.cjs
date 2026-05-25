// ESLint flat-config. Enforces the path-level import lint from
// rubix/docs/scope/mobile/APP-SHELL.md §Import lint — the mobile app must
// NOT bundle web-only ui-kit / sdui-react / theme-editor surfaces. Named-
// export discipline (only `tokenStrategy` from `starter-ui-core/auth`) is
// CODEOWNERS-enforced for now per APP-SHELL.md Named-export caveat; this
// rule covers the path-level half of the matrix.

const RESTRICTED = [
  { name: '@nube/starter-ui-kit', message: 'web-only. use @nube/starter-ui-kit-native.' },
  { name: '@nube/starter-ui-flow', message: 'web-only flow editor. not in mobile scope.' },
  { name: '@nube/starter-ui-export', message: 'web-only.' },
  { name: '@nube/starter-ui-authz', message: 'web-only admin surface. not in mobile scope.' },
  { name: '@nube/starter-ui-ai-builder', message: 'web-only.' },
  { name: '@nube/starter-sdui-react', message: 'web-only.' },
  { name: '@nube/starter-ui-dashboard', message: 'web-only. use @nube/starter-ui-dashboard-native.' },
  { name: '@nube/starter-ui-sdui-react', message: "mobile must import the headless subpath: '@nube/starter-ui-sdui-react/headless'." },
  { name: '@nube/starter-ui-core/layout', message: 'web-only layout primitives.' },
  { name: '@nube/starter-ui-core/theme-editor/utils/apply-theme', message: 'web-only theme-editor utils.' },
  { name: '@nube/starter-ui-core/theme-editor/utils/apply-preferences', message: 'web-only theme-editor utils.' },
  { name: '@nube/starter-ui-core/theme-editor/utils/generate-css', message: 'web-only theme-editor utils.' },
  { name: '@nube/starter-ui-core/theme-editor/utils/tailwind-css', message: 'web-only theme-editor utils.' },
  { name: '@nube/starter-ui-core/theme-editor/utils/parse-css-input', message: 'web-only theme-editor utils.' },
  { name: '@nube/starter-ui-core/theme-editor/transport', message: 'web-only theme-editor transport.' },
];

module.exports = [
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: {
      parser: require('@typescript-eslint/parser'),
      parserOptions: { ecmaVersion: 2022, sourceType: 'module' },
    },
    rules: {
      'no-restricted-imports': ['error', { paths: RESTRICTED }],
    },
  },
  {
    ignores: ['node_modules', '.expo', 'dist', 'web-build'],
  },
];
