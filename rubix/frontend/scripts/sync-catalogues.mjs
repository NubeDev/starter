#!/usr/bin/env node
// `pnpm --filter @nube/rubix-frontend sync-catalogues`
//
// Copies the `rubix.*` keys from `rubix/crates/rubix-spi/catalogues/{en,es}.json`
// into `rubix/frontend/src/i18n/{en,es}.json`. The SPI catalogues are
// the source of truth for runtime-emitted, agent-side i18n strings
// (skill outputs, system probes, error templates). The frontend
// shell catalogues own everything else (nav, buttons, settings).
//
// Idempotent: only `rubix.*` keys are touched; existing
// non-`rubix.*` keys are preserved verbatim. Run on demand; CI
// gates parity via `pnpm -w run check:i18n`.

import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const root = resolve(here, '..')
const spi = resolve(root, '../crates/rubix-spi/catalogues')
const dst = resolve(root, 'src/i18n')

const LOCALES = ['en', 'es']

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

function writeJson(path, obj) {
  // Stable key order: existing non-rubix keys first (in original
  // order), then rubix.* keys sorted alphabetically. Trailing newline
  // for git friendliness.
  writeFileSync(path, JSON.stringify(obj, null, 2) + '\n')
}

let changed = 0
for (const lang of LOCALES) {
  const source = readJson(resolve(spi, `${lang}.json`))
  const target = readJson(resolve(dst, `${lang}.json`))

  const keptNonRubix = Object.fromEntries(
    Object.entries(target).filter(([k]) => !k.startsWith('rubix.')),
  )
  const rubixKeys = Object.fromEntries(
    Object.entries(source)
      .filter(([k]) => k.startsWith('rubix.'))
      .sort(([a], [b]) => a.localeCompare(b)),
  )
  const merged = { ...keptNonRubix, ...rubixKeys }

  const before = JSON.stringify(target)
  const after = JSON.stringify(merged)
  if (before !== after) {
    writeJson(resolve(dst, `${lang}.json`), merged)
    changed += 1
    console.log(`updated src/i18n/${lang}.json (${Object.keys(rubixKeys).length} rubix.* keys)`)
  } else {
    console.log(`src/i18n/${lang}.json already in sync`)
  }
}

console.log(changed === 0 ? 'sync-catalogues: no changes' : `sync-catalogues: ${changed} file(s) updated`)
