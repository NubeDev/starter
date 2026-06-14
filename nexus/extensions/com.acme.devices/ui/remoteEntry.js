(function () {
	'use strict';

	try{if(typeof document != 'undefined'){var elementStyle = document.createElement('style');elementStyle.appendChild(document.createTextNode("/*! tailwindcss v4.3.0 | MIT License | https://tailwindcss.com */\n@layer properties {\n  @supports (((-webkit-hyphens: none)) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color: rgb(from red r g b)))) {\n    *, [data-ext-id=\"com.acme.devices\"] :before, [data-ext-id=\"com.acme.devices\"]:before, [data-ext-id=\"com.acme.devices\"] :after, [data-ext-id=\"com.acme.devices\"]:after, [data-ext-id=\"com.acme.devices\"] ::backdrop, [data-ext-id=\"com.acme.devices\"]::backdrop {\n      --tw-border-style: solid;\n      --tw-leading: initial;\n      --tw-font-weight: initial;\n      --tw-tracking: initial;\n      --tw-ordinal: initial;\n      --tw-slashed-zero: initial;\n      --tw-numeric-figure: initial;\n      --tw-numeric-spacing: initial;\n      --tw-numeric-fraction: initial;\n      --tw-shadow: 0 0 #0000;\n      --tw-shadow-color: initial;\n      --tw-shadow-alpha: 100%;\n      --tw-inset-shadow: 0 0 #0000;\n      --tw-inset-shadow-color: initial;\n      --tw-inset-shadow-alpha: 100%;\n      --tw-ring-color: initial;\n      --tw-ring-shadow: 0 0 #0000;\n      --tw-inset-ring-color: initial;\n      --tw-inset-ring-shadow: 0 0 #0000;\n      --tw-ring-inset: initial;\n      --tw-ring-offset-width: 0px;\n      --tw-ring-offset-color: #fff;\n      --tw-ring-offset-shadow: 0 0 #0000;\n      --tw-outline-style: solid;\n      --tw-duration: initial;\n      --tw-ease: initial;\n    }\n  }\n}\n\n[data-ext-id=\"com.acme.devices\"] .relative, [data-ext-id=\"com.acme.devices\"].relative {\n  position: relative;\n}\n\n[data-ext-id=\"com.acme.devices\"] .mx-auto, [data-ext-id=\"com.acme.devices\"].mx-auto {\n  margin-inline: auto;\n}\n\n[data-ext-id=\"com.acme.devices\"] .mt-0\\.5, [data-ext-id=\"com.acme.devices\"].mt-0\\.5 {\n  margin-top: calc(var(--spacing, .25rem) * .5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .ml-auto, [data-ext-id=\"com.acme.devices\"].ml-auto {\n  margin-left: auto;\n}\n\n[data-ext-id=\"com.acme.devices\"] .flex, [data-ext-id=\"com.acme.devices\"].flex {\n  display: flex;\n}\n\n[data-ext-id=\"com.acme.devices\"] .grid, [data-ext-id=\"com.acme.devices\"].grid {\n  display: grid;\n}\n\n[data-ext-id=\"com.acme.devices\"] .inline-flex, [data-ext-id=\"com.acme.devices\"].inline-flex {\n  display: inline-flex;\n}\n\n[data-ext-id=\"com.acme.devices\"] .table, [data-ext-id=\"com.acme.devices\"].table {\n  display: table;\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-3\\.5, [data-ext-id=\"com.acme.devices\"].size-3\\.5 {\n  width: calc(var(--spacing, .25rem) * 3.5);\n  height: calc(var(--spacing, .25rem) * 3.5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-4, [data-ext-id=\"com.acme.devices\"].size-4 {\n  width: calc(var(--spacing, .25rem) * 4);\n  height: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-5, [data-ext-id=\"com.acme.devices\"].size-5 {\n  width: calc(var(--spacing, .25rem) * 5);\n  height: calc(var(--spacing, .25rem) * 5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-6, [data-ext-id=\"com.acme.devices\"].size-6 {\n  width: calc(var(--spacing, .25rem) * 6);\n  height: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-8, [data-ext-id=\"com.acme.devices\"].size-8 {\n  width: calc(var(--spacing, .25rem) * 8);\n  height: calc(var(--spacing, .25rem) * 8);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-9, [data-ext-id=\"com.acme.devices\"].size-9 {\n  width: calc(var(--spacing, .25rem) * 9);\n  height: calc(var(--spacing, .25rem) * 9);\n}\n\n[data-ext-id=\"com.acme.devices\"] .size-12, [data-ext-id=\"com.acme.devices\"].size-12 {\n  width: calc(var(--spacing, .25rem) * 12);\n  height: calc(var(--spacing, .25rem) * 12);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-1\\.5, [data-ext-id=\"com.acme.devices\"].h-1\\.5 {\n  height: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-2, [data-ext-id=\"com.acme.devices\"].h-2 {\n  height: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-8, [data-ext-id=\"com.acme.devices\"].h-8 {\n  height: calc(var(--spacing, .25rem) * 8);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-9, [data-ext-id=\"com.acme.devices\"].h-9 {\n  height: calc(var(--spacing, .25rem) * 9);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-10, [data-ext-id=\"com.acme.devices\"].h-10 {\n  height: calc(var(--spacing, .25rem) * 10);\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-full, [data-ext-id=\"com.acme.devices\"].h-full {\n  height: 100%;\n}\n\n[data-ext-id=\"com.acme.devices\"] .h-px, [data-ext-id=\"com.acme.devices\"].h-px {\n  height: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .w-1\\.5, [data-ext-id=\"com.acme.devices\"].w-1\\.5 {\n  width: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .w-4, [data-ext-id=\"com.acme.devices\"].w-4 {\n  width: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .w-fit, [data-ext-id=\"com.acme.devices\"].w-fit {\n  width: fit-content;\n}\n\n[data-ext-id=\"com.acme.devices\"] .w-full, [data-ext-id=\"com.acme.devices\"].w-full {\n  width: 100%;\n}\n\n[data-ext-id=\"com.acme.devices\"] .w-px, [data-ext-id=\"com.acme.devices\"].w-px {\n  width: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .max-w-3xl, [data-ext-id=\"com.acme.devices\"].max-w-3xl {\n  max-width: var(--container-3xl, 48rem);\n}\n\n[data-ext-id=\"com.acme.devices\"] .max-w-5xl, [data-ext-id=\"com.acme.devices\"].max-w-5xl {\n  max-width: var(--container-5xl, 64rem);\n}\n\n[data-ext-id=\"com.acme.devices\"] .max-w-md, [data-ext-id=\"com.acme.devices\"].max-w-md {\n  max-width: var(--container-md, 28rem);\n}\n\n[data-ext-id=\"com.acme.devices\"] .flex-1, [data-ext-id=\"com.acme.devices\"].flex-1 {\n  flex: 1;\n}\n\n[data-ext-id=\"com.acme.devices\"] .shrink-0, [data-ext-id=\"com.acme.devices\"].shrink-0 {\n  flex-shrink: 0;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-collapse, [data-ext-id=\"com.acme.devices\"].border-collapse {\n  border-collapse: collapse;\n}\n\n[data-ext-id=\"com.acme.devices\"] .animate-spin, [data-ext-id=\"com.acme.devices\"].animate-spin {\n  animation: var(--animate-spin, spin 1s linear infinite);\n}\n\n[data-ext-id=\"com.acme.devices\"] .grid-cols-1, [data-ext-id=\"com.acme.devices\"].grid-cols-1 {\n  grid-template-columns: repeat(1, minmax(0, 1fr));\n}\n\n[data-ext-id=\"com.acme.devices\"] .grid-cols-\\[6rem_1fr\\], [data-ext-id=\"com.acme.devices\"].grid-cols-\\[6rem_1fr\\] {\n  grid-template-columns: 6rem 1fr;\n}\n\n[data-ext-id=\"com.acme.devices\"] .grid-cols-\\[7rem_1fr\\], [data-ext-id=\"com.acme.devices\"].grid-cols-\\[7rem_1fr\\] {\n  grid-template-columns: 7rem 1fr;\n}\n\n[data-ext-id=\"com.acme.devices\"] .flex-col, [data-ext-id=\"com.acme.devices\"].flex-col {\n  flex-direction: column;\n}\n\n[data-ext-id=\"com.acme.devices\"] .flex-wrap, [data-ext-id=\"com.acme.devices\"].flex-wrap {\n  flex-wrap: wrap;\n}\n\n[data-ext-id=\"com.acme.devices\"] .place-items-center, [data-ext-id=\"com.acme.devices\"].place-items-center {\n  place-items: center;\n}\n\n[data-ext-id=\"com.acme.devices\"] .items-center, [data-ext-id=\"com.acme.devices\"].items-center {\n  align-items: center;\n}\n\n[data-ext-id=\"com.acme.devices\"] .items-start, [data-ext-id=\"com.acme.devices\"].items-start {\n  align-items: flex-start;\n}\n\n[data-ext-id=\"com.acme.devices\"] .justify-between, [data-ext-id=\"com.acme.devices\"].justify-between {\n  justify-content: space-between;\n}\n\n[data-ext-id=\"com.acme.devices\"] .justify-center, [data-ext-id=\"com.acme.devices\"].justify-center {\n  justify-content: center;\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-0\\.5, [data-ext-id=\"com.acme.devices\"].gap-0\\.5 {\n  gap: calc(var(--spacing, .25rem) * .5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-1, [data-ext-id=\"com.acme.devices\"].gap-1 {\n  gap: calc(var(--spacing, .25rem) * 1);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-1\\.5, [data-ext-id=\"com.acme.devices\"].gap-1\\.5 {\n  gap: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-2, [data-ext-id=\"com.acme.devices\"].gap-2 {\n  gap: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-3, [data-ext-id=\"com.acme.devices\"].gap-3 {\n  gap: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-4, [data-ext-id=\"com.acme.devices\"].gap-4 {\n  gap: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-5, [data-ext-id=\"com.acme.devices\"].gap-5 {\n  gap: calc(var(--spacing, .25rem) * 5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-6, [data-ext-id=\"com.acme.devices\"].gap-6 {\n  gap: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-x-3, [data-ext-id=\"com.acme.devices\"].gap-x-3 {\n  column-gap: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-x-4, [data-ext-id=\"com.acme.devices\"].gap-x-4 {\n  column-gap: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-y-1, [data-ext-id=\"com.acme.devices\"].gap-y-1 {\n  row-gap: calc(var(--spacing, .25rem) * 1);\n}\n\n[data-ext-id=\"com.acme.devices\"] .gap-y-1\\.5, [data-ext-id=\"com.acme.devices\"].gap-y-1\\.5 {\n  row-gap: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .truncate, [data-ext-id=\"com.acme.devices\"].truncate {\n  text-overflow: ellipsis;\n  white-space: nowrap;\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.acme.devices\"] .overflow-hidden, [data-ext-id=\"com.acme.devices\"].overflow-hidden {\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.acme.devices\"] .overflow-x-auto, [data-ext-id=\"com.acme.devices\"].overflow-x-auto {\n  overflow-x: auto;\n}\n\n[data-ext-id=\"com.acme.devices\"] .rounded, [data-ext-id=\"com.acme.devices\"].rounded {\n  border-radius: .25rem;\n}\n\n[data-ext-id=\"com.acme.devices\"] .rounded-full, [data-ext-id=\"com.acme.devices\"].rounded-full {\n  border-radius: 3.40282e38px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border, [data-ext-id=\"com.acme.devices\"].border {\n  border-style: var(--tw-border-style);\n  border-width: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-t, [data-ext-id=\"com.acme.devices\"].border-t {\n  border-top-style: var(--tw-border-style);\n  border-top-width: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-b, [data-ext-id=\"com.acme.devices\"].border-b {\n  border-bottom-style: var(--tw-border-style);\n  border-bottom-width: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-dashed, [data-ext-id=\"com.acme.devices\"].border-dashed {\n  --tw-border-style: dashed;\n  border-style: dashed;\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-emerald-600\\/30, [data-ext-id=\"com.acme.devices\"].border-emerald-600\\/30 {\n  border-color: #0097674d;\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.acme.devices\"] .border-emerald-600\\/30, [data-ext-id=\"com.acme.devices\"].border-emerald-600\\/30 {\n    border-color: color-mix(in oklab, var(--color-emerald-600, oklch(59.6% .145 163.225)) 30%, transparent);\n  }\n}\n\n[data-ext-id=\"com.acme.devices\"] .border-transparent, [data-ext-id=\"com.acme.devices\"].border-transparent {\n  border-color: #0000;\n}\n\n[data-ext-id=\"com.acme.devices\"] .bg-emerald-600\\/10, [data-ext-id=\"com.acme.devices\"].bg-emerald-600\\/10 {\n  background-color: #0097671a;\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.acme.devices\"] .bg-emerald-600\\/10, [data-ext-id=\"com.acme.devices\"].bg-emerald-600\\/10 {\n    background-color: color-mix(in oklab, var(--color-emerald-600, oklch(59.6% .145 163.225)) 10%, transparent);\n  }\n}\n\n[data-ext-id=\"com.acme.devices\"] .bg-transparent, [data-ext-id=\"com.acme.devices\"].bg-transparent {\n  background-color: #0000;\n}\n\n[data-ext-id=\"com.acme.devices\"] .p-1, [data-ext-id=\"com.acme.devices\"].p-1 {\n  padding: calc(var(--spacing, .25rem) * 1);\n}\n\n[data-ext-id=\"com.acme.devices\"] .p-2, [data-ext-id=\"com.acme.devices\"].p-2 {\n  padding: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .p-3, [data-ext-id=\"com.acme.devices\"].p-3 {\n  padding: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.acme.devices\"] .p-4, [data-ext-id=\"com.acme.devices\"].p-4 {\n  padding: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .px-1, [data-ext-id=\"com.acme.devices\"].px-1 {\n  padding-inline: calc(var(--spacing, .25rem) * 1);\n}\n\n[data-ext-id=\"com.acme.devices\"] .px-2, [data-ext-id=\"com.acme.devices\"].px-2 {\n  padding-inline: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .px-3, [data-ext-id=\"com.acme.devices\"].px-3 {\n  padding-inline: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.acme.devices\"] .px-4, [data-ext-id=\"com.acme.devices\"].px-4 {\n  padding-inline: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .px-6, [data-ext-id=\"com.acme.devices\"].px-6 {\n  padding-inline: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.acme.devices\"] .py-0\\.5, [data-ext-id=\"com.acme.devices\"].py-0\\.5 {\n  padding-block: calc(var(--spacing, .25rem) * .5);\n}\n\n[data-ext-id=\"com.acme.devices\"] .py-2, [data-ext-id=\"com.acme.devices\"].py-2 {\n  padding-block: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .py-6, [data-ext-id=\"com.acme.devices\"].py-6 {\n  padding-block: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.acme.devices\"] .py-8, [data-ext-id=\"com.acme.devices\"].py-8 {\n  padding-block: calc(var(--spacing, .25rem) * 8);\n}\n\n[data-ext-id=\"com.acme.devices\"] .pt-4, [data-ext-id=\"com.acme.devices\"].pt-4 {\n  padding-top: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .pt-6, [data-ext-id=\"com.acme.devices\"].pt-6 {\n  padding-top: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.acme.devices\"] .pr-4, [data-ext-id=\"com.acme.devices\"].pr-4 {\n  padding-right: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .pb-2, [data-ext-id=\"com.acme.devices\"].pb-2 {\n  padding-bottom: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-center, [data-ext-id=\"com.acme.devices\"].text-center {\n  text-align: center;\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-left, [data-ext-id=\"com.acme.devices\"].text-left {\n  text-align: left;\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-2xl, [data-ext-id=\"com.acme.devices\"].text-2xl {\n  font-size: var(--text-2xl, 1.5rem);\n  line-height: var(--tw-leading, var(--text-2xl--line-height, calc(2 / 1.5)));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-3xl, [data-ext-id=\"com.acme.devices\"].text-3xl {\n  font-size: var(--text-3xl, 1.875rem);\n  line-height: var(--tw-leading, var(--text-3xl--line-height, calc(2.25 / 1.875)));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-base, [data-ext-id=\"com.acme.devices\"].text-base {\n  font-size: var(--text-base, 1rem);\n  line-height: var(--tw-leading, var(--text-base--line-height, calc(1.5 / 1)));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-sm, [data-ext-id=\"com.acme.devices\"].text-sm {\n  font-size: var(--text-sm, .875rem);\n  line-height: var(--tw-leading, var(--text-sm--line-height, calc(1.25 / .875)));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-xs, [data-ext-id=\"com.acme.devices\"].text-xs {\n  font-size: var(--text-xs, .75rem);\n  line-height: var(--tw-leading, var(--text-xs--line-height, calc(1 / .75)));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-\\[11px\\], [data-ext-id=\"com.acme.devices\"].text-\\[11px\\] {\n  font-size: 11px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .leading-none, [data-ext-id=\"com.acme.devices\"].leading-none {\n  --tw-leading: 1;\n  line-height: 1;\n}\n\n[data-ext-id=\"com.acme.devices\"] .leading-relaxed, [data-ext-id=\"com.acme.devices\"].leading-relaxed {\n  --tw-leading: var(--leading-relaxed, 1.625);\n  line-height: var(--leading-relaxed, 1.625);\n}\n\n[data-ext-id=\"com.acme.devices\"] .leading-tight, [data-ext-id=\"com.acme.devices\"].leading-tight {\n  --tw-leading: var(--leading-tight, 1.25);\n  line-height: var(--leading-tight, 1.25);\n}\n\n[data-ext-id=\"com.acme.devices\"] .font-medium, [data-ext-id=\"com.acme.devices\"].font-medium {\n  --tw-font-weight: var(--font-weight-medium, 500);\n  font-weight: var(--font-weight-medium, 500);\n}\n\n[data-ext-id=\"com.acme.devices\"] .font-normal, [data-ext-id=\"com.acme.devices\"].font-normal {\n  --tw-font-weight: var(--font-weight-normal, 400);\n  font-weight: var(--font-weight-normal, 400);\n}\n\n[data-ext-id=\"com.acme.devices\"] .font-semibold, [data-ext-id=\"com.acme.devices\"].font-semibold {\n  --tw-font-weight: var(--font-weight-semibold, 600);\n  font-weight: var(--font-weight-semibold, 600);\n}\n\n[data-ext-id=\"com.acme.devices\"] .tracking-tight, [data-ext-id=\"com.acme.devices\"].tracking-tight {\n  --tw-tracking: var(--tracking-tight, -.025em);\n  letter-spacing: var(--tracking-tight, -.025em);\n}\n\n[data-ext-id=\"com.acme.devices\"] .whitespace-nowrap, [data-ext-id=\"com.acme.devices\"].whitespace-nowrap {\n  white-space: nowrap;\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-emerald-600, [data-ext-id=\"com.acme.devices\"].text-emerald-600 {\n  color: var(--color-emerald-600, oklch(59.6% .145 163.225));\n}\n\n[data-ext-id=\"com.acme.devices\"] .text-emerald-700, [data-ext-id=\"com.acme.devices\"].text-emerald-700 {\n  color: var(--color-emerald-700, oklch(50.8% .118 165.612));\n}\n\n[data-ext-id=\"com.acme.devices\"] .tabular-nums, [data-ext-id=\"com.acme.devices\"].tabular-nums {\n  --tw-numeric-spacing: tabular-nums;\n  font-variant-numeric: var(--tw-ordinal, ) var(--tw-slashed-zero, ) var(--tw-numeric-figure, ) var(--tw-numeric-spacing, ) var(--tw-numeric-fraction, );\n}\n\n[data-ext-id=\"com.acme.devices\"] .shadow-sm, [data-ext-id=\"com.acme.devices\"].shadow-sm {\n  --tw-shadow: 0 1px 3px 0 var(--tw-shadow-color, #0000001a), 0 1px 2px -1px var(--tw-shadow-color, #0000001a);\n  box-shadow: var(--tw-inset-shadow), var(--tw-inset-ring-shadow), var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow);\n}\n\n[data-ext-id=\"com.acme.devices\"] .outline, [data-ext-id=\"com.acme.devices\"].outline {\n  outline-style: var(--tw-outline-style);\n  outline-width: 1px;\n}\n\n[data-ext-id=\"com.acme.devices\"] .transition-\\[width\\], [data-ext-id=\"com.acme.devices\"].transition-\\[width\\] {\n  transition-property: width;\n  transition-timing-function: var(--tw-ease, var(--default-transition-timing-function, cubic-bezier(.4, 0, .2, 1)));\n  transition-duration: var(--tw-duration, var(--default-transition-duration, .15s));\n}\n\n[data-ext-id=\"com.acme.devices\"] .transition-all, [data-ext-id=\"com.acme.devices\"].transition-all {\n  transition-property: all;\n  transition-timing-function: var(--tw-ease, var(--default-transition-timing-function, cubic-bezier(.4, 0, .2, 1)));\n  transition-duration: var(--tw-duration, var(--default-transition-duration, .15s));\n}\n\n[data-ext-id=\"com.acme.devices\"] .transition-colors, [data-ext-id=\"com.acme.devices\"].transition-colors {\n  transition-property: color, background-color, border-color, outline-color, text-decoration-color, fill, stroke, --tw-gradient-from, --tw-gradient-via, --tw-gradient-to;\n  transition-timing-function: var(--tw-ease, var(--default-transition-timing-function, cubic-bezier(.4, 0, .2, 1)));\n  transition-duration: var(--tw-duration, var(--default-transition-duration, .15s));\n}\n\n[data-ext-id=\"com.acme.devices\"] .duration-500, [data-ext-id=\"com.acme.devices\"].duration-500 {\n  --tw-duration: .5s;\n  transition-duration: .5s;\n}\n\n[data-ext-id=\"com.acme.devices\"] .ease-out, [data-ext-id=\"com.acme.devices\"].ease-out {\n  --tw-ease: var(--ease-out, cubic-bezier(0, 0, .2, 1));\n  transition-timing-function: var(--ease-out, cubic-bezier(0, 0, .2, 1));\n}\n\n[data-ext-id=\"com.acme.devices\"] .outline-none, [data-ext-id=\"com.acme.devices\"].outline-none {\n  --tw-outline-style: none;\n  outline-style: none;\n}\n\n[data-ext-id=\"com.acme.devices\"] .last\\:border-0:last-child, [data-ext-id=\"com.acme.devices\"].last\\:border-0:last-child {\n  border-style: var(--tw-border-style);\n  border-width: 0;\n}\n\n[data-ext-id=\"com.acme.devices\"] .focus-visible\\:ring-2:focus-visible, [data-ext-id=\"com.acme.devices\"].focus-visible\\:ring-2:focus-visible {\n  --tw-ring-shadow: var(--tw-ring-inset, ) 0 0 0 calc(2px + var(--tw-ring-offset-width)) var(--tw-ring-color, currentcolor);\n  box-shadow: var(--tw-inset-shadow), var(--tw-inset-ring-shadow), var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow);\n}\n\n[data-ext-id=\"com.acme.devices\"] .disabled\\:pointer-events-none:disabled, [data-ext-id=\"com.acme.devices\"].disabled\\:pointer-events-none:disabled {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.acme.devices\"] .disabled\\:opacity-50:disabled, [data-ext-id=\"com.acme.devices\"].disabled\\:opacity-50:disabled {\n  opacity: .5;\n}\n\n[data-ext-id=\"com.acme.devices\"] .disabled\\:opacity-60:disabled, [data-ext-id=\"com.acme.devices\"].disabled\\:opacity-60:disabled {\n  opacity: .6;\n}\n\n@media (min-width: 40rem) {\n  [data-ext-id=\"com.acme.devices\"] .sm\\:grid-cols-3, [data-ext-id=\"com.acme.devices\"].sm\\:grid-cols-3 {\n    grid-template-columns: repeat(3, minmax(0, 1fr));\n  }\n}\n\n@media (prefers-color-scheme: dark) {\n  [data-ext-id=\"com.acme.devices\"] .dark\\:text-emerald-400, [data-ext-id=\"com.acme.devices\"].dark\\:text-emerald-400 {\n    color: var(--color-emerald-400, oklch(76.5% .177 163.223));\n  }\n}\n\n[data-ext-id=\"com.acme.devices\"] .\\[\\&_svg\\]\\:pointer-events-none svg, [data-ext-id=\"com.acme.devices\"].\\[\\&_svg\\]\\:pointer-events-none svg {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.acme.devices\"] .\\[\\&_svg\\]\\:size-3 svg, [data-ext-id=\"com.acme.devices\"].\\[\\&_svg\\]\\:size-3 svg {\n  width: calc(var(--spacing, .25rem) * 3);\n  height: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.acme.devices\"] .\\[\\&_svg\\]\\:size-4 svg, [data-ext-id=\"com.acme.devices\"].\\[\\&_svg\\]\\:size-4 svg {\n  width: calc(var(--spacing, .25rem) * 4);\n  height: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.acme.devices\"] .\\[\\&_svg\\]\\:shrink-0 svg, [data-ext-id=\"com.acme.devices\"].\\[\\&_svg\\]\\:shrink-0 svg {\n  flex-shrink: 0;\n}\n\n@property --tw-border-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n\n@property --tw-leading {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-font-weight {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-tracking {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ordinal {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-slashed-zero {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-figure {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-spacing {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-numeric-fraction {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-shadow-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-shadow-alpha {\n  syntax: \"<percentage>\";\n  inherits: false;\n  initial-value: 100%;\n}\n\n@property --tw-inset-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-inset-shadow-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-inset-shadow-alpha {\n  syntax: \"<percentage>\";\n  inherits: false;\n  initial-value: 100%;\n}\n\n@property --tw-ring-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ring-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-inset-ring-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-inset-ring-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-ring-inset {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ring-offset-width {\n  syntax: \"<length>\";\n  inherits: false;\n  initial-value: 0;\n}\n\n@property --tw-ring-offset-color {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: #fff;\n}\n\n@property --tw-ring-offset-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-outline-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n\n@property --tw-duration {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ease {\n  syntax: \"*\";\n  inherits: false\n}\n\n@keyframes spin {\n  to {\n    transform: rotate(360deg);\n  }\n}"));document.head.appendChild(elementStyle);}}catch(e){console.error('vite-plugin-css-injected-by-js', e);}

})();
import { jsx, jsxs, Fragment } from 'react/jsx-runtime';
import * as React from 'react';
import { forwardRef, createElement } from 'react';

const HOST_CLIENT_CTX_KEY = "__starterExtSdkHostClientContextV1";
const HostClientContext = globalThis[HOST_CLIENT_CTX_KEY] ?? (globalThis[HOST_CLIENT_CTX_KEY] = React.createContext(null));
function useHostClient() {
  const client = React.useContext(HostClientContext);
  if (!client) {
    throw new Error(
      "useHostClient() called outside <ExtensionHostClientProvider>. The host shell must wrap extension slots in ExtensionHostProvider."
    );
  }
  return client;
}

const SLOT_CTX_KEY = "__starterExtSdkSlotContextV2";
const Context = globalThis[SLOT_CTX_KEY] ?? (globalThis[SLOT_CTX_KEY] = React.createContext(null));
function useSlotContext() {
  const ctx = React.useContext(Context);
  if (!ctx) {
    throw new Error(
      "useSlotContext() called outside <SlotContextProvider>. The host's federation runtime must wrap exposed components in SlotContextProvider."
    );
  }
  return ctx;
}

const DEFAULT_BLOCK_SHELL_MESSAGES = {
  loading: "Loading…",
  errorTitle: "Extension failed:"
};
function mergeBlockShellMessages(override) {
  return override ? { ...DEFAULT_BLOCK_SHELL_MESSAGES, ...override } : DEFAULT_BLOCK_SHELL_MESSAGES;
}
function BlockShell(props) {
  const slot = useSlotContext();
  const messages = React.useMemo(
    () => mergeBlockShellMessages(props.messages),
    [props.messages]
  );
  return /* @__PURE__ */ jsx(
    "div",
    {
      className: props.className ? `starter-ext-block ${props.className}` : "starter-ext-block",
      "data-ext-id": slot.extensionId,
      "data-ext-slot": slot.slotId,
      children: /* @__PURE__ */ jsx(
        ExtensionErrorBoundary,
        {
          extensionId: slot.extensionId,
          fallback: props.errorFallback,
          errorTitle: messages.errorTitle,
          children: /* @__PURE__ */ jsx(
            React.Suspense,
            {
              fallback: props.loading ?? /* @__PURE__ */ jsx(DefaultLoading, { slotId: slot.slotId, label: messages.loading }),
              children: props.children
            }
          )
        }
      )
    }
  );
}
class ExtensionErrorBoundary extends React.Component {
  state = { error: null };
  static getDerivedStateFromError(error) {
    return { error };
  }
  componentDidCatch(error, info) {
    console.error(
      `[starter-ext] extension ${this.props.extensionId} crashed in render:`,
      error,
      info
    );
  }
  render() {
    if (this.state.error !== null) {
      const fb = this.props.fallback;
      if (fb) {
        return fb(this.state.error, this.props.extensionId);
      }
      return defaultErrorFallback(
        this.state.error,
        this.props.extensionId,
        this.props.errorTitle
      );
    }
    return this.props.children;
  }
}
function defaultErrorFallback(err, extensionId, title) {
  const msg = err instanceof Error ? err.message : String(err);
  return /* @__PURE__ */ jsxs("div", { role: "alert", className: "starter-ext-block__error", children: [
    /* @__PURE__ */ jsx("strong", { children: title }),
    " ",
    extensionId,
    /* @__PURE__ */ jsx("div", { children: msg })
  ] });
}
function DefaultLoading(props) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "aria-busy": "true",
      "aria-live": "polite",
      className: "starter-ext-block__loading",
      "data-slot": props.slotId,
      children: props.label
    }
  );
}

const HOST_BINDINGS_CTX_KEY = "__starterExtSdkHostBindingsContextV1";
const HostBindingsContext = globalThis[HOST_BINDINGS_CTX_KEY] ?? (globalThis[HOST_BINDINGS_CTX_KEY] = React.createContext(null));
function HostBindingsProvider(props) {
  return /* @__PURE__ */ jsx(HostBindingsContext.Provider, { value: props.bindings, children: props.children });
}

function registerExtensionContributions(handle, contributions) {
  const bindings = { extensionId: handle.id, singletons: handle.singletons };
  const wrapped = {};
  for (const [name, Component] of Object.entries(contributions.components)) {
    wrapped[name] = wrapWithBindings(name, Component, bindings);
  }
  handle.register({ components: wrapped });
}
function wrapWithBindings(displayName, Component, bindings) {
  const Wrapped = (props) => /* @__PURE__ */ jsx(HostBindingsProvider, { bindings, children: /* @__PURE__ */ jsx(Component, { ...props }) });
  Wrapped.displayName = `HostBindings(${bindings.extensionId}:${displayName})`;
  return Wrapped;
}

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */

const toKebabCase = (string) => string.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
const mergeClasses = (...classes) => classes.filter((className, index, array) => {
  return Boolean(className) && className.trim() !== "" && array.indexOf(className) === index;
}).join(" ").trim();

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */

var defaultAttributes = {
  xmlns: "http://www.w3.org/2000/svg",
  width: 24,
  height: 24,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round",
  strokeLinejoin: "round"
};

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Icon = forwardRef(
  ({
    color = "currentColor",
    size = 24,
    strokeWidth = 2,
    absoluteStrokeWidth,
    className = "",
    children,
    iconNode,
    ...rest
  }, ref) => {
    return createElement(
      "svg",
      {
        ref,
        ...defaultAttributes,
        width: size,
        height: size,
        stroke: color,
        strokeWidth: absoluteStrokeWidth ? Number(strokeWidth) * 24 / Number(size) : strokeWidth,
        className: mergeClasses("lucide", className),
        ...rest
      },
      [
        ...iconNode.map(([tag, attrs]) => createElement(tag, attrs)),
        ...Array.isArray(children) ? children : [children]
      ]
    );
  }
);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const createLucideIcon = (iconName, iconNode) => {
  const Component = forwardRef(
    ({ className, ...props }, ref) => createElement(Icon, {
      ref,
      iconNode,
      className: mergeClasses(`lucide-${toKebabCase(iconName)}`, className),
      ...props
    })
  );
  Component.displayName = `${iconName}`;
  return Component;
};

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ArrowRight = createLucideIcon("ArrowRight", [
  ["path", { d: "M5 12h14", key: "1ays0h" }],
  ["path", { d: "m12 5 7 7-7 7", key: "xquz4c" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const BadgeCheck = createLucideIcon("BadgeCheck", [
  [
    "path",
    {
      d: "M3.85 8.62a4 4 0 0 1 4.78-4.77 4 4 0 0 1 6.74 0 4 4 0 0 1 4.78 4.78 4 4 0 0 1 0 6.74 4 4 0 0 1-4.77 4.78 4 4 0 0 1-6.75 0 4 4 0 0 1-4.78-4.77 4 4 0 0 1 0-6.76Z",
      key: "3c2336"
    }
  ],
  ["path", { d: "m9 12 2 2 4-4", key: "dzmm74" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Ban = createLucideIcon("Ban", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m4.9 4.9 14.2 14.2", key: "1m5liu" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Box = createLucideIcon("Box", [
  [
    "path",
    {
      d: "M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z",
      key: "hh9hay"
    }
  ],
  ["path", { d: "m3.3 7 8.7 5 8.7-5", key: "g66t2b" }],
  ["path", { d: "M12 22V12", key: "d0xqtd" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Boxes = createLucideIcon("Boxes", [
  [
    "path",
    {
      d: "M2.97 12.92A2 2 0 0 0 2 14.63v3.24a2 2 0 0 0 .97 1.71l3 1.8a2 2 0 0 0 2.06 0L12 19v-5.5l-5-3-4.03 2.42Z",
      key: "lc1i9w"
    }
  ],
  ["path", { d: "m7 16.5-4.74-2.85", key: "1o9zyk" }],
  ["path", { d: "m7 16.5 5-3", key: "va8pkn" }],
  ["path", { d: "M7 16.5v5.17", key: "jnp8gn" }],
  [
    "path",
    {
      d: "M12 13.5V19l3.97 2.38a2 2 0 0 0 2.06 0l3-1.8a2 2 0 0 0 .97-1.71v-3.24a2 2 0 0 0-.97-1.71L17 10.5l-5 3Z",
      key: "8zsnat"
    }
  ],
  ["path", { d: "m17 16.5-5-3", key: "8arw3v" }],
  ["path", { d: "m17 16.5 4.74-2.85", key: "8rfmw" }],
  ["path", { d: "M17 16.5v5.17", key: "k6z78m" }],
  [
    "path",
    {
      d: "M7.97 4.42A2 2 0 0 0 7 6.13v4.37l5 3 5-3V6.13a2 2 0 0 0-.97-1.71l-3-1.8a2 2 0 0 0-2.06 0l-3 1.8Z",
      key: "1xygjf"
    }
  ],
  ["path", { d: "M12 8 7.26 5.15", key: "1vbdud" }],
  ["path", { d: "m12 8 4.74-2.85", key: "3rx089" }],
  ["path", { d: "M12 13.5V8", key: "1io7kd" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ChevronRight = createLucideIcon("ChevronRight", [
  ["path", { d: "m9 18 6-6-6-6", key: "mthhwq" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const CircleCheck = createLucideIcon("CircleCheck", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m9 12 2 2 4-4", key: "dzmm74" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const CircleX = createLucideIcon("CircleX", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "m15 9-6 6", key: "1uzhvr" }],
  ["path", { d: "m9 9 6 6", key: "z0biqf" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Circle = createLucideIcon("Circle", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Cpu = createLucideIcon("Cpu", [
  ["rect", { width: "16", height: "16", x: "4", y: "4", rx: "2", key: "14l7u7" }],
  ["rect", { width: "6", height: "6", x: "9", y: "9", rx: "1", key: "5aljv4" }],
  ["path", { d: "M15 2v2", key: "13l42r" }],
  ["path", { d: "M15 20v2", key: "15mkzm" }],
  ["path", { d: "M2 15h2", key: "1gxd5l" }],
  ["path", { d: "M2 9h2", key: "1bbxkp" }],
  ["path", { d: "M20 15h2", key: "19e6y8" }],
  ["path", { d: "M20 9h2", key: "19tzq7" }],
  ["path", { d: "M9 2v2", key: "165o2o" }],
  ["path", { d: "M9 20v2", key: "i2bqo8" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Dice5 = createLucideIcon("Dice5", [
  ["rect", { width: "18", height: "18", x: "3", y: "3", rx: "2", ry: "2", key: "1m3agn" }],
  ["path", { d: "M16 8h.01", key: "cr5u4v" }],
  ["path", { d: "M8 8h.01", key: "1e4136" }],
  ["path", { d: "M8 16h.01", key: "18s6g9" }],
  ["path", { d: "M16 16h.01", key: "1f9h7w" }],
  ["path", { d: "M12 12h.01", key: "1mp3jc" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Info = createLucideIcon("Info", [
  ["circle", { cx: "12", cy: "12", r: "10", key: "1mglay" }],
  ["path", { d: "M12 16v-4", key: "1dtifu" }],
  ["path", { d: "M12 8h.01", key: "e9boi3" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const KeyRound = createLucideIcon("KeyRound", [
  [
    "path",
    {
      d: "M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z",
      key: "1s6t7t"
    }
  ],
  ["circle", { cx: "16.5", cy: "7.5", r: ".5", fill: "currentColor", key: "w0ekpg" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const LayoutDashboard = createLucideIcon("LayoutDashboard", [
  ["rect", { width: "7", height: "9", x: "3", y: "3", rx: "1", key: "10lvy0" }],
  ["rect", { width: "7", height: "5", x: "14", y: "3", rx: "1", key: "16une8" }],
  ["rect", { width: "7", height: "9", x: "14", y: "12", rx: "1", key: "1hutg5" }],
  ["rect", { width: "7", height: "5", x: "3", y: "16", rx: "1", key: "ldoo1y" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const LoaderCircle = createLucideIcon("LoaderCircle", [
  ["path", { d: "M21 12a9 9 0 1 1-6.219-8.56", key: "13zald" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const MapPin = createLucideIcon("MapPin", [
  [
    "path",
    {
      d: "M20 10c0 4.993-5.539 10.193-7.399 11.799a1 1 0 0 1-1.202 0C9.539 20.193 4 14.993 4 10a8 8 0 0 1 16 0",
      key: "1r0f0z"
    }
  ],
  ["circle", { cx: "12", cy: "10", r: "3", key: "ilqhr7" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const PartyPopper = createLucideIcon("PartyPopper", [
  ["path", { d: "M5.8 11.3 2 22l10.7-3.79", key: "gwxi1d" }],
  ["path", { d: "M4 3h.01", key: "1vcuye" }],
  ["path", { d: "M22 8h.01", key: "1mrtc2" }],
  ["path", { d: "M15 2h.01", key: "1cjtqr" }],
  ["path", { d: "M22 20h.01", key: "1mrys2" }],
  [
    "path",
    {
      d: "m22 2-2.24.75a2.9 2.9 0 0 0-1.96 3.12c.1.86-.57 1.63-1.45 1.63h-.38c-.86 0-1.6.6-1.76 1.44L14 10",
      key: "hbicv8"
    }
  ],
  [
    "path",
    { d: "m22 13-.82-.33c-.86-.34-1.82.2-1.98 1.11c-.11.7-.72 1.22-1.43 1.22H17", key: "1i94pl" }
  ],
  ["path", { d: "m11 2 .33.82c.34.86-.2 1.82-1.11 1.98C9.52 4.9 9 5.52 9 6.23V7", key: "1cofks" }],
  [
    "path",
    {
      d: "M11 13c1.93 1.93 2.83 4.17 2 5-.83.83-3.07-.07-5-2-1.93-1.93-2.83-4.17-2-5 .83-.83 3.07.07 5 2Z",
      key: "4kbmks"
    }
  ]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Radio = createLucideIcon("Radio", [
  ["path", { d: "M4.9 19.1C1 15.2 1 8.8 4.9 4.9", key: "1vaf9d" }],
  ["path", { d: "M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5", key: "u1ii0m" }],
  ["circle", { cx: "12", cy: "12", r: "2", key: "1c9p78" }],
  ["path", { d: "M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5", key: "1j5fej" }],
  ["path", { d: "M19.1 4.9C23 8.8 23 15.1 19.1 19", key: "10b0cb" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const RefreshCw = createLucideIcon("RefreshCw", [
  ["path", { d: "M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8", key: "v9h5vc" }],
  ["path", { d: "M21 3v5h-5", key: "1q7to0" }],
  ["path", { d: "M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16", key: "3uifl3" }],
  ["path", { d: "M8 16H3v5", key: "1cv678" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const RotateCcw = createLucideIcon("RotateCcw", [
  ["path", { d: "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8", key: "1357e3" }],
  ["path", { d: "M3 3v5h5", key: "1xhq8a" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ScanLine = createLucideIcon("ScanLine", [
  ["path", { d: "M3 7V5a2 2 0 0 1 2-2h2", key: "aa7l1z" }],
  ["path", { d: "M17 3h2a2 2 0 0 1 2 2v2", key: "4qcy5o" }],
  ["path", { d: "M21 17v2a2 2 0 0 1-2 2h-2", key: "6vwrx8" }],
  ["path", { d: "M7 21H5a2 2 0 0 1-2-2v-2", key: "ioqczr" }],
  ["path", { d: "M7 12h10", key: "b7w52i" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ShieldCheck = createLucideIcon("ShieldCheck", [
  [
    "path",
    {
      d: "M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z",
      key: "oel41y"
    }
  ],
  ["path", { d: "m9 12 2 2 4-4", key: "dzmm74" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Sparkles = createLucideIcon("Sparkles", [
  [
    "path",
    {
      d: "M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z",
      key: "4pj2yx"
    }
  ],
  ["path", { d: "M20 3v4", key: "1olli1" }],
  ["path", { d: "M22 5h-4", key: "1gvqau" }],
  ["path", { d: "M4 17v2", key: "vumght" }],
  ["path", { d: "M5 18H3", key: "zchphs" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Table2 = createLucideIcon("Table2", [
  [
    "path",
    {
      d: "M9 3H5a2 2 0 0 0-2 2v4m6-6h10a2 2 0 0 1 2 2v4M9 3v18m0 0h10a2 2 0 0 0 2-2V9M9 21H5a2 2 0 0 1-2-2V9m0 0h18",
      key: "gugj83"
    }
  ]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Users = createLucideIcon("Users", [
  ["path", { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2", key: "1yyitq" }],
  ["circle", { cx: "9", cy: "7", r: "4", key: "nufk8" }],
  ["path", { d: "M22 21v-2a4 4 0 0 0-3-3.87", key: "kshegd" }],
  ["path", { d: "M16 3.13a4 4 0 0 1 0 7.75", key: "1da9ce" }]
]);

class StarterError extends Error {
  status;
  problem;
  /**
   * Machine-readable error code. Set by client-side factories
   * (`invalidResponse` etc.) for cases the server cannot tag
   * itself. Server-driven errors carry their tag in `problem.type`.
   */
  code;
  constructor(status, message, problem, code) {
    super(message);
    this.name = "StarterError";
    this.status = status;
    this.problem = problem;
    this.code = code;
  }
  static async fromResponse(res) {
    let problem;
    try {
      const body = await res.clone().json();
      if (body && typeof body === "object" && "type" in body && "title" in body) {
        problem = body;
      }
    } catch {
    }
    let msg = problem?.title;
    if (!msg) {
      try {
        const text = (await res.clone().text()).trim();
        if (text) msg = text;
      } catch {
      }
    }
    return new StarterError(res.status, msg ?? `HTTP ${res.status}`, problem);
  }
  /**
   * Build an error for a 2xx response whose body is not JSON.
   * Typical cause: a dev-server SPA fallback returned `index.html`
   * instead of forwarding the request to the API — meaning the
   * client is asking a path the proxy does not cover. Surfaced as
   * `status = 502` + `code = "invalid-response-content-type"` so
   * callers (notably `AuthProvider`) can distinguish it from a
   * genuine server error.
   */
  static invalidResponse(url, contentType) {
    const ct = contentType ?? "<none>";
    return new StarterError(
      502,
      `Expected JSON from ${url} but got content-type ${ct}. This usually means the request was not routed to the API (e.g. the dev-server proxy is missing this path).`,
      void 0,
      "invalid-response-content-type"
    );
  }
  // Type guard. With one arg, narrows to StarterError; with two, also
  // requires that `.status` matches.
  static is(err, status) {
    if (!(err instanceof StarterError)) return false;
    return status === void 0 || err.status === status;
  }
}

function readCookie(name) {
  if (typeof document === "undefined") return void 0;
  for (const part of document.cookie.split(";")) {
    const [k, v] = part.trim().split("=");
    if (k === name) return v;
  }
  return void 0;
}
function readCsrfHeader(cookieName = "starter_csrf") {
  const csrf = readCookie(cookieName);
  return csrf ? { "X-CSRF-Token": csrf } : {};
}
const MUTATING = /* @__PURE__ */ new Set(["POST", "PUT", "PATCH", "DELETE"]);
function csrfHeaderForMethod(method, cookieName = "starter_csrf") {
  const m = (method ?? "GET").toUpperCase();
  return MUTATING.has(m) ? readCsrfHeader(cookieName) : {};
}

function isJsonContentType(value) {
  if (!value) return false;
  const semi = value.indexOf(";");
  const main = (semi === -1 ? value : value.slice(0, semi)).trim().toLowerCase();
  return main === "application/json" || main === "application/problem+json" || main.endsWith("+json");
}
async function fetchJson(client, path, init = {}) {
  const headers = {
    ...client.headers,
    ...csrfHeaderForMethod(init.method),
    ...init.headers
  };
  const url = `${client.baseUrl}${path}`;
  const res = await client.fetch(url, {
    ...init,
    credentials: "include",
    headers
  });
  if (!res.ok) throw await StarterError.fromResponse(res);
  if (!isJsonContentType(res.headers.get("content-type"))) {
    throw StarterError.invalidResponse(url, res.headers.get("content-type"));
  }
  return await res.json();
}

function r(e){var t,f,n="";if("string"==typeof e||"number"==typeof e)n+=e;else if("object"==typeof e)if(Array.isArray(e)){var o=e.length;for(t=0;t<o;t++)e[t]&&(f=r(e[t]))&&(n&&(n+=" "),n+=f);}else for(f in e)e[f]&&(n&&(n+=" "),n+=f);return n}function clsx(){for(var e,t,f=0,n="",o=arguments.length;f<o;f++)(e=arguments[f])&&(t=r(e))&&(n&&(n+=" "),n+=t);return n}

const CLASS_PART_SEPARATOR = '-';
const createClassGroupUtils = config => {
  const classMap = createClassMap(config);
  const {
    conflictingClassGroups,
    conflictingClassGroupModifiers
  } = config;
  const getClassGroupId = className => {
    const classParts = className.split(CLASS_PART_SEPARATOR);
    // Classes like `-inset-1` produce an empty string as first classPart. We assume that classes for negative values are used correctly and remove it from classParts.
    if (classParts[0] === '' && classParts.length !== 1) {
      classParts.shift();
    }
    return getGroupRecursive(classParts, classMap) || getGroupIdForArbitraryProperty(className);
  };
  const getConflictingClassGroupIds = (classGroupId, hasPostfixModifier) => {
    const conflicts = conflictingClassGroups[classGroupId] || [];
    if (hasPostfixModifier && conflictingClassGroupModifiers[classGroupId]) {
      return [...conflicts, ...conflictingClassGroupModifiers[classGroupId]];
    }
    return conflicts;
  };
  return {
    getClassGroupId,
    getConflictingClassGroupIds
  };
};
const getGroupRecursive = (classParts, classPartObject) => {
  if (classParts.length === 0) {
    return classPartObject.classGroupId;
  }
  const currentClassPart = classParts[0];
  const nextClassPartObject = classPartObject.nextPart.get(currentClassPart);
  const classGroupFromNextClassPart = nextClassPartObject ? getGroupRecursive(classParts.slice(1), nextClassPartObject) : undefined;
  if (classGroupFromNextClassPart) {
    return classGroupFromNextClassPart;
  }
  if (classPartObject.validators.length === 0) {
    return undefined;
  }
  const classRest = classParts.join(CLASS_PART_SEPARATOR);
  return classPartObject.validators.find(({
    validator
  }) => validator(classRest))?.classGroupId;
};
const arbitraryPropertyRegex = /^\[(.+)\]$/;
const getGroupIdForArbitraryProperty = className => {
  if (arbitraryPropertyRegex.test(className)) {
    const arbitraryPropertyClassName = arbitraryPropertyRegex.exec(className)[1];
    const property = arbitraryPropertyClassName?.substring(0, arbitraryPropertyClassName.indexOf(':'));
    if (property) {
      // I use two dots here because one dot is used as prefix for class groups in plugins
      return 'arbitrary..' + property;
    }
  }
};
/**
 * Exported for testing only
 */
const createClassMap = config => {
  const {
    theme,
    prefix
  } = config;
  const classMap = {
    nextPart: new Map(),
    validators: []
  };
  const prefixedClassGroupEntries = getPrefixedClassGroupEntries(Object.entries(config.classGroups), prefix);
  prefixedClassGroupEntries.forEach(([classGroupId, classGroup]) => {
    processClassesRecursively(classGroup, classMap, classGroupId, theme);
  });
  return classMap;
};
const processClassesRecursively = (classGroup, classPartObject, classGroupId, theme) => {
  classGroup.forEach(classDefinition => {
    if (typeof classDefinition === 'string') {
      const classPartObjectToEdit = classDefinition === '' ? classPartObject : getPart(classPartObject, classDefinition);
      classPartObjectToEdit.classGroupId = classGroupId;
      return;
    }
    if (typeof classDefinition === 'function') {
      if (isThemeGetter(classDefinition)) {
        processClassesRecursively(classDefinition(theme), classPartObject, classGroupId, theme);
        return;
      }
      classPartObject.validators.push({
        validator: classDefinition,
        classGroupId
      });
      return;
    }
    Object.entries(classDefinition).forEach(([key, classGroup]) => {
      processClassesRecursively(classGroup, getPart(classPartObject, key), classGroupId, theme);
    });
  });
};
const getPart = (classPartObject, path) => {
  let currentClassPartObject = classPartObject;
  path.split(CLASS_PART_SEPARATOR).forEach(pathPart => {
    if (!currentClassPartObject.nextPart.has(pathPart)) {
      currentClassPartObject.nextPart.set(pathPart, {
        nextPart: new Map(),
        validators: []
      });
    }
    currentClassPartObject = currentClassPartObject.nextPart.get(pathPart);
  });
  return currentClassPartObject;
};
const isThemeGetter = func => func.isThemeGetter;
const getPrefixedClassGroupEntries = (classGroupEntries, prefix) => {
  if (!prefix) {
    return classGroupEntries;
  }
  return classGroupEntries.map(([classGroupId, classGroup]) => {
    const prefixedClassGroup = classGroup.map(classDefinition => {
      if (typeof classDefinition === 'string') {
        return prefix + classDefinition;
      }
      if (typeof classDefinition === 'object') {
        return Object.fromEntries(Object.entries(classDefinition).map(([key, value]) => [prefix + key, value]));
      }
      return classDefinition;
    });
    return [classGroupId, prefixedClassGroup];
  });
};

// LRU cache inspired from hashlru (https://github.com/dominictarr/hashlru/blob/v1.0.4/index.js) but object replaced with Map to improve performance
const createLruCache = maxCacheSize => {
  if (maxCacheSize < 1) {
    return {
      get: () => undefined,
      set: () => {}
    };
  }
  let cacheSize = 0;
  let cache = new Map();
  let previousCache = new Map();
  const update = (key, value) => {
    cache.set(key, value);
    cacheSize++;
    if (cacheSize > maxCacheSize) {
      cacheSize = 0;
      previousCache = cache;
      cache = new Map();
    }
  };
  return {
    get(key) {
      let value = cache.get(key);
      if (value !== undefined) {
        return value;
      }
      if ((value = previousCache.get(key)) !== undefined) {
        update(key, value);
        return value;
      }
    },
    set(key, value) {
      if (cache.has(key)) {
        cache.set(key, value);
      } else {
        update(key, value);
      }
    }
  };
};
const IMPORTANT_MODIFIER = '!';
const createParseClassName = config => {
  const {
    separator,
    experimentalParseClassName
  } = config;
  const isSeparatorSingleCharacter = separator.length === 1;
  const firstSeparatorCharacter = separator[0];
  const separatorLength = separator.length;
  // parseClassName inspired by https://github.com/tailwindlabs/tailwindcss/blob/v3.2.2/src/util/splitAtTopLevelOnly.js
  const parseClassName = className => {
    const modifiers = [];
    let bracketDepth = 0;
    let modifierStart = 0;
    let postfixModifierPosition;
    for (let index = 0; index < className.length; index++) {
      let currentCharacter = className[index];
      if (bracketDepth === 0) {
        if (currentCharacter === firstSeparatorCharacter && (isSeparatorSingleCharacter || className.slice(index, index + separatorLength) === separator)) {
          modifiers.push(className.slice(modifierStart, index));
          modifierStart = index + separatorLength;
          continue;
        }
        if (currentCharacter === '/') {
          postfixModifierPosition = index;
          continue;
        }
      }
      if (currentCharacter === '[') {
        bracketDepth++;
      } else if (currentCharacter === ']') {
        bracketDepth--;
      }
    }
    const baseClassNameWithImportantModifier = modifiers.length === 0 ? className : className.substring(modifierStart);
    const hasImportantModifier = baseClassNameWithImportantModifier.startsWith(IMPORTANT_MODIFIER);
    const baseClassName = hasImportantModifier ? baseClassNameWithImportantModifier.substring(1) : baseClassNameWithImportantModifier;
    const maybePostfixModifierPosition = postfixModifierPosition && postfixModifierPosition > modifierStart ? postfixModifierPosition - modifierStart : undefined;
    return {
      modifiers,
      hasImportantModifier,
      baseClassName,
      maybePostfixModifierPosition
    };
  };
  if (experimentalParseClassName) {
    return className => experimentalParseClassName({
      className,
      parseClassName
    });
  }
  return parseClassName;
};
/**
 * Sorts modifiers according to following schema:
 * - Predefined modifiers are sorted alphabetically
 * - When an arbitrary variant appears, it must be preserved which modifiers are before and after it
 */
const sortModifiers = modifiers => {
  if (modifiers.length <= 1) {
    return modifiers;
  }
  const sortedModifiers = [];
  let unsortedModifiers = [];
  modifiers.forEach(modifier => {
    const isArbitraryVariant = modifier[0] === '[';
    if (isArbitraryVariant) {
      sortedModifiers.push(...unsortedModifiers.sort(), modifier);
      unsortedModifiers = [];
    } else {
      unsortedModifiers.push(modifier);
    }
  });
  sortedModifiers.push(...unsortedModifiers.sort());
  return sortedModifiers;
};
const createConfigUtils = config => ({
  cache: createLruCache(config.cacheSize),
  parseClassName: createParseClassName(config),
  ...createClassGroupUtils(config)
});
const SPLIT_CLASSES_REGEX = /\s+/;
const mergeClassList = (classList, configUtils) => {
  const {
    parseClassName,
    getClassGroupId,
    getConflictingClassGroupIds
  } = configUtils;
  /**
   * Set of classGroupIds in following format:
   * `{importantModifier}{variantModifiers}{classGroupId}`
   * @example 'float'
   * @example 'hover:focus:bg-color'
   * @example 'md:!pr'
   */
  const classGroupsInConflict = [];
  const classNames = classList.trim().split(SPLIT_CLASSES_REGEX);
  let result = '';
  for (let index = classNames.length - 1; index >= 0; index -= 1) {
    const originalClassName = classNames[index];
    const {
      modifiers,
      hasImportantModifier,
      baseClassName,
      maybePostfixModifierPosition
    } = parseClassName(originalClassName);
    let hasPostfixModifier = Boolean(maybePostfixModifierPosition);
    let classGroupId = getClassGroupId(hasPostfixModifier ? baseClassName.substring(0, maybePostfixModifierPosition) : baseClassName);
    if (!classGroupId) {
      if (!hasPostfixModifier) {
        // Not a Tailwind class
        result = originalClassName + (result.length > 0 ? ' ' + result : result);
        continue;
      }
      classGroupId = getClassGroupId(baseClassName);
      if (!classGroupId) {
        // Not a Tailwind class
        result = originalClassName + (result.length > 0 ? ' ' + result : result);
        continue;
      }
      hasPostfixModifier = false;
    }
    const variantModifier = sortModifiers(modifiers).join(':');
    const modifierId = hasImportantModifier ? variantModifier + IMPORTANT_MODIFIER : variantModifier;
    const classId = modifierId + classGroupId;
    if (classGroupsInConflict.includes(classId)) {
      // Tailwind class omitted due to conflict
      continue;
    }
    classGroupsInConflict.push(classId);
    const conflictGroups = getConflictingClassGroupIds(classGroupId, hasPostfixModifier);
    for (let i = 0; i < conflictGroups.length; ++i) {
      const group = conflictGroups[i];
      classGroupsInConflict.push(modifierId + group);
    }
    // Tailwind class not in conflict
    result = originalClassName + (result.length > 0 ? ' ' + result : result);
  }
  return result;
};

/**
 * The code in this file is copied from https://github.com/lukeed/clsx and modified to suit the needs of tailwind-merge better.
 *
 * Specifically:
 * - Runtime code from https://github.com/lukeed/clsx/blob/v1.2.1/src/index.js
 * - TypeScript types from https://github.com/lukeed/clsx/blob/v1.2.1/clsx.d.ts
 *
 * Original code has MIT license: Copyright (c) Luke Edwards <luke.edwards05@gmail.com> (lukeed.com)
 */
function twJoin() {
  let index = 0;
  let argument;
  let resolvedValue;
  let string = '';
  while (index < arguments.length) {
    if (argument = arguments[index++]) {
      if (resolvedValue = toValue(argument)) {
        string && (string += ' ');
        string += resolvedValue;
      }
    }
  }
  return string;
}
const toValue = mix => {
  if (typeof mix === 'string') {
    return mix;
  }
  let resolvedValue;
  let string = '';
  for (let k = 0; k < mix.length; k++) {
    if (mix[k]) {
      if (resolvedValue = toValue(mix[k])) {
        string && (string += ' ');
        string += resolvedValue;
      }
    }
  }
  return string;
};
function createTailwindMerge(createConfigFirst, ...createConfigRest) {
  let configUtils;
  let cacheGet;
  let cacheSet;
  let functionToCall = initTailwindMerge;
  function initTailwindMerge(classList) {
    const config = createConfigRest.reduce((previousConfig, createConfigCurrent) => createConfigCurrent(previousConfig), createConfigFirst());
    configUtils = createConfigUtils(config);
    cacheGet = configUtils.cache.get;
    cacheSet = configUtils.cache.set;
    functionToCall = tailwindMerge;
    return tailwindMerge(classList);
  }
  function tailwindMerge(classList) {
    const cachedResult = cacheGet(classList);
    if (cachedResult) {
      return cachedResult;
    }
    const result = mergeClassList(classList, configUtils);
    cacheSet(classList, result);
    return result;
  }
  return function callTailwindMerge() {
    return functionToCall(twJoin.apply(null, arguments));
  };
}
const fromTheme = key => {
  const themeGetter = theme => theme[key] || [];
  themeGetter.isThemeGetter = true;
  return themeGetter;
};
const arbitraryValueRegex = /^\[(?:([a-z-]+):)?(.+)\]$/i;
const fractionRegex = /^\d+\/\d+$/;
const stringLengths = /*#__PURE__*/new Set(['px', 'full', 'screen']);
const tshirtUnitRegex = /^(\d+(\.\d+)?)?(xs|sm|md|lg|xl)$/;
const lengthUnitRegex = /\d+(%|px|r?em|[sdl]?v([hwib]|min|max)|pt|pc|in|cm|mm|cap|ch|ex|r?lh|cq(w|h|i|b|min|max))|\b(calc|min|max|clamp)\(.+\)|^0$/;
const colorFunctionRegex = /^(rgba?|hsla?|hwb|(ok)?(lab|lch)|color-mix)\(.+\)$/;
// Shadow always begins with x and y offset separated by underscore optionally prepended by inset
const shadowRegex = /^(inset_)?-?((\d+)?\.?(\d+)[a-z]+|0)_-?((\d+)?\.?(\d+)[a-z]+|0)/;
const imageRegex = /^(url|image|image-set|cross-fade|element|(repeating-)?(linear|radial|conic)-gradient)\(.+\)$/;
const isLength = value => isNumber(value) || stringLengths.has(value) || fractionRegex.test(value);
const isArbitraryLength = value => getIsArbitraryValue(value, 'length', isLengthOnly);
const isNumber = value => Boolean(value) && !Number.isNaN(Number(value));
const isArbitraryNumber = value => getIsArbitraryValue(value, 'number', isNumber);
const isInteger = value => Boolean(value) && Number.isInteger(Number(value));
const isPercent = value => value.endsWith('%') && isNumber(value.slice(0, -1));
const isArbitraryValue = value => arbitraryValueRegex.test(value);
const isTshirtSize = value => tshirtUnitRegex.test(value);
const sizeLabels = /*#__PURE__*/new Set(['length', 'size', 'percentage']);
const isArbitrarySize = value => getIsArbitraryValue(value, sizeLabels, isNever);
const isArbitraryPosition = value => getIsArbitraryValue(value, 'position', isNever);
const imageLabels = /*#__PURE__*/new Set(['image', 'url']);
const isArbitraryImage = value => getIsArbitraryValue(value, imageLabels, isImage);
const isArbitraryShadow = value => getIsArbitraryValue(value, '', isShadow);
const isAny = () => true;
const getIsArbitraryValue = (value, label, testValue) => {
  const result = arbitraryValueRegex.exec(value);
  if (result) {
    if (result[1]) {
      return typeof label === 'string' ? result[1] === label : label.has(result[1]);
    }
    return testValue(result[2]);
  }
  return false;
};
const isLengthOnly = value =>
// `colorFunctionRegex` check is necessary because color functions can have percentages in them which which would be incorrectly classified as lengths.
// For example, `hsl(0 0% 0%)` would be classified as a length without this check.
// I could also use lookbehind assertion in `lengthUnitRegex` but that isn't supported widely enough.
lengthUnitRegex.test(value) && !colorFunctionRegex.test(value);
const isNever = () => false;
const isShadow = value => shadowRegex.test(value);
const isImage = value => imageRegex.test(value);
const getDefaultConfig = () => {
  const colors = fromTheme('colors');
  const spacing = fromTheme('spacing');
  const blur = fromTheme('blur');
  const brightness = fromTheme('brightness');
  const borderColor = fromTheme('borderColor');
  const borderRadius = fromTheme('borderRadius');
  const borderSpacing = fromTheme('borderSpacing');
  const borderWidth = fromTheme('borderWidth');
  const contrast = fromTheme('contrast');
  const grayscale = fromTheme('grayscale');
  const hueRotate = fromTheme('hueRotate');
  const invert = fromTheme('invert');
  const gap = fromTheme('gap');
  const gradientColorStops = fromTheme('gradientColorStops');
  const gradientColorStopPositions = fromTheme('gradientColorStopPositions');
  const inset = fromTheme('inset');
  const margin = fromTheme('margin');
  const opacity = fromTheme('opacity');
  const padding = fromTheme('padding');
  const saturate = fromTheme('saturate');
  const scale = fromTheme('scale');
  const sepia = fromTheme('sepia');
  const skew = fromTheme('skew');
  const space = fromTheme('space');
  const translate = fromTheme('translate');
  const getOverscroll = () => ['auto', 'contain', 'none'];
  const getOverflow = () => ['auto', 'hidden', 'clip', 'visible', 'scroll'];
  const getSpacingWithAutoAndArbitrary = () => ['auto', isArbitraryValue, spacing];
  const getSpacingWithArbitrary = () => [isArbitraryValue, spacing];
  const getLengthWithEmptyAndArbitrary = () => ['', isLength, isArbitraryLength];
  const getNumberWithAutoAndArbitrary = () => ['auto', isNumber, isArbitraryValue];
  const getPositions = () => ['bottom', 'center', 'left', 'left-bottom', 'left-top', 'right', 'right-bottom', 'right-top', 'top'];
  const getLineStyles = () => ['solid', 'dashed', 'dotted', 'double', 'none'];
  const getBlendModes = () => ['normal', 'multiply', 'screen', 'overlay', 'darken', 'lighten', 'color-dodge', 'color-burn', 'hard-light', 'soft-light', 'difference', 'exclusion', 'hue', 'saturation', 'color', 'luminosity'];
  const getAlign = () => ['start', 'end', 'center', 'between', 'around', 'evenly', 'stretch'];
  const getZeroAndEmpty = () => ['', '0', isArbitraryValue];
  const getBreaks = () => ['auto', 'avoid', 'all', 'avoid-page', 'page', 'left', 'right', 'column'];
  const getNumberAndArbitrary = () => [isNumber, isArbitraryValue];
  return {
    cacheSize: 500,
    separator: ':',
    theme: {
      colors: [isAny],
      spacing: [isLength, isArbitraryLength],
      blur: ['none', '', isTshirtSize, isArbitraryValue],
      brightness: getNumberAndArbitrary(),
      borderColor: [colors],
      borderRadius: ['none', '', 'full', isTshirtSize, isArbitraryValue],
      borderSpacing: getSpacingWithArbitrary(),
      borderWidth: getLengthWithEmptyAndArbitrary(),
      contrast: getNumberAndArbitrary(),
      grayscale: getZeroAndEmpty(),
      hueRotate: getNumberAndArbitrary(),
      invert: getZeroAndEmpty(),
      gap: getSpacingWithArbitrary(),
      gradientColorStops: [colors],
      gradientColorStopPositions: [isPercent, isArbitraryLength],
      inset: getSpacingWithAutoAndArbitrary(),
      margin: getSpacingWithAutoAndArbitrary(),
      opacity: getNumberAndArbitrary(),
      padding: getSpacingWithArbitrary(),
      saturate: getNumberAndArbitrary(),
      scale: getNumberAndArbitrary(),
      sepia: getZeroAndEmpty(),
      skew: getNumberAndArbitrary(),
      space: getSpacingWithArbitrary(),
      translate: getSpacingWithArbitrary()
    },
    classGroups: {
      // Layout
      /**
       * Aspect Ratio
       * @see https://tailwindcss.com/docs/aspect-ratio
       */
      aspect: [{
        aspect: ['auto', 'square', 'video', isArbitraryValue]
      }],
      /**
       * Container
       * @see https://tailwindcss.com/docs/container
       */
      container: ['container'],
      /**
       * Columns
       * @see https://tailwindcss.com/docs/columns
       */
      columns: [{
        columns: [isTshirtSize]
      }],
      /**
       * Break After
       * @see https://tailwindcss.com/docs/break-after
       */
      'break-after': [{
        'break-after': getBreaks()
      }],
      /**
       * Break Before
       * @see https://tailwindcss.com/docs/break-before
       */
      'break-before': [{
        'break-before': getBreaks()
      }],
      /**
       * Break Inside
       * @see https://tailwindcss.com/docs/break-inside
       */
      'break-inside': [{
        'break-inside': ['auto', 'avoid', 'avoid-page', 'avoid-column']
      }],
      /**
       * Box Decoration Break
       * @see https://tailwindcss.com/docs/box-decoration-break
       */
      'box-decoration': [{
        'box-decoration': ['slice', 'clone']
      }],
      /**
       * Box Sizing
       * @see https://tailwindcss.com/docs/box-sizing
       */
      box: [{
        box: ['border', 'content']
      }],
      /**
       * Display
       * @see https://tailwindcss.com/docs/display
       */
      display: ['block', 'inline-block', 'inline', 'flex', 'inline-flex', 'table', 'inline-table', 'table-caption', 'table-cell', 'table-column', 'table-column-group', 'table-footer-group', 'table-header-group', 'table-row-group', 'table-row', 'flow-root', 'grid', 'inline-grid', 'contents', 'list-item', 'hidden'],
      /**
       * Floats
       * @see https://tailwindcss.com/docs/float
       */
      float: [{
        float: ['right', 'left', 'none', 'start', 'end']
      }],
      /**
       * Clear
       * @see https://tailwindcss.com/docs/clear
       */
      clear: [{
        clear: ['left', 'right', 'both', 'none', 'start', 'end']
      }],
      /**
       * Isolation
       * @see https://tailwindcss.com/docs/isolation
       */
      isolation: ['isolate', 'isolation-auto'],
      /**
       * Object Fit
       * @see https://tailwindcss.com/docs/object-fit
       */
      'object-fit': [{
        object: ['contain', 'cover', 'fill', 'none', 'scale-down']
      }],
      /**
       * Object Position
       * @see https://tailwindcss.com/docs/object-position
       */
      'object-position': [{
        object: [...getPositions(), isArbitraryValue]
      }],
      /**
       * Overflow
       * @see https://tailwindcss.com/docs/overflow
       */
      overflow: [{
        overflow: getOverflow()
      }],
      /**
       * Overflow X
       * @see https://tailwindcss.com/docs/overflow
       */
      'overflow-x': [{
        'overflow-x': getOverflow()
      }],
      /**
       * Overflow Y
       * @see https://tailwindcss.com/docs/overflow
       */
      'overflow-y': [{
        'overflow-y': getOverflow()
      }],
      /**
       * Overscroll Behavior
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      overscroll: [{
        overscroll: getOverscroll()
      }],
      /**
       * Overscroll Behavior X
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      'overscroll-x': [{
        'overscroll-x': getOverscroll()
      }],
      /**
       * Overscroll Behavior Y
       * @see https://tailwindcss.com/docs/overscroll-behavior
       */
      'overscroll-y': [{
        'overscroll-y': getOverscroll()
      }],
      /**
       * Position
       * @see https://tailwindcss.com/docs/position
       */
      position: ['static', 'fixed', 'absolute', 'relative', 'sticky'],
      /**
       * Top / Right / Bottom / Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      inset: [{
        inset: [inset]
      }],
      /**
       * Right / Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      'inset-x': [{
        'inset-x': [inset]
      }],
      /**
       * Top / Bottom
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      'inset-y': [{
        'inset-y': [inset]
      }],
      /**
       * Start
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      start: [{
        start: [inset]
      }],
      /**
       * End
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      end: [{
        end: [inset]
      }],
      /**
       * Top
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      top: [{
        top: [inset]
      }],
      /**
       * Right
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      right: [{
        right: [inset]
      }],
      /**
       * Bottom
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      bottom: [{
        bottom: [inset]
      }],
      /**
       * Left
       * @see https://tailwindcss.com/docs/top-right-bottom-left
       */
      left: [{
        left: [inset]
      }],
      /**
       * Visibility
       * @see https://tailwindcss.com/docs/visibility
       */
      visibility: ['visible', 'invisible', 'collapse'],
      /**
       * Z-Index
       * @see https://tailwindcss.com/docs/z-index
       */
      z: [{
        z: ['auto', isInteger, isArbitraryValue]
      }],
      // Flexbox and Grid
      /**
       * Flex Basis
       * @see https://tailwindcss.com/docs/flex-basis
       */
      basis: [{
        basis: getSpacingWithAutoAndArbitrary()
      }],
      /**
       * Flex Direction
       * @see https://tailwindcss.com/docs/flex-direction
       */
      'flex-direction': [{
        flex: ['row', 'row-reverse', 'col', 'col-reverse']
      }],
      /**
       * Flex Wrap
       * @see https://tailwindcss.com/docs/flex-wrap
       */
      'flex-wrap': [{
        flex: ['wrap', 'wrap-reverse', 'nowrap']
      }],
      /**
       * Flex
       * @see https://tailwindcss.com/docs/flex
       */
      flex: [{
        flex: ['1', 'auto', 'initial', 'none', isArbitraryValue]
      }],
      /**
       * Flex Grow
       * @see https://tailwindcss.com/docs/flex-grow
       */
      grow: [{
        grow: getZeroAndEmpty()
      }],
      /**
       * Flex Shrink
       * @see https://tailwindcss.com/docs/flex-shrink
       */
      shrink: [{
        shrink: getZeroAndEmpty()
      }],
      /**
       * Order
       * @see https://tailwindcss.com/docs/order
       */
      order: [{
        order: ['first', 'last', 'none', isInteger, isArbitraryValue]
      }],
      /**
       * Grid Template Columns
       * @see https://tailwindcss.com/docs/grid-template-columns
       */
      'grid-cols': [{
        'grid-cols': [isAny]
      }],
      /**
       * Grid Column Start / End
       * @see https://tailwindcss.com/docs/grid-column
       */
      'col-start-end': [{
        col: ['auto', {
          span: ['full', isInteger, isArbitraryValue]
        }, isArbitraryValue]
      }],
      /**
       * Grid Column Start
       * @see https://tailwindcss.com/docs/grid-column
       */
      'col-start': [{
        'col-start': getNumberWithAutoAndArbitrary()
      }],
      /**
       * Grid Column End
       * @see https://tailwindcss.com/docs/grid-column
       */
      'col-end': [{
        'col-end': getNumberWithAutoAndArbitrary()
      }],
      /**
       * Grid Template Rows
       * @see https://tailwindcss.com/docs/grid-template-rows
       */
      'grid-rows': [{
        'grid-rows': [isAny]
      }],
      /**
       * Grid Row Start / End
       * @see https://tailwindcss.com/docs/grid-row
       */
      'row-start-end': [{
        row: ['auto', {
          span: [isInteger, isArbitraryValue]
        }, isArbitraryValue]
      }],
      /**
       * Grid Row Start
       * @see https://tailwindcss.com/docs/grid-row
       */
      'row-start': [{
        'row-start': getNumberWithAutoAndArbitrary()
      }],
      /**
       * Grid Row End
       * @see https://tailwindcss.com/docs/grid-row
       */
      'row-end': [{
        'row-end': getNumberWithAutoAndArbitrary()
      }],
      /**
       * Grid Auto Flow
       * @see https://tailwindcss.com/docs/grid-auto-flow
       */
      'grid-flow': [{
        'grid-flow': ['row', 'col', 'dense', 'row-dense', 'col-dense']
      }],
      /**
       * Grid Auto Columns
       * @see https://tailwindcss.com/docs/grid-auto-columns
       */
      'auto-cols': [{
        'auto-cols': ['auto', 'min', 'max', 'fr', isArbitraryValue]
      }],
      /**
       * Grid Auto Rows
       * @see https://tailwindcss.com/docs/grid-auto-rows
       */
      'auto-rows': [{
        'auto-rows': ['auto', 'min', 'max', 'fr', isArbitraryValue]
      }],
      /**
       * Gap
       * @see https://tailwindcss.com/docs/gap
       */
      gap: [{
        gap: [gap]
      }],
      /**
       * Gap X
       * @see https://tailwindcss.com/docs/gap
       */
      'gap-x': [{
        'gap-x': [gap]
      }],
      /**
       * Gap Y
       * @see https://tailwindcss.com/docs/gap
       */
      'gap-y': [{
        'gap-y': [gap]
      }],
      /**
       * Justify Content
       * @see https://tailwindcss.com/docs/justify-content
       */
      'justify-content': [{
        justify: ['normal', ...getAlign()]
      }],
      /**
       * Justify Items
       * @see https://tailwindcss.com/docs/justify-items
       */
      'justify-items': [{
        'justify-items': ['start', 'end', 'center', 'stretch']
      }],
      /**
       * Justify Self
       * @see https://tailwindcss.com/docs/justify-self
       */
      'justify-self': [{
        'justify-self': ['auto', 'start', 'end', 'center', 'stretch']
      }],
      /**
       * Align Content
       * @see https://tailwindcss.com/docs/align-content
       */
      'align-content': [{
        content: ['normal', ...getAlign(), 'baseline']
      }],
      /**
       * Align Items
       * @see https://tailwindcss.com/docs/align-items
       */
      'align-items': [{
        items: ['start', 'end', 'center', 'baseline', 'stretch']
      }],
      /**
       * Align Self
       * @see https://tailwindcss.com/docs/align-self
       */
      'align-self': [{
        self: ['auto', 'start', 'end', 'center', 'stretch', 'baseline']
      }],
      /**
       * Place Content
       * @see https://tailwindcss.com/docs/place-content
       */
      'place-content': [{
        'place-content': [...getAlign(), 'baseline']
      }],
      /**
       * Place Items
       * @see https://tailwindcss.com/docs/place-items
       */
      'place-items': [{
        'place-items': ['start', 'end', 'center', 'baseline', 'stretch']
      }],
      /**
       * Place Self
       * @see https://tailwindcss.com/docs/place-self
       */
      'place-self': [{
        'place-self': ['auto', 'start', 'end', 'center', 'stretch']
      }],
      // Spacing
      /**
       * Padding
       * @see https://tailwindcss.com/docs/padding
       */
      p: [{
        p: [padding]
      }],
      /**
       * Padding X
       * @see https://tailwindcss.com/docs/padding
       */
      px: [{
        px: [padding]
      }],
      /**
       * Padding Y
       * @see https://tailwindcss.com/docs/padding
       */
      py: [{
        py: [padding]
      }],
      /**
       * Padding Start
       * @see https://tailwindcss.com/docs/padding
       */
      ps: [{
        ps: [padding]
      }],
      /**
       * Padding End
       * @see https://tailwindcss.com/docs/padding
       */
      pe: [{
        pe: [padding]
      }],
      /**
       * Padding Top
       * @see https://tailwindcss.com/docs/padding
       */
      pt: [{
        pt: [padding]
      }],
      /**
       * Padding Right
       * @see https://tailwindcss.com/docs/padding
       */
      pr: [{
        pr: [padding]
      }],
      /**
       * Padding Bottom
       * @see https://tailwindcss.com/docs/padding
       */
      pb: [{
        pb: [padding]
      }],
      /**
       * Padding Left
       * @see https://tailwindcss.com/docs/padding
       */
      pl: [{
        pl: [padding]
      }],
      /**
       * Margin
       * @see https://tailwindcss.com/docs/margin
       */
      m: [{
        m: [margin]
      }],
      /**
       * Margin X
       * @see https://tailwindcss.com/docs/margin
       */
      mx: [{
        mx: [margin]
      }],
      /**
       * Margin Y
       * @see https://tailwindcss.com/docs/margin
       */
      my: [{
        my: [margin]
      }],
      /**
       * Margin Start
       * @see https://tailwindcss.com/docs/margin
       */
      ms: [{
        ms: [margin]
      }],
      /**
       * Margin End
       * @see https://tailwindcss.com/docs/margin
       */
      me: [{
        me: [margin]
      }],
      /**
       * Margin Top
       * @see https://tailwindcss.com/docs/margin
       */
      mt: [{
        mt: [margin]
      }],
      /**
       * Margin Right
       * @see https://tailwindcss.com/docs/margin
       */
      mr: [{
        mr: [margin]
      }],
      /**
       * Margin Bottom
       * @see https://tailwindcss.com/docs/margin
       */
      mb: [{
        mb: [margin]
      }],
      /**
       * Margin Left
       * @see https://tailwindcss.com/docs/margin
       */
      ml: [{
        ml: [margin]
      }],
      /**
       * Space Between X
       * @see https://tailwindcss.com/docs/space
       */
      'space-x': [{
        'space-x': [space]
      }],
      /**
       * Space Between X Reverse
       * @see https://tailwindcss.com/docs/space
       */
      'space-x-reverse': ['space-x-reverse'],
      /**
       * Space Between Y
       * @see https://tailwindcss.com/docs/space
       */
      'space-y': [{
        'space-y': [space]
      }],
      /**
       * Space Between Y Reverse
       * @see https://tailwindcss.com/docs/space
       */
      'space-y-reverse': ['space-y-reverse'],
      // Sizing
      /**
       * Width
       * @see https://tailwindcss.com/docs/width
       */
      w: [{
        w: ['auto', 'min', 'max', 'fit', 'svw', 'lvw', 'dvw', isArbitraryValue, spacing]
      }],
      /**
       * Min-Width
       * @see https://tailwindcss.com/docs/min-width
       */
      'min-w': [{
        'min-w': [isArbitraryValue, spacing, 'min', 'max', 'fit']
      }],
      /**
       * Max-Width
       * @see https://tailwindcss.com/docs/max-width
       */
      'max-w': [{
        'max-w': [isArbitraryValue, spacing, 'none', 'full', 'min', 'max', 'fit', 'prose', {
          screen: [isTshirtSize]
        }, isTshirtSize]
      }],
      /**
       * Height
       * @see https://tailwindcss.com/docs/height
       */
      h: [{
        h: [isArbitraryValue, spacing, 'auto', 'min', 'max', 'fit', 'svh', 'lvh', 'dvh']
      }],
      /**
       * Min-Height
       * @see https://tailwindcss.com/docs/min-height
       */
      'min-h': [{
        'min-h': [isArbitraryValue, spacing, 'min', 'max', 'fit', 'svh', 'lvh', 'dvh']
      }],
      /**
       * Max-Height
       * @see https://tailwindcss.com/docs/max-height
       */
      'max-h': [{
        'max-h': [isArbitraryValue, spacing, 'min', 'max', 'fit', 'svh', 'lvh', 'dvh']
      }],
      /**
       * Size
       * @see https://tailwindcss.com/docs/size
       */
      size: [{
        size: [isArbitraryValue, spacing, 'auto', 'min', 'max', 'fit']
      }],
      // Typography
      /**
       * Font Size
       * @see https://tailwindcss.com/docs/font-size
       */
      'font-size': [{
        text: ['base', isTshirtSize, isArbitraryLength]
      }],
      /**
       * Font Smoothing
       * @see https://tailwindcss.com/docs/font-smoothing
       */
      'font-smoothing': ['antialiased', 'subpixel-antialiased'],
      /**
       * Font Style
       * @see https://tailwindcss.com/docs/font-style
       */
      'font-style': ['italic', 'not-italic'],
      /**
       * Font Weight
       * @see https://tailwindcss.com/docs/font-weight
       */
      'font-weight': [{
        font: ['thin', 'extralight', 'light', 'normal', 'medium', 'semibold', 'bold', 'extrabold', 'black', isArbitraryNumber]
      }],
      /**
       * Font Family
       * @see https://tailwindcss.com/docs/font-family
       */
      'font-family': [{
        font: [isAny]
      }],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-normal': ['normal-nums'],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-ordinal': ['ordinal'],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-slashed-zero': ['slashed-zero'],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-figure': ['lining-nums', 'oldstyle-nums'],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-spacing': ['proportional-nums', 'tabular-nums'],
      /**
       * Font Variant Numeric
       * @see https://tailwindcss.com/docs/font-variant-numeric
       */
      'fvn-fraction': ['diagonal-fractions', 'stacked-fractions'],
      /**
       * Letter Spacing
       * @see https://tailwindcss.com/docs/letter-spacing
       */
      tracking: [{
        tracking: ['tighter', 'tight', 'normal', 'wide', 'wider', 'widest', isArbitraryValue]
      }],
      /**
       * Line Clamp
       * @see https://tailwindcss.com/docs/line-clamp
       */
      'line-clamp': [{
        'line-clamp': ['none', isNumber, isArbitraryNumber]
      }],
      /**
       * Line Height
       * @see https://tailwindcss.com/docs/line-height
       */
      leading: [{
        leading: ['none', 'tight', 'snug', 'normal', 'relaxed', 'loose', isLength, isArbitraryValue]
      }],
      /**
       * List Style Image
       * @see https://tailwindcss.com/docs/list-style-image
       */
      'list-image': [{
        'list-image': ['none', isArbitraryValue]
      }],
      /**
       * List Style Type
       * @see https://tailwindcss.com/docs/list-style-type
       */
      'list-style-type': [{
        list: ['none', 'disc', 'decimal', isArbitraryValue]
      }],
      /**
       * List Style Position
       * @see https://tailwindcss.com/docs/list-style-position
       */
      'list-style-position': [{
        list: ['inside', 'outside']
      }],
      /**
       * Placeholder Color
       * @deprecated since Tailwind CSS v3.0.0
       * @see https://tailwindcss.com/docs/placeholder-color
       */
      'placeholder-color': [{
        placeholder: [colors]
      }],
      /**
       * Placeholder Opacity
       * @see https://tailwindcss.com/docs/placeholder-opacity
       */
      'placeholder-opacity': [{
        'placeholder-opacity': [opacity]
      }],
      /**
       * Text Alignment
       * @see https://tailwindcss.com/docs/text-align
       */
      'text-alignment': [{
        text: ['left', 'center', 'right', 'justify', 'start', 'end']
      }],
      /**
       * Text Color
       * @see https://tailwindcss.com/docs/text-color
       */
      'text-color': [{
        text: [colors]
      }],
      /**
       * Text Opacity
       * @see https://tailwindcss.com/docs/text-opacity
       */
      'text-opacity': [{
        'text-opacity': [opacity]
      }],
      /**
       * Text Decoration
       * @see https://tailwindcss.com/docs/text-decoration
       */
      'text-decoration': ['underline', 'overline', 'line-through', 'no-underline'],
      /**
       * Text Decoration Style
       * @see https://tailwindcss.com/docs/text-decoration-style
       */
      'text-decoration-style': [{
        decoration: [...getLineStyles(), 'wavy']
      }],
      /**
       * Text Decoration Thickness
       * @see https://tailwindcss.com/docs/text-decoration-thickness
       */
      'text-decoration-thickness': [{
        decoration: ['auto', 'from-font', isLength, isArbitraryLength]
      }],
      /**
       * Text Underline Offset
       * @see https://tailwindcss.com/docs/text-underline-offset
       */
      'underline-offset': [{
        'underline-offset': ['auto', isLength, isArbitraryValue]
      }],
      /**
       * Text Decoration Color
       * @see https://tailwindcss.com/docs/text-decoration-color
       */
      'text-decoration-color': [{
        decoration: [colors]
      }],
      /**
       * Text Transform
       * @see https://tailwindcss.com/docs/text-transform
       */
      'text-transform': ['uppercase', 'lowercase', 'capitalize', 'normal-case'],
      /**
       * Text Overflow
       * @see https://tailwindcss.com/docs/text-overflow
       */
      'text-overflow': ['truncate', 'text-ellipsis', 'text-clip'],
      /**
       * Text Wrap
       * @see https://tailwindcss.com/docs/text-wrap
       */
      'text-wrap': [{
        text: ['wrap', 'nowrap', 'balance', 'pretty']
      }],
      /**
       * Text Indent
       * @see https://tailwindcss.com/docs/text-indent
       */
      indent: [{
        indent: getSpacingWithArbitrary()
      }],
      /**
       * Vertical Alignment
       * @see https://tailwindcss.com/docs/vertical-align
       */
      'vertical-align': [{
        align: ['baseline', 'top', 'middle', 'bottom', 'text-top', 'text-bottom', 'sub', 'super', isArbitraryValue]
      }],
      /**
       * Whitespace
       * @see https://tailwindcss.com/docs/whitespace
       */
      whitespace: [{
        whitespace: ['normal', 'nowrap', 'pre', 'pre-line', 'pre-wrap', 'break-spaces']
      }],
      /**
       * Word Break
       * @see https://tailwindcss.com/docs/word-break
       */
      break: [{
        break: ['normal', 'words', 'all', 'keep']
      }],
      /**
       * Hyphens
       * @see https://tailwindcss.com/docs/hyphens
       */
      hyphens: [{
        hyphens: ['none', 'manual', 'auto']
      }],
      /**
       * Content
       * @see https://tailwindcss.com/docs/content
       */
      content: [{
        content: ['none', isArbitraryValue]
      }],
      // Backgrounds
      /**
       * Background Attachment
       * @see https://tailwindcss.com/docs/background-attachment
       */
      'bg-attachment': [{
        bg: ['fixed', 'local', 'scroll']
      }],
      /**
       * Background Clip
       * @see https://tailwindcss.com/docs/background-clip
       */
      'bg-clip': [{
        'bg-clip': ['border', 'padding', 'content', 'text']
      }],
      /**
       * Background Opacity
       * @deprecated since Tailwind CSS v3.0.0
       * @see https://tailwindcss.com/docs/background-opacity
       */
      'bg-opacity': [{
        'bg-opacity': [opacity]
      }],
      /**
       * Background Origin
       * @see https://tailwindcss.com/docs/background-origin
       */
      'bg-origin': [{
        'bg-origin': ['border', 'padding', 'content']
      }],
      /**
       * Background Position
       * @see https://tailwindcss.com/docs/background-position
       */
      'bg-position': [{
        bg: [...getPositions(), isArbitraryPosition]
      }],
      /**
       * Background Repeat
       * @see https://tailwindcss.com/docs/background-repeat
       */
      'bg-repeat': [{
        bg: ['no-repeat', {
          repeat: ['', 'x', 'y', 'round', 'space']
        }]
      }],
      /**
       * Background Size
       * @see https://tailwindcss.com/docs/background-size
       */
      'bg-size': [{
        bg: ['auto', 'cover', 'contain', isArbitrarySize]
      }],
      /**
       * Background Image
       * @see https://tailwindcss.com/docs/background-image
       */
      'bg-image': [{
        bg: ['none', {
          'gradient-to': ['t', 'tr', 'r', 'br', 'b', 'bl', 'l', 'tl']
        }, isArbitraryImage]
      }],
      /**
       * Background Color
       * @see https://tailwindcss.com/docs/background-color
       */
      'bg-color': [{
        bg: [colors]
      }],
      /**
       * Gradient Color Stops From Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-from-pos': [{
        from: [gradientColorStopPositions]
      }],
      /**
       * Gradient Color Stops Via Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-via-pos': [{
        via: [gradientColorStopPositions]
      }],
      /**
       * Gradient Color Stops To Position
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-to-pos': [{
        to: [gradientColorStopPositions]
      }],
      /**
       * Gradient Color Stops From
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-from': [{
        from: [gradientColorStops]
      }],
      /**
       * Gradient Color Stops Via
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-via': [{
        via: [gradientColorStops]
      }],
      /**
       * Gradient Color Stops To
       * @see https://tailwindcss.com/docs/gradient-color-stops
       */
      'gradient-to': [{
        to: [gradientColorStops]
      }],
      // Borders
      /**
       * Border Radius
       * @see https://tailwindcss.com/docs/border-radius
       */
      rounded: [{
        rounded: [borderRadius]
      }],
      /**
       * Border Radius Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-s': [{
        'rounded-s': [borderRadius]
      }],
      /**
       * Border Radius End
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-e': [{
        'rounded-e': [borderRadius]
      }],
      /**
       * Border Radius Top
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-t': [{
        'rounded-t': [borderRadius]
      }],
      /**
       * Border Radius Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-r': [{
        'rounded-r': [borderRadius]
      }],
      /**
       * Border Radius Bottom
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-b': [{
        'rounded-b': [borderRadius]
      }],
      /**
       * Border Radius Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-l': [{
        'rounded-l': [borderRadius]
      }],
      /**
       * Border Radius Start Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-ss': [{
        'rounded-ss': [borderRadius]
      }],
      /**
       * Border Radius Start End
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-se': [{
        'rounded-se': [borderRadius]
      }],
      /**
       * Border Radius End End
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-ee': [{
        'rounded-ee': [borderRadius]
      }],
      /**
       * Border Radius End Start
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-es': [{
        'rounded-es': [borderRadius]
      }],
      /**
       * Border Radius Top Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-tl': [{
        'rounded-tl': [borderRadius]
      }],
      /**
       * Border Radius Top Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-tr': [{
        'rounded-tr': [borderRadius]
      }],
      /**
       * Border Radius Bottom Right
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-br': [{
        'rounded-br': [borderRadius]
      }],
      /**
       * Border Radius Bottom Left
       * @see https://tailwindcss.com/docs/border-radius
       */
      'rounded-bl': [{
        'rounded-bl': [borderRadius]
      }],
      /**
       * Border Width
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w': [{
        border: [borderWidth]
      }],
      /**
       * Border Width X
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-x': [{
        'border-x': [borderWidth]
      }],
      /**
       * Border Width Y
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-y': [{
        'border-y': [borderWidth]
      }],
      /**
       * Border Width Start
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-s': [{
        'border-s': [borderWidth]
      }],
      /**
       * Border Width End
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-e': [{
        'border-e': [borderWidth]
      }],
      /**
       * Border Width Top
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-t': [{
        'border-t': [borderWidth]
      }],
      /**
       * Border Width Right
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-r': [{
        'border-r': [borderWidth]
      }],
      /**
       * Border Width Bottom
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-b': [{
        'border-b': [borderWidth]
      }],
      /**
       * Border Width Left
       * @see https://tailwindcss.com/docs/border-width
       */
      'border-w-l': [{
        'border-l': [borderWidth]
      }],
      /**
       * Border Opacity
       * @see https://tailwindcss.com/docs/border-opacity
       */
      'border-opacity': [{
        'border-opacity': [opacity]
      }],
      /**
       * Border Style
       * @see https://tailwindcss.com/docs/border-style
       */
      'border-style': [{
        border: [...getLineStyles(), 'hidden']
      }],
      /**
       * Divide Width X
       * @see https://tailwindcss.com/docs/divide-width
       */
      'divide-x': [{
        'divide-x': [borderWidth]
      }],
      /**
       * Divide Width X Reverse
       * @see https://tailwindcss.com/docs/divide-width
       */
      'divide-x-reverse': ['divide-x-reverse'],
      /**
       * Divide Width Y
       * @see https://tailwindcss.com/docs/divide-width
       */
      'divide-y': [{
        'divide-y': [borderWidth]
      }],
      /**
       * Divide Width Y Reverse
       * @see https://tailwindcss.com/docs/divide-width
       */
      'divide-y-reverse': ['divide-y-reverse'],
      /**
       * Divide Opacity
       * @see https://tailwindcss.com/docs/divide-opacity
       */
      'divide-opacity': [{
        'divide-opacity': [opacity]
      }],
      /**
       * Divide Style
       * @see https://tailwindcss.com/docs/divide-style
       */
      'divide-style': [{
        divide: getLineStyles()
      }],
      /**
       * Border Color
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color': [{
        border: [borderColor]
      }],
      /**
       * Border Color X
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-x': [{
        'border-x': [borderColor]
      }],
      /**
       * Border Color Y
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-y': [{
        'border-y': [borderColor]
      }],
      /**
       * Border Color S
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-s': [{
        'border-s': [borderColor]
      }],
      /**
       * Border Color E
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-e': [{
        'border-e': [borderColor]
      }],
      /**
       * Border Color Top
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-t': [{
        'border-t': [borderColor]
      }],
      /**
       * Border Color Right
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-r': [{
        'border-r': [borderColor]
      }],
      /**
       * Border Color Bottom
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-b': [{
        'border-b': [borderColor]
      }],
      /**
       * Border Color Left
       * @see https://tailwindcss.com/docs/border-color
       */
      'border-color-l': [{
        'border-l': [borderColor]
      }],
      /**
       * Divide Color
       * @see https://tailwindcss.com/docs/divide-color
       */
      'divide-color': [{
        divide: [borderColor]
      }],
      /**
       * Outline Style
       * @see https://tailwindcss.com/docs/outline-style
       */
      'outline-style': [{
        outline: ['', ...getLineStyles()]
      }],
      /**
       * Outline Offset
       * @see https://tailwindcss.com/docs/outline-offset
       */
      'outline-offset': [{
        'outline-offset': [isLength, isArbitraryValue]
      }],
      /**
       * Outline Width
       * @see https://tailwindcss.com/docs/outline-width
       */
      'outline-w': [{
        outline: [isLength, isArbitraryLength]
      }],
      /**
       * Outline Color
       * @see https://tailwindcss.com/docs/outline-color
       */
      'outline-color': [{
        outline: [colors]
      }],
      /**
       * Ring Width
       * @see https://tailwindcss.com/docs/ring-width
       */
      'ring-w': [{
        ring: getLengthWithEmptyAndArbitrary()
      }],
      /**
       * Ring Width Inset
       * @see https://tailwindcss.com/docs/ring-width
       */
      'ring-w-inset': ['ring-inset'],
      /**
       * Ring Color
       * @see https://tailwindcss.com/docs/ring-color
       */
      'ring-color': [{
        ring: [colors]
      }],
      /**
       * Ring Opacity
       * @see https://tailwindcss.com/docs/ring-opacity
       */
      'ring-opacity': [{
        'ring-opacity': [opacity]
      }],
      /**
       * Ring Offset Width
       * @see https://tailwindcss.com/docs/ring-offset-width
       */
      'ring-offset-w': [{
        'ring-offset': [isLength, isArbitraryLength]
      }],
      /**
       * Ring Offset Color
       * @see https://tailwindcss.com/docs/ring-offset-color
       */
      'ring-offset-color': [{
        'ring-offset': [colors]
      }],
      // Effects
      /**
       * Box Shadow
       * @see https://tailwindcss.com/docs/box-shadow
       */
      shadow: [{
        shadow: ['', 'inner', 'none', isTshirtSize, isArbitraryShadow]
      }],
      /**
       * Box Shadow Color
       * @see https://tailwindcss.com/docs/box-shadow-color
       */
      'shadow-color': [{
        shadow: [isAny]
      }],
      /**
       * Opacity
       * @see https://tailwindcss.com/docs/opacity
       */
      opacity: [{
        opacity: [opacity]
      }],
      /**
       * Mix Blend Mode
       * @see https://tailwindcss.com/docs/mix-blend-mode
       */
      'mix-blend': [{
        'mix-blend': [...getBlendModes(), 'plus-lighter', 'plus-darker']
      }],
      /**
       * Background Blend Mode
       * @see https://tailwindcss.com/docs/background-blend-mode
       */
      'bg-blend': [{
        'bg-blend': getBlendModes()
      }],
      // Filters
      /**
       * Filter
       * @deprecated since Tailwind CSS v3.0.0
       * @see https://tailwindcss.com/docs/filter
       */
      filter: [{
        filter: ['', 'none']
      }],
      /**
       * Blur
       * @see https://tailwindcss.com/docs/blur
       */
      blur: [{
        blur: [blur]
      }],
      /**
       * Brightness
       * @see https://tailwindcss.com/docs/brightness
       */
      brightness: [{
        brightness: [brightness]
      }],
      /**
       * Contrast
       * @see https://tailwindcss.com/docs/contrast
       */
      contrast: [{
        contrast: [contrast]
      }],
      /**
       * Drop Shadow
       * @see https://tailwindcss.com/docs/drop-shadow
       */
      'drop-shadow': [{
        'drop-shadow': ['', 'none', isTshirtSize, isArbitraryValue]
      }],
      /**
       * Grayscale
       * @see https://tailwindcss.com/docs/grayscale
       */
      grayscale: [{
        grayscale: [grayscale]
      }],
      /**
       * Hue Rotate
       * @see https://tailwindcss.com/docs/hue-rotate
       */
      'hue-rotate': [{
        'hue-rotate': [hueRotate]
      }],
      /**
       * Invert
       * @see https://tailwindcss.com/docs/invert
       */
      invert: [{
        invert: [invert]
      }],
      /**
       * Saturate
       * @see https://tailwindcss.com/docs/saturate
       */
      saturate: [{
        saturate: [saturate]
      }],
      /**
       * Sepia
       * @see https://tailwindcss.com/docs/sepia
       */
      sepia: [{
        sepia: [sepia]
      }],
      /**
       * Backdrop Filter
       * @deprecated since Tailwind CSS v3.0.0
       * @see https://tailwindcss.com/docs/backdrop-filter
       */
      'backdrop-filter': [{
        'backdrop-filter': ['', 'none']
      }],
      /**
       * Backdrop Blur
       * @see https://tailwindcss.com/docs/backdrop-blur
       */
      'backdrop-blur': [{
        'backdrop-blur': [blur]
      }],
      /**
       * Backdrop Brightness
       * @see https://tailwindcss.com/docs/backdrop-brightness
       */
      'backdrop-brightness': [{
        'backdrop-brightness': [brightness]
      }],
      /**
       * Backdrop Contrast
       * @see https://tailwindcss.com/docs/backdrop-contrast
       */
      'backdrop-contrast': [{
        'backdrop-contrast': [contrast]
      }],
      /**
       * Backdrop Grayscale
       * @see https://tailwindcss.com/docs/backdrop-grayscale
       */
      'backdrop-grayscale': [{
        'backdrop-grayscale': [grayscale]
      }],
      /**
       * Backdrop Hue Rotate
       * @see https://tailwindcss.com/docs/backdrop-hue-rotate
       */
      'backdrop-hue-rotate': [{
        'backdrop-hue-rotate': [hueRotate]
      }],
      /**
       * Backdrop Invert
       * @see https://tailwindcss.com/docs/backdrop-invert
       */
      'backdrop-invert': [{
        'backdrop-invert': [invert]
      }],
      /**
       * Backdrop Opacity
       * @see https://tailwindcss.com/docs/backdrop-opacity
       */
      'backdrop-opacity': [{
        'backdrop-opacity': [opacity]
      }],
      /**
       * Backdrop Saturate
       * @see https://tailwindcss.com/docs/backdrop-saturate
       */
      'backdrop-saturate': [{
        'backdrop-saturate': [saturate]
      }],
      /**
       * Backdrop Sepia
       * @see https://tailwindcss.com/docs/backdrop-sepia
       */
      'backdrop-sepia': [{
        'backdrop-sepia': [sepia]
      }],
      // Tables
      /**
       * Border Collapse
       * @see https://tailwindcss.com/docs/border-collapse
       */
      'border-collapse': [{
        border: ['collapse', 'separate']
      }],
      /**
       * Border Spacing
       * @see https://tailwindcss.com/docs/border-spacing
       */
      'border-spacing': [{
        'border-spacing': [borderSpacing]
      }],
      /**
       * Border Spacing X
       * @see https://tailwindcss.com/docs/border-spacing
       */
      'border-spacing-x': [{
        'border-spacing-x': [borderSpacing]
      }],
      /**
       * Border Spacing Y
       * @see https://tailwindcss.com/docs/border-spacing
       */
      'border-spacing-y': [{
        'border-spacing-y': [borderSpacing]
      }],
      /**
       * Table Layout
       * @see https://tailwindcss.com/docs/table-layout
       */
      'table-layout': [{
        table: ['auto', 'fixed']
      }],
      /**
       * Caption Side
       * @see https://tailwindcss.com/docs/caption-side
       */
      caption: [{
        caption: ['top', 'bottom']
      }],
      // Transitions and Animation
      /**
       * Tranisition Property
       * @see https://tailwindcss.com/docs/transition-property
       */
      transition: [{
        transition: ['none', 'all', '', 'colors', 'opacity', 'shadow', 'transform', isArbitraryValue]
      }],
      /**
       * Transition Duration
       * @see https://tailwindcss.com/docs/transition-duration
       */
      duration: [{
        duration: getNumberAndArbitrary()
      }],
      /**
       * Transition Timing Function
       * @see https://tailwindcss.com/docs/transition-timing-function
       */
      ease: [{
        ease: ['linear', 'in', 'out', 'in-out', isArbitraryValue]
      }],
      /**
       * Transition Delay
       * @see https://tailwindcss.com/docs/transition-delay
       */
      delay: [{
        delay: getNumberAndArbitrary()
      }],
      /**
       * Animation
       * @see https://tailwindcss.com/docs/animation
       */
      animate: [{
        animate: ['none', 'spin', 'ping', 'pulse', 'bounce', isArbitraryValue]
      }],
      // Transforms
      /**
       * Transform
       * @see https://tailwindcss.com/docs/transform
       */
      transform: [{
        transform: ['', 'gpu', 'none']
      }],
      /**
       * Scale
       * @see https://tailwindcss.com/docs/scale
       */
      scale: [{
        scale: [scale]
      }],
      /**
       * Scale X
       * @see https://tailwindcss.com/docs/scale
       */
      'scale-x': [{
        'scale-x': [scale]
      }],
      /**
       * Scale Y
       * @see https://tailwindcss.com/docs/scale
       */
      'scale-y': [{
        'scale-y': [scale]
      }],
      /**
       * Rotate
       * @see https://tailwindcss.com/docs/rotate
       */
      rotate: [{
        rotate: [isInteger, isArbitraryValue]
      }],
      /**
       * Translate X
       * @see https://tailwindcss.com/docs/translate
       */
      'translate-x': [{
        'translate-x': [translate]
      }],
      /**
       * Translate Y
       * @see https://tailwindcss.com/docs/translate
       */
      'translate-y': [{
        'translate-y': [translate]
      }],
      /**
       * Skew X
       * @see https://tailwindcss.com/docs/skew
       */
      'skew-x': [{
        'skew-x': [skew]
      }],
      /**
       * Skew Y
       * @see https://tailwindcss.com/docs/skew
       */
      'skew-y': [{
        'skew-y': [skew]
      }],
      /**
       * Transform Origin
       * @see https://tailwindcss.com/docs/transform-origin
       */
      'transform-origin': [{
        origin: ['center', 'top', 'top-right', 'right', 'bottom-right', 'bottom', 'bottom-left', 'left', 'top-left', isArbitraryValue]
      }],
      // Interactivity
      /**
       * Accent Color
       * @see https://tailwindcss.com/docs/accent-color
       */
      accent: [{
        accent: ['auto', colors]
      }],
      /**
       * Appearance
       * @see https://tailwindcss.com/docs/appearance
       */
      appearance: [{
        appearance: ['none', 'auto']
      }],
      /**
       * Cursor
       * @see https://tailwindcss.com/docs/cursor
       */
      cursor: [{
        cursor: ['auto', 'default', 'pointer', 'wait', 'text', 'move', 'help', 'not-allowed', 'none', 'context-menu', 'progress', 'cell', 'crosshair', 'vertical-text', 'alias', 'copy', 'no-drop', 'grab', 'grabbing', 'all-scroll', 'col-resize', 'row-resize', 'n-resize', 'e-resize', 's-resize', 'w-resize', 'ne-resize', 'nw-resize', 'se-resize', 'sw-resize', 'ew-resize', 'ns-resize', 'nesw-resize', 'nwse-resize', 'zoom-in', 'zoom-out', isArbitraryValue]
      }],
      /**
       * Caret Color
       * @see https://tailwindcss.com/docs/just-in-time-mode#caret-color-utilities
       */
      'caret-color': [{
        caret: [colors]
      }],
      /**
       * Pointer Events
       * @see https://tailwindcss.com/docs/pointer-events
       */
      'pointer-events': [{
        'pointer-events': ['none', 'auto']
      }],
      /**
       * Resize
       * @see https://tailwindcss.com/docs/resize
       */
      resize: [{
        resize: ['none', 'y', 'x', '']
      }],
      /**
       * Scroll Behavior
       * @see https://tailwindcss.com/docs/scroll-behavior
       */
      'scroll-behavior': [{
        scroll: ['auto', 'smooth']
      }],
      /**
       * Scroll Margin
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-m': [{
        'scroll-m': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin X
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-mx': [{
        'scroll-mx': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Y
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-my': [{
        'scroll-my': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Start
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-ms': [{
        'scroll-ms': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin End
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-me': [{
        'scroll-me': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Top
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-mt': [{
        'scroll-mt': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Right
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-mr': [{
        'scroll-mr': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Bottom
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-mb': [{
        'scroll-mb': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Margin Left
       * @see https://tailwindcss.com/docs/scroll-margin
       */
      'scroll-ml': [{
        'scroll-ml': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-p': [{
        'scroll-p': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding X
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-px': [{
        'scroll-px': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Y
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-py': [{
        'scroll-py': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Start
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-ps': [{
        'scroll-ps': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding End
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-pe': [{
        'scroll-pe': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Top
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-pt': [{
        'scroll-pt': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Right
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-pr': [{
        'scroll-pr': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Bottom
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-pb': [{
        'scroll-pb': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Padding Left
       * @see https://tailwindcss.com/docs/scroll-padding
       */
      'scroll-pl': [{
        'scroll-pl': getSpacingWithArbitrary()
      }],
      /**
       * Scroll Snap Align
       * @see https://tailwindcss.com/docs/scroll-snap-align
       */
      'snap-align': [{
        snap: ['start', 'end', 'center', 'align-none']
      }],
      /**
       * Scroll Snap Stop
       * @see https://tailwindcss.com/docs/scroll-snap-stop
       */
      'snap-stop': [{
        snap: ['normal', 'always']
      }],
      /**
       * Scroll Snap Type
       * @see https://tailwindcss.com/docs/scroll-snap-type
       */
      'snap-type': [{
        snap: ['none', 'x', 'y', 'both']
      }],
      /**
       * Scroll Snap Type Strictness
       * @see https://tailwindcss.com/docs/scroll-snap-type
       */
      'snap-strictness': [{
        snap: ['mandatory', 'proximity']
      }],
      /**
       * Touch Action
       * @see https://tailwindcss.com/docs/touch-action
       */
      touch: [{
        touch: ['auto', 'none', 'manipulation']
      }],
      /**
       * Touch Action X
       * @see https://tailwindcss.com/docs/touch-action
       */
      'touch-x': [{
        'touch-pan': ['x', 'left', 'right']
      }],
      /**
       * Touch Action Y
       * @see https://tailwindcss.com/docs/touch-action
       */
      'touch-y': [{
        'touch-pan': ['y', 'up', 'down']
      }],
      /**
       * Touch Action Pinch Zoom
       * @see https://tailwindcss.com/docs/touch-action
       */
      'touch-pz': ['touch-pinch-zoom'],
      /**
       * User Select
       * @see https://tailwindcss.com/docs/user-select
       */
      select: [{
        select: ['none', 'text', 'all', 'auto']
      }],
      /**
       * Will Change
       * @see https://tailwindcss.com/docs/will-change
       */
      'will-change': [{
        'will-change': ['auto', 'scroll', 'contents', 'transform', isArbitraryValue]
      }],
      // SVG
      /**
       * Fill
       * @see https://tailwindcss.com/docs/fill
       */
      fill: [{
        fill: [colors, 'none']
      }],
      /**
       * Stroke Width
       * @see https://tailwindcss.com/docs/stroke-width
       */
      'stroke-w': [{
        stroke: [isLength, isArbitraryLength, isArbitraryNumber]
      }],
      /**
       * Stroke
       * @see https://tailwindcss.com/docs/stroke
       */
      stroke: [{
        stroke: [colors, 'none']
      }],
      // Accessibility
      /**
       * Screen Readers
       * @see https://tailwindcss.com/docs/screen-readers
       */
      sr: ['sr-only', 'not-sr-only'],
      /**
       * Forced Color Adjust
       * @see https://tailwindcss.com/docs/forced-color-adjust
       */
      'forced-color-adjust': [{
        'forced-color-adjust': ['auto', 'none']
      }]
    },
    conflictingClassGroups: {
      overflow: ['overflow-x', 'overflow-y'],
      overscroll: ['overscroll-x', 'overscroll-y'],
      inset: ['inset-x', 'inset-y', 'start', 'end', 'top', 'right', 'bottom', 'left'],
      'inset-x': ['right', 'left'],
      'inset-y': ['top', 'bottom'],
      flex: ['basis', 'grow', 'shrink'],
      gap: ['gap-x', 'gap-y'],
      p: ['px', 'py', 'ps', 'pe', 'pt', 'pr', 'pb', 'pl'],
      px: ['pr', 'pl'],
      py: ['pt', 'pb'],
      m: ['mx', 'my', 'ms', 'me', 'mt', 'mr', 'mb', 'ml'],
      mx: ['mr', 'ml'],
      my: ['mt', 'mb'],
      size: ['w', 'h'],
      'font-size': ['leading'],
      'fvn-normal': ['fvn-ordinal', 'fvn-slashed-zero', 'fvn-figure', 'fvn-spacing', 'fvn-fraction'],
      'fvn-ordinal': ['fvn-normal'],
      'fvn-slashed-zero': ['fvn-normal'],
      'fvn-figure': ['fvn-normal'],
      'fvn-spacing': ['fvn-normal'],
      'fvn-fraction': ['fvn-normal'],
      'line-clamp': ['display', 'overflow'],
      rounded: ['rounded-s', 'rounded-e', 'rounded-t', 'rounded-r', 'rounded-b', 'rounded-l', 'rounded-ss', 'rounded-se', 'rounded-ee', 'rounded-es', 'rounded-tl', 'rounded-tr', 'rounded-br', 'rounded-bl'],
      'rounded-s': ['rounded-ss', 'rounded-es'],
      'rounded-e': ['rounded-se', 'rounded-ee'],
      'rounded-t': ['rounded-tl', 'rounded-tr'],
      'rounded-r': ['rounded-tr', 'rounded-br'],
      'rounded-b': ['rounded-br', 'rounded-bl'],
      'rounded-l': ['rounded-tl', 'rounded-bl'],
      'border-spacing': ['border-spacing-x', 'border-spacing-y'],
      'border-w': ['border-w-s', 'border-w-e', 'border-w-t', 'border-w-r', 'border-w-b', 'border-w-l'],
      'border-w-x': ['border-w-r', 'border-w-l'],
      'border-w-y': ['border-w-t', 'border-w-b'],
      'border-color': ['border-color-s', 'border-color-e', 'border-color-t', 'border-color-r', 'border-color-b', 'border-color-l'],
      'border-color-x': ['border-color-r', 'border-color-l'],
      'border-color-y': ['border-color-t', 'border-color-b'],
      'scroll-m': ['scroll-mx', 'scroll-my', 'scroll-ms', 'scroll-me', 'scroll-mt', 'scroll-mr', 'scroll-mb', 'scroll-ml'],
      'scroll-mx': ['scroll-mr', 'scroll-ml'],
      'scroll-my': ['scroll-mt', 'scroll-mb'],
      'scroll-p': ['scroll-px', 'scroll-py', 'scroll-ps', 'scroll-pe', 'scroll-pt', 'scroll-pr', 'scroll-pb', 'scroll-pl'],
      'scroll-px': ['scroll-pr', 'scroll-pl'],
      'scroll-py': ['scroll-pt', 'scroll-pb'],
      touch: ['touch-x', 'touch-y', 'touch-pz'],
      'touch-x': ['touch'],
      'touch-y': ['touch'],
      'touch-pz': ['touch']
    },
    conflictingClassGroupModifiers: {
      'font-size': ['leading']
    }
  };
};
const twMerge = /*#__PURE__*/createTailwindMerge(getDefaultConfig);

function cn(...inputs) {
  return twMerge(clsx(inputs));
}

function Card({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card",
      className: cn(
        "flex flex-col gap-6 rounded-xl border bg-card py-6 text-card-foreground shadow-sm",
        className
      ),
      ...props
    }
  );
}
function CardHeader({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card-header",
      className: cn("flex flex-col gap-1.5 px-6", className),
      ...props
    }
  );
}
function CardTitle({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card-title",
      className: cn("font-semibold leading-none tracking-tight", className),
      ...props
    }
  );
}
function CardDescription({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card-description",
      className: cn("text-sm text-muted-foreground", className),
      ...props
    }
  );
}
function CardContent({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card-content",
      className: cn("px-6", className),
      ...props
    }
  );
}
function CardFooter({ className, ...props }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "card-footer",
      className: cn("flex items-center px-6", className),
      ...props
    }
  );
}

const falsyToString = (value)=>typeof value === "boolean" ? `${value}` : value === 0 ? "0" : value;
const cx = clsx;
const cva = (base, config)=>(props)=>{
        var _config_compoundVariants;
        if ((config === null || config === void 0 ? void 0 : config.variants) == null) return cx(base, props === null || props === void 0 ? void 0 : props.class, props === null || props === void 0 ? void 0 : props.className);
        const { variants, defaultVariants } = config;
        const getVariantClassNames = Object.keys(variants).map((variant)=>{
            const variantProp = props === null || props === void 0 ? void 0 : props[variant];
            const defaultVariantProp = defaultVariants === null || defaultVariants === void 0 ? void 0 : defaultVariants[variant];
            if (variantProp === null) return null;
            const variantKey = falsyToString(variantProp) || falsyToString(defaultVariantProp);
            return variants[variant][variantKey];
        });
        const propsWithoutUndefined = props && Object.entries(props).reduce((acc, param)=>{
            let [key, value] = param;
            if (value === undefined) {
                return acc;
            }
            acc[key] = value;
            return acc;
        }, {});
        const getCompoundVariantClassNames = config === null || config === void 0 ? void 0 : (_config_compoundVariants = config.compoundVariants) === null || _config_compoundVariants === void 0 ? void 0 : _config_compoundVariants.reduce((acc, param)=>{
            let { class: cvClass, className: cvClassName, ...compoundVariantOptions } = param;
            return Object.entries(compoundVariantOptions).every((param)=>{
                let [key, value] = param;
                return Array.isArray(value) ? value.includes({
                    ...defaultVariants,
                    ...propsWithoutUndefined
                }[key]) : ({
                    ...defaultVariants,
                    ...propsWithoutUndefined
                })[key] === value;
            }) ? [
                ...acc,
                cvClass,
                cvClassName
            ] : acc;
        }, []);
        return cx(base, getVariantClassNames, getCompoundVariantClassNames, props === null || props === void 0 ? void 0 : props.class, props === null || props === void 0 ? void 0 : props.className);
    };

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow-sm hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        outline: "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
        ghost: "hover:bg-accent hover:text-accent-foreground",
        destructive: "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90"
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-6",
        icon: "size-9"
      }
    },
    defaultVariants: { variant: "default", size: "default" }
  }
);
function Button({ className, variant, size, ...props }) {
  return /* @__PURE__ */ jsx(
    "button",
    {
      "data-slot": "button",
      className: cn(buttonVariants({ variant, size }), className),
      ...props
    }
  );
}

const badgeVariants = cva(
  "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium w-fit whitespace-nowrap [&_svg]:size-3 [&_svg]:pointer-events-none",
  {
    variants: {
      variant: {
        default: "border-transparent bg-primary text-primary-foreground",
        secondary: "border-transparent bg-secondary text-secondary-foreground",
        outline: "text-foreground",
        success: "border-transparent bg-primary/15 text-primary",
        destructive: "border-transparent bg-destructive text-destructive-foreground"
      }
    },
    defaultVariants: { variant: "default" }
  }
);
function Badge({
  className,
  variant,
  ...props
}) {
  return /* @__PURE__ */ jsx(
    "span",
    {
      "data-slot": "badge",
      className: cn(badgeVariants({ variant }), className),
      ...props
    }
  );
}

function Separator({
  className,
  orientation = "horizontal",
  ...props
}) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-slot": "separator",
      role: "separator",
      "aria-orientation": orientation,
      className: cn(
        "shrink-0 bg-border",
        orientation === "horizontal" ? "h-px w-full" : "h-full w-px",
        className
      ),
      ...props
    }
  );
}

const EXTENSION_ID$3 = "com.acme.devices";
function sampleBarcode() {
  const part = () => Math.random().toString(36).slice(2, 6).toUpperCase().padEnd(4, "0");
  return `ACME-${part()}-${part()}`;
}
function ConsumerOnboard() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(
    "div",
    {
      "data-ext-id": EXTENSION_ID$3,
      className: "mx-auto flex max-w-md flex-col gap-5 p-1",
      children: /* @__PURE__ */ jsx(Wizard, {})
    }
  ) });
}
function Wizard() {
  const client = useHostClient();
  const [step, setStep] = React.useState("buy");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState(null);
  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [barcode, setBarcode] = React.useState(() => sampleBarcode());
  const [location, setLocation] = React.useState("Living Room");
  const [result, setResult] = React.useState(null);
  const signup = React.useCallback(() => {
    setError(null);
    setBusy(true);
    fetchJson(client, `/auth/signup`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ email: email.trim().toLowerCase(), password })
    }).then(() => setStep("addDevice")).catch((e) => setError(friendly(e))).finally(() => setBusy(false));
  }, [client, email, password]);
  const addDevice = React.useCallback(() => {
    setError(null);
    setBusy(true);
    fetchJson(client, `${client.apiPrefix}/onboard`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        email: email.trim().toLowerCase(),
        barcode: barcode.trim(),
        location
      })
    }).then((r) => {
      setResult(r);
      setStep("ready");
    }).catch((e) => setError(friendly(e))).finally(() => setBusy(false));
  }, [client, email, barcode, location]);
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx("span", { className: "grid size-8 place-items-center rounded-xl bg-primary/15 text-primary", children: /* @__PURE__ */ jsx(Box, { className: "size-4" }) }),
        /* @__PURE__ */ jsxs("div", { className: "flex flex-col leading-tight", children: [
          /* @__PURE__ */ jsx("span", { className: "text-sm font-semibold", children: "Acme Home" }),
          /* @__PURE__ */ jsx("span", { className: "text-xs text-muted-foreground", children: "device companion app" })
        ] })
      ] }),
      /* @__PURE__ */ jsx(StepDots, { step })
    ] }),
    error ? /* @__PURE__ */ jsx("div", { className: "rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive", children: error }) : null,
    step === "buy" ? /* @__PURE__ */ jsx(BuyCard, { onNext: () => setStep("signup") }) : step === "signup" ? /* @__PURE__ */ jsx(
      SignupCard,
      {
        email,
        password,
        busy,
        onEmail: setEmail,
        onPassword: setPassword,
        onBack: () => setStep("buy"),
        onNext: signup
      }
    ) : step === "addDevice" ? /* @__PURE__ */ jsx(
      AddDeviceCard,
      {
        barcode,
        location,
        busy,
        onBarcode: setBarcode,
        onLocation: setLocation,
        onShuffle: () => setBarcode(sampleBarcode()),
        onNext: addDevice
      }
    ) : /* @__PURE__ */ jsx(
      ReadyCard,
      {
        result,
        email,
        location,
        baseUrl: client.baseUrl
      }
    ),
    /* @__PURE__ */ jsx("p", { className: "px-1 text-center text-[11px] leading-relaxed text-muted-foreground", children: "A real end-to-end flow: self-service signup, then a privileged backend step provisions your device and a private dashboard scoped to you. Log in as the new user afterwards and you'll see only your device." })
  ] });
}
function BuyCard({ onNext }) {
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(Sparkles, { className: "size-5" }),
        " You bought an Acme Sensor"
      ] }),
      /* @__PURE__ */ jsx(CardDescription, { children: "Set it up in two minutes. Create your account, scan the box barcode, and your dashboard is ready." })
    ] }),
    /* @__PURE__ */ jsx(CardContent, { className: "flex flex-col gap-3", children: /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3 rounded-lg border bg-muted/30 p-3", children: [
      /* @__PURE__ */ jsx("span", { className: "grid size-12 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary", children: /* @__PURE__ */ jsx(Cpu, { className: "size-6" }) }),
      /* @__PURE__ */ jsxs("div", { className: "flex flex-col", children: [
        /* @__PURE__ */ jsx("span", { className: "text-sm font-medium", children: "Acme Sensor — Model S1" }),
        /* @__PURE__ */ jsx("span", { className: "text-xs text-muted-foreground", children: "Temperature · humidity · air quality" })
      ] }),
      /* @__PURE__ */ jsx(Badge, { variant: "secondary", className: "ml-auto", children: "In the box" })
    ] }) }),
    /* @__PURE__ */ jsx(CardFooter, { children: /* @__PURE__ */ jsxs(Button, { onClick: onNext, className: "w-full", children: [
      "I bought this — set it up ",
      /* @__PURE__ */ jsx(ArrowRight, {})
    ] }) })
  ] });
}
function SignupCard({
  email,
  password,
  busy,
  onEmail,
  onPassword,
  onBack,
  onNext
}) {
  const canSubmit = email.includes("@") && password.length >= 8 && !busy;
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(KeyRound, { className: "size-5" }),
        " Create your account"
      ] }),
      /* @__PURE__ */ jsx(CardDescription, { children: "Self-service signup — this creates a real user (default role: reader)." })
    ] }),
    /* @__PURE__ */ jsxs(CardContent, { className: "flex flex-col gap-3", children: [
      /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Email" }),
        /* @__PURE__ */ jsx(
          "input",
          {
            type: "email",
            value: email,
            onChange: (e) => onEmail(e.target.value),
            placeholder: "you@home.example",
            disabled: busy,
            className: "h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          }
        )
      ] }),
      /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
        /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground", children: [
          "Password ",
          /* @__PURE__ */ jsx("span", { className: "text-muted-foreground/70", children: "(min 8 chars)" })
        ] }),
        /* @__PURE__ */ jsx(
          "input",
          {
            type: "password",
            value: password,
            onChange: (e) => onPassword(e.target.value),
            placeholder: "••••••••",
            disabled: busy,
            className: "h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ jsxs(CardFooter, { className: "gap-2", children: [
      /* @__PURE__ */ jsx(Button, { variant: "ghost", onClick: onBack, disabled: busy, children: "Back" }),
      /* @__PURE__ */ jsxs(Button, { onClick: onNext, disabled: !canSubmit, className: "ml-auto", children: [
        busy ? /* @__PURE__ */ jsx(LoaderCircle, { className: "animate-spin" }) : /* @__PURE__ */ jsx(ArrowRight, {}),
        busy ? "Creating…" : "Create account"
      ] })
    ] })
  ] });
}
function AddDeviceCard({
  barcode,
  location,
  busy,
  onBarcode,
  onLocation,
  onShuffle,
  onNext
}) {
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2", children: [
        /* @__PURE__ */ jsx(ScanLine, { className: "size-5" }),
        " Add your device"
      ] }),
      /* @__PURE__ */ jsx(CardDescription, { children: "Scan the barcode on the box. (No scanner here — use the sample, or shuffle a new one.)" })
    ] }),
    /* @__PURE__ */ jsxs(CardContent, { className: "flex flex-col gap-3", children: [
      /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Device barcode" }),
        /* @__PURE__ */ jsxs("div", { className: "flex gap-2", children: [
          /* @__PURE__ */ jsx(
            "input",
            {
              value: barcode,
              onChange: (e) => onBarcode(e.target.value),
              disabled: busy,
              className: "h-9 flex-1 rounded-md border border-input bg-transparent px-3 font-mono text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
            }
          ),
          /* @__PURE__ */ jsx(Button, { type: "button", size: "sm", variant: "outline", onClick: onShuffle, disabled: busy, children: "Shuffle" })
        ] })
      ] }),
      /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Where is it?" }),
        /* @__PURE__ */ jsx(
          "input",
          {
            value: location,
            onChange: (e) => onLocation(e.target.value),
            disabled: busy,
            className: "h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
          }
        )
      ] })
    ] }),
    /* @__PURE__ */ jsx(CardFooter, { children: /* @__PURE__ */ jsxs(Button, { onClick: onNext, disabled: busy || !barcode.trim(), className: "w-full", children: [
      busy ? /* @__PURE__ */ jsx(LoaderCircle, { className: "animate-spin" }) : /* @__PURE__ */ jsx(ScanLine, {}),
      busy ? "Setting up your workspace…" : "Register device"
    ] }) })
  ] });
}
function ReadyCard({
  result,
  email,
  location,
  baseUrl
}) {
  const dashUrl = result ? `${baseUrl}/dashboards/${result.dashboard_slug}` : "#";
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2 text-emerald-700 dark:text-emerald-400", children: [
        /* @__PURE__ */ jsx(PartyPopper, { className: "size-5" }),
        " You're all set"
      ] }),
      /* @__PURE__ */ jsx(CardDescription, { children: "Your device is registered and your private dashboard is ready." })
    ] }),
    /* @__PURE__ */ jsxs(CardContent, { className: "flex flex-col gap-3 text-sm", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center gap-3 rounded-lg border border-emerald-600/30 bg-emerald-600/10 p-3", children: [
        /* @__PURE__ */ jsx(BadgeCheck, { className: "size-5 text-emerald-600 dark:text-emerald-400" }),
        /* @__PURE__ */ jsxs("div", { className: "flex flex-col", children: [
          /* @__PURE__ */ jsx("span", { className: "font-medium", children: "Device provisioned" }),
          /* @__PURE__ */ jsxs("span", { className: "font-mono text-xs text-muted-foreground", children: [
            result?.device_id ?? "—",
            " · ",
            location
          ] })
        ] })
      ] }),
      /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-[6rem_1fr] gap-x-3 gap-y-1.5 text-xs", children: [
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Account" }),
        /* @__PURE__ */ jsx("span", { className: "font-mono", children: email }),
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Your space" }),
        /* @__PURE__ */ jsx("span", { className: "flex items-center gap-1.5", children: /* @__PURE__ */ jsx(Badge, { variant: "secondary", children: result?.team_slug ?? "—" }) }),
        /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Dashboard" }),
        /* @__PURE__ */ jsx("span", { className: "font-mono", children: result?.dashboard_slug ?? "—" })
      ] }),
      /* @__PURE__ */ jsx(Separator, {}),
      /* @__PURE__ */ jsxs("p", { className: "flex items-start gap-2 text-xs text-muted-foreground", children: [
        /* @__PURE__ */ jsx(ShieldCheck, { className: "mt-0.5 size-3.5 shrink-0" }),
        "Access is scoped to you: log in as",
        " ",
        /* @__PURE__ */ jsx("span", { className: "font-mono text-foreground", children: email }),
        " and your sidebar shows only “My devices” — just this one device, nothing else in the tenant."
      ] })
    ] }),
    /* @__PURE__ */ jsx(CardFooter, { children: /* @__PURE__ */ jsx("a", { href: dashUrl, className: "w-full", children: /* @__PURE__ */ jsxs(Button, { className: "w-full", children: [
      "Open my dashboard ",
      /* @__PURE__ */ jsx(ArrowRight, {})
    ] }) }) })
  ] });
}
function StepDots({ step }) {
  const order = ["buy", "signup", "addDevice", "ready"];
  const idx = order.indexOf(step);
  return /* @__PURE__ */ jsx("div", { className: "flex items-center gap-1", children: order.map((s, i) => /* @__PURE__ */ jsx(
    "span",
    {
      className: `h-1.5 rounded-full transition-all ${i <= idx ? "w-4 bg-primary" : "w-1.5 bg-muted-foreground/30"}`
    },
    s
  )) });
}
function friendly(e) {
  const msg = e instanceof Error ? e.message : String(e);
  if (/409|conflict|already/i.test(msg)) {
    return "That email is already registered. Try logging in, or use a different email.";
  }
  if (/password/i.test(msg) && /short|length|8/i.test(msg)) {
    return "Password must be at least 8 characters.";
  }
  return msg;
}

const EXTENSION_ID$2 = "com.acme.devices";
function DevicesDashboard() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(
    "div",
    {
      "data-ext-id": EXTENSION_ID$2,
      className: "mx-auto flex max-w-5xl flex-col gap-5 p-1",
      children: /* @__PURE__ */ jsx(DashboardInner, {})
    }
  ) });
}
function DashboardInner() {
  const client = useHostClient();
  const [rows, setRows] = React.useState([]);
  const [loading, setLoading] = React.useState(false);
  const [err, setErr] = React.useState(null);
  const load = React.useCallback(() => {
    setLoading(true);
    setErr(null);
    fetchJson(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ kind: "com.acme.devices.devices_list", params: {} })
    }).then((r) => setRows(Array.isArray(r.rows) ? r.rows : [])).catch((e) => setErr(friendlyError$1(e))).finally(() => setLoading(false));
  }, [client]);
  React.useEffect(() => {
    load();
  }, [load]);
  const stats = React.useMemo(() => summarize(rows), [rows]);
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsxs("div", { className: "flex items-start justify-between gap-4", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-1.5", children: [
        /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: "Acme Devices · Fleet overview" }),
        /* @__PURE__ */ jsxs("h1", { className: "flex items-center gap-2 text-2xl font-semibold tracking-tight", children: [
          /* @__PURE__ */ jsx(LayoutDashboard, { className: "size-6" }),
          " Devices dashboard"
        ] })
      ] }),
      /* @__PURE__ */ jsxs(
        Button,
        {
          type: "button",
          size: "sm",
          variant: "outline",
          onClick: load,
          disabled: loading,
          title: "Reload from the server",
          children: [
            /* @__PURE__ */ jsx(RefreshCw, { className: loading ? "animate-spin" : "" }),
            " Refresh"
          ]
        }
      )
    ] }),
    /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-1 gap-4 sm:grid-cols-3", children: [
      /* @__PURE__ */ jsx(
        StatCard,
        {
          icon: /* @__PURE__ */ jsx(Boxes, { className: "size-4" }),
          label: "Devices",
          value: String(stats.total),
          hint: "Persisted in the nexus DB"
        }
      ),
      /* @__PURE__ */ jsx(
        StatCard,
        {
          icon: /* @__PURE__ */ jsx(Users, { className: "size-4" }),
          label: "Teams",
          value: String(stats.teams.length),
          hint: stats.teams.length ? stats.teams.join(", ") : "tenant-wide only"
        }
      ),
      /* @__PURE__ */ jsx(
        StatCard,
        {
          icon: /* @__PURE__ */ jsx(MapPin, { className: "size-4" }),
          label: "Locations",
          value: String(stats.locations.length),
          hint: stats.locations.length ? `${stats.locations.length} distinct` : "none recorded"
        }
      )
    ] }),
    /* @__PURE__ */ jsxs(Card, { children: [
      /* @__PURE__ */ jsxs(CardHeader, { children: [
        /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2 text-base", children: [
          /* @__PURE__ */ jsx(Table2, { className: "size-4" }),
          " All devices"
        ] }),
        /* @__PURE__ */ jsxs(CardDescription, { children: [
          "Read from the extension's own nexus table (",
          /* @__PURE__ */ jsx("code", { className: "font-mono", children: "com_acme_devices__devices" }),
          ") via the",
          " ",
          /* @__PURE__ */ jsx("code", { className: "font-mono", children: "devices_list" }),
          " kind — scoped to your tenant & team by the host's un-spoofable identity tokens."
        ] })
      ] }),
      /* @__PURE__ */ jsx(CardContent, { children: err ? /* @__PURE__ */ jsx("div", { className: "rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive", children: err }) : rows.length === 0 ? /* @__PURE__ */ jsx("p", { className: "py-8 text-center text-sm text-muted-foreground", children: loading ? "Loading…" : "No devices yet — provision one on the “Provision device” page." }) : /* @__PURE__ */ jsx("div", { className: "overflow-x-auto", children: /* @__PURE__ */ jsxs("table", { className: "w-full border-collapse text-sm", children: [
        /* @__PURE__ */ jsx("thead", { children: /* @__PURE__ */ jsxs("tr", { className: "border-b text-left text-xs text-muted-foreground", children: [
          /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "device_id" }),
          /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "barcode" }),
          /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "location" }),
          /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "owner" }),
          /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "team" })
        ] }) }),
        /* @__PURE__ */ jsx("tbody", { children: rows.map((d, i) => /* @__PURE__ */ jsxs("tr", { className: "border-b last:border-0", children: [
          /* @__PURE__ */ jsx("td", { className: "py-2 pr-4 font-mono text-xs", children: d.device_id ?? "—" }),
          /* @__PURE__ */ jsx("td", { className: "py-2 pr-4 font-mono text-xs", children: d.barcode ?? "—" }),
          /* @__PURE__ */ jsx("td", { className: "py-2 pr-4", children: d.location || "—" }),
          /* @__PURE__ */ jsx("td", { className: "py-2 pr-4 font-mono text-xs", children: d.owner ? `${d.owner.slice(0, 8)}…` : "—" }),
          /* @__PURE__ */ jsx("td", { className: "py-2 pr-4", children: d.team ? /* @__PURE__ */ jsx(Badge, { variant: "secondary", children: d.team }) : /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "tenant-wide" }) })
        ] }, d.device_id ?? i)) })
      ] }) }) })
    ] })
  ] });
}
function StatCard({
  icon,
  label,
  value,
  hint
}) {
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { className: "pb-2", children: [
      /* @__PURE__ */ jsxs(CardDescription, { className: "flex items-center gap-1.5", children: [
        icon,
        " ",
        label
      ] }),
      /* @__PURE__ */ jsx(CardTitle, { className: "text-3xl tabular-nums", children: value })
    ] }),
    /* @__PURE__ */ jsx(CardContent, { children: /* @__PURE__ */ jsx("p", { className: "truncate text-xs text-muted-foreground", title: hint, children: hint }) })
  ] });
}
function summarize(rows) {
  const teams = /* @__PURE__ */ new Set();
  const locations = /* @__PURE__ */ new Set();
  for (const r of rows) {
    if (r.team) teams.add(r.team);
    if (r.location) locations.add(r.location);
  }
  return {
    total: rows.length,
    teams: [...teams].sort(),
    locations: [...locations].sort()
  };
}
function friendlyError$1(e) {
  const msg = e instanceof Error ? e.message : String(e);
  if (/403|forbidden|csrf/i.test(msg)) {
    return `${msg} — if this is a CSRF error, reload to refresh your session token.`;
  }
  return msg;
}

function Progress({
  value = 0,
  className
}) {
  const pct = Math.max(0, Math.min(100, value));
  return /* @__PURE__ */ jsx(
    "div",
    {
      role: "progressbar",
      "aria-valuemin": 0,
      "aria-valuemax": 100,
      "aria-valuenow": pct,
      className: cn(
        "relative h-2 w-full overflow-hidden rounded-full bg-secondary",
        className
      ),
      children: /* @__PURE__ */ jsx(
        "div",
        {
          className: "h-full rounded-full bg-primary transition-[width] duration-500 ease-out",
          style: { width: `${pct}%` }
        }
      )
    }
  );
}

const EXTENSION_ID$1 = "com.acme.devices";
const TEMPLATE_ID = "com.acme.add-device";
const STEPS = [
  {
    node: "com.acme.create-device",
    title: "Create device",
    detail: "Provision a device record from the barcode (idempotent on barcode)."
  },
  {
    node: "com.acme.register-sensor",
    title: "Register sensor",
    detail: "Attach the device's sensor (idempotent on device id)."
  }
];
function stableId(prefix, key) {
  let hash = 0xcbf29ce484222325n;
  for (const b of new TextEncoder().encode(key)) {
    hash ^= BigInt(b);
    hash = hash * 0x100000001b3n & 0xffffffffffffffffn;
  }
  return `${prefix}-${hash.toString(16).padStart(16, "0")}`;
}
function randomBarcode() {
  const part = () => Math.random().toString(36).slice(2, 6).toUpperCase().padEnd(4, "0");
  return `ACME-${part()}-${part()}`;
}
function DevicesPanel() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(
    "div",
    {
      "data-ext-id": EXTENSION_ID$1,
      className: "mx-auto flex max-w-3xl flex-col gap-5 p-1",
      children: /* @__PURE__ */ jsx(PageInner, {})
    }
  ) });
}
function PageInner() {
  const client = useHostClient();
  const [barcode, setBarcode] = React.useState(() => randomBarcode());
  const [location, setLocation] = React.useState("Roof AHU-3");
  const [runId, setRunId] = React.useState(null);
  const [status, setStatus] = React.useState("idle");
  const [step, setStep] = React.useState(null);
  const [done, setDone] = React.useState(0);
  const [error, setError] = React.useState(null);
  const [resumable, setResumable] = React.useState(false);
  const [snap, setSnap] = React.useState(null);
  const esRef = React.useRef(null);
  const pollRef = React.useRef(null);
  const stopPoll = React.useCallback(() => {
    if (pollRef.current) clearInterval(pollRef.current);
    pollRef.current = null;
  }, []);
  const closeStream = React.useCallback(() => {
    esRef.current?.close();
    esRef.current = null;
  }, []);
  const refreshSnapshot = React.useCallback(
    async (id) => {
      try {
        const s = await fetchJson(
          client,
          `${client.apiPrefix}/setup/runs/${id}`
        );
        setSnap(s);
        if (typeof s.progress?.done === "number") setDone(s.progress.done);
        if (s.progress?.current_step) setStep(s.progress.current_step);
        if (s.status === "completed") {
          setStatus("completed");
          closeStream();
          stopPoll();
        } else if (s.status === "failed") {
          setStatus("failed");
          setResumable(s.resumable ?? true);
          closeStream();
          stopPoll();
        } else if (s.status === "cancelled") {
          setStatus("cancelled");
          closeStream();
          stopPoll();
        }
      } catch {
      }
    },
    [client, closeStream, stopPoll]
  );
  const openStream = React.useCallback(
    (id) => {
      closeStream();
      const url = `${client.apiPrefix}/setup/runs/${id}/events`;
      const es = new EventSource(url, { withCredentials: true });
      esRef.current = es;
      es.onmessage = (raw) => {
        let ev;
        try {
          ev = JSON.parse(raw.data);
        } catch {
          return;
        }
        if (ev.current_step) setStep(ev.current_step);
        if (typeof ev.total === "number") {
          void refreshSnapshot(id);
        }
        if (ev.event === "failed") {
          setStatus("failed");
          setError(ev.error ?? "step failed");
          setResumable(ev.resumable ?? true);
          closeStream();
          stopPoll();
        } else if (ev.event === "completed" || ev.status === "completed") {
          setStatus("completed");
          setDone(STEPS.length);
          void refreshSnapshot(id);
          closeStream();
          stopPoll();
        } else if (ev.event === "cancelled") {
          setStatus("cancelled");
          closeStream();
          stopPoll();
        }
      };
      es.onerror = () => {
        closeStream();
      };
    },
    [client, closeStream, refreshSnapshot, stopPoll]
  );
  const startPoll = React.useCallback(
    (id) => {
      stopPoll();
      void refreshSnapshot(id);
      pollRef.current = setInterval(() => void refreshSnapshot(id), 600);
    },
    [refreshSnapshot, stopPoll]
  );
  React.useEffect(
    () => () => {
      closeStream();
      stopPoll();
    },
    [closeStream, stopPoll]
  );
  const launch = React.useCallback(() => {
    setError(null);
    setStatus("running");
    setStep(null);
    setDone(0);
    setSnap(null);
    setResumable(false);
    fetchJson(
      client,
      `${client.apiPrefix}/setup/templates/${TEMPLATE_ID}/run`,
      {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ barcode, location })
      }
    ).then((r) => {
      setRunId(r.run_id);
      openStream(r.run_id);
      startPoll(r.run_id);
    }).catch((e) => {
      setStatus("failed");
      setError(friendlyError(e));
    });
  }, [client, barcode, location, openStream, startPoll]);
  const resume = React.useCallback(() => {
    if (!runId) return;
    setError(null);
    setStatus("running");
    setResumable(false);
    fetchJson(
      client,
      `${client.apiPrefix}/setup/runs/${runId}/resume`,
      { method: "POST" }
    ).then(() => {
      openStream(runId);
      startPoll(runId);
    }).catch((e) => {
      setStatus("failed");
      setError(friendlyError(e));
    });
  }, [client, runId, openStream, startPoll]);
  const cancel = React.useCallback(() => {
    if (!runId) return;
    fetchJson(
      client,
      `${client.apiPrefix}/setup/runs/${runId}/cancel`,
      { method: "POST" }
    ).then(() => {
      setStatus("cancelled");
      closeStream();
      stopPoll();
      if (runId) void refreshSnapshot(runId);
    }).catch((e) => setError(friendlyError(e)));
  }, [client, runId, closeStream, stopPoll, refreshSnapshot]);
  const reset = React.useCallback(() => {
    closeStream();
    stopPoll();
    setRunId(null);
    setStatus("idle");
    setStep(null);
    setDone(0);
    setError(null);
    setResumable(false);
    setSnap(null);
    setBarcode(randomBarcode());
  }, [closeStream, stopPoll]);
  const busy = status === "running";
  const deviceId = barcode ? stableId("dev", barcode) : "";
  const sensorId = deviceId ? stableId("sen", deviceId) : "";
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsxs("div", { className: "flex items-start justify-between gap-4", children: [
      /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-1.5", children: [
        /* @__PURE__ */ jsx("p", { className: "text-sm text-muted-foreground", children: "Acme Devices · Setup automation demo" }),
        /* @__PURE__ */ jsxs("h1", { className: "flex items-center gap-2 text-2xl font-semibold tracking-tight", children: [
          /* @__PURE__ */ jsx(Cpu, { className: "size-6" }),
          " Provision a device"
        ] })
      ] }),
      /* @__PURE__ */ jsx(StatusBadge, { status })
    ] }),
    /* @__PURE__ */ jsx(Card, { className: "border-dashed bg-muted/30", children: /* @__PURE__ */ jsxs(CardContent, { className: "flex gap-3 pt-6 text-sm", children: [
      /* @__PURE__ */ jsx(Info, { className: "mt-0.5 size-4 shrink-0 text-muted-foreground" }),
      /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-1.5 text-muted-foreground", children: [
        /* @__PURE__ */ jsxs("p", { children: [
          /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "What's happening here?" }),
          " ",
          "This simulates a field technician scanning a new device's",
          " ",
          /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "box barcode" }),
          " to set it up. The barcode is just an identifier — provisioning runs a small multi-step ",
          /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "automation" }),
          " ",
          "on the server."
        ] }),
        /* @__PURE__ */ jsxs("p", { children: [
          "You don't have a real scanner, so use the sample barcode below (or",
          " ",
          /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "Randomize" }),
          " a new one), then press ",
          /* @__PURE__ */ jsx("span", { className: "font-medium text-foreground", children: "Provision" }),
          ". The same barcode always provisions the same device — that's the idempotency the automation guarantees."
        ] })
      ] })
    ] }) }),
    /* @__PURE__ */ jsxs(Card, { children: [
      /* @__PURE__ */ jsxs(CardHeader, { children: [
        /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2", children: [
          /* @__PURE__ */ jsx(ScanLine, { className: "size-5" }),
          " Scan to provision"
        ] }),
        /* @__PURE__ */ jsxs(CardDescription, { children: [
          "Runs the ",
          /* @__PURE__ */ jsx("code", { className: "font-mono", children: TEMPLATE_ID }),
          " automation — instant launch, streamed per-step progress, resume from a failed step."
        ] })
      ] }),
      /* @__PURE__ */ jsxs(CardContent, { className: "flex flex-col gap-4", children: [
        /* @__PURE__ */ jsx(Separator, {}),
        /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
          /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground", children: [
            "Device barcode ",
            /* @__PURE__ */ jsx("span", { className: "text-muted-foreground/70", children: "(scanned box label)" })
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "flex gap-2", children: [
            /* @__PURE__ */ jsx(
              "input",
              {
                value: barcode,
                onChange: (e) => setBarcode(e.target.value),
                placeholder: "e.g. ACME-7F3A-9C21",
                disabled: busy,
                className: "h-9 flex-1 rounded-md border border-input bg-transparent px-3 font-mono text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
              }
            ),
            /* @__PURE__ */ jsxs(
              Button,
              {
                type: "button",
                size: "sm",
                variant: "outline",
                onClick: () => setBarcode(randomBarcode()),
                disabled: busy,
                title: "Generate a sample barcode",
                children: [
                  /* @__PURE__ */ jsx(Dice5, {}),
                  " Randomize"
                ]
              }
            )
          ] }),
          deviceId ? /* @__PURE__ */ jsxs("span", { className: "text-xs text-muted-foreground", children: [
            "→ will provision device",
            " ",
            /* @__PURE__ */ jsx("code", { className: "font-mono text-foreground", children: deviceId })
          ] }) : null
        ] }),
        /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1.5 text-sm", children: [
          /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "Install location" }),
          /* @__PURE__ */ jsx(
            "input",
            {
              value: location,
              onChange: (e) => setLocation(e.target.value),
              disabled: busy,
              className: "h-9 rounded-md border border-input bg-transparent px-3 text-sm outline-none ring-ring focus-visible:ring-2 disabled:opacity-60"
            }
          )
        ] })
      ] }),
      /* @__PURE__ */ jsxs(CardFooter, { className: "flex-wrap gap-2", children: [
        /* @__PURE__ */ jsxs(Button, { onClick: launch, disabled: !barcode || busy, children: [
          busy ? /* @__PURE__ */ jsx(LoaderCircle, { className: "animate-spin" }) : /* @__PURE__ */ jsx(ScanLine, {}),
          busy ? "Provisioning…" : "Provision"
        ] }),
        status === "failed" && resumable ? /* @__PURE__ */ jsxs(Button, { variant: "default", onClick: resume, children: [
          /* @__PURE__ */ jsx(RotateCcw, {}),
          " Retry from failed step"
        ] }) : null,
        busy ? /* @__PURE__ */ jsxs(Button, { variant: "outline", onClick: cancel, children: [
          /* @__PURE__ */ jsx(Ban, {}),
          " Cancel"
        ] }) : null,
        runId && !busy ? /* @__PURE__ */ jsx(Button, { variant: "ghost", onClick: reset, children: "Provision another" }) : null
      ] })
    ] }),
    runId ? /* @__PURE__ */ jsxs(Card, { children: [
      /* @__PURE__ */ jsxs(CardHeader, { children: [
        /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between gap-3", children: [
          /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2 text-base", children: [
            /* @__PURE__ */ jsx(Radio, { className: "size-4" }),
            " Run progress"
          ] }),
          /* @__PURE__ */ jsxs("span", { className: "text-xs tabular-nums text-muted-foreground", children: [
            done,
            "/",
            STEPS.length,
            " steps"
          ] })
        ] }),
        /* @__PURE__ */ jsxs(CardDescription, { children: [
          "run ",
          /* @__PURE__ */ jsx("code", { className: "font-mono", children: runId })
        ] })
      ] }),
      /* @__PURE__ */ jsxs(CardContent, { className: "flex flex-col gap-4", children: [
        /* @__PURE__ */ jsx(Progress, { value: done / STEPS.length * 100 }),
        /* @__PURE__ */ jsx("ol", { className: "flex flex-col gap-1", children: STEPS.map((s, i) => /* @__PURE__ */ jsx(
          StepRow,
          {
            index: i,
            title: s.title,
            detail: s.detail,
            state: stepState(i, done, step, s.node, status)
          },
          s.node
        )) }),
        error ? /* @__PURE__ */ jsx("div", { className: "rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive", children: error }) : null,
        status === "completed" ? /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-2 rounded-md border border-emerald-600/30 bg-emerald-600/10 p-3 text-sm", children: [
          /* @__PURE__ */ jsxs("p", { className: "flex items-center gap-2 font-medium text-emerald-700 dark:text-emerald-400", children: [
            /* @__PURE__ */ jsx(CircleCheck, { className: "size-4" }),
            " Device provisioned"
          ] }),
          /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1 font-mono text-xs text-foreground", children: [
            /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "device_id" }),
            /* @__PURE__ */ jsx("span", { children: deviceId }),
            /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "sensor_id" }),
            /* @__PURE__ */ jsx("span", { children: sensorId }),
            /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "location" }),
            /* @__PURE__ */ jsx("span", { className: "font-sans", children: location })
          ] })
        ] }) : null
      ] }),
      snap ? /* @__PURE__ */ jsxs(CardFooter, { className: "flex-col items-start gap-2 border-t pt-4", children: [
        /* @__PURE__ */ jsxs("p", { className: "flex items-center gap-1.5 text-xs font-medium text-muted-foreground", children: [
          /* @__PURE__ */ jsx(ShieldCheck, { className: "size-3.5" }),
          " Trusted identity — seeded by the server from your session, not this form"
        ] }),
        /* @__PURE__ */ jsxs("div", { className: "flex flex-wrap gap-1.5", children: [
          snap.team ? /* @__PURE__ */ jsxs(Badge, { variant: "secondary", children: [
            "team: ",
            snap.team
          ] }) : null,
          snap.tenant_id ? /* @__PURE__ */ jsxs(Badge, { variant: "secondary", children: [
            "tenant: ",
            snap.tenant_id
          ] }) : null,
          snap.owner ? /* @__PURE__ */ jsxs(Badge, { variant: "outline", className: "font-mono", children: [
            "owner: ",
            snap.owner.slice(0, 8),
            "…"
          ] }) : null
        ] })
      ] }) : null
    ] }) : null,
    /* @__PURE__ */ jsx(DevicesTable, { reloadKey: status === "completed" ? runId : null })
  ] });
}
function DevicesTable({ reloadKey }) {
  const client = useHostClient();
  const [rows, setRows] = React.useState([]);
  const [loading, setLoading] = React.useState(false);
  const [err, setErr] = React.useState(null);
  const load = React.useCallback(() => {
    setLoading(true);
    setErr(null);
    fetchJson(client, `${client.apiPrefix}/query`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ kind: "com.acme.devices.devices_list", params: {} })
    }).then((r) => setRows(Array.isArray(r.rows) ? r.rows : [])).catch((e) => setErr(friendlyError(e))).finally(() => setLoading(false));
  }, [client]);
  React.useEffect(() => {
    load();
  }, [load, reloadKey]);
  return /* @__PURE__ */ jsxs(Card, { children: [
    /* @__PURE__ */ jsxs(CardHeader, { children: [
      /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-between gap-3", children: [
        /* @__PURE__ */ jsxs(CardTitle, { className: "flex items-center gap-2 text-base", children: [
          /* @__PURE__ */ jsx(Table2, { className: "size-4" }),
          " Provisioned devices"
        ] }),
        /* @__PURE__ */ jsxs(
          Button,
          {
            type: "button",
            size: "sm",
            variant: "ghost",
            onClick: load,
            disabled: loading,
            title: "Reload from the server",
            children: [
              /* @__PURE__ */ jsx(RefreshCw, { className: loading ? "animate-spin" : "" }),
              " Refresh"
            ]
          }
        )
      ] }),
      /* @__PURE__ */ jsxs(CardDescription, { children: [
        "Persisted in the extension's own nexus table (",
        /* @__PURE__ */ jsx("code", { className: "font-mono", children: "com_acme_devices__devices" }),
        "), read back via the ",
        /* @__PURE__ */ jsx("code", { className: "font-mono", children: "devices_list" }),
        " kind — scoped to your tenant & team."
      ] })
    ] }),
    /* @__PURE__ */ jsx(CardContent, { children: err ? /* @__PURE__ */ jsx("div", { className: "rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive", children: err }) : rows.length === 0 ? /* @__PURE__ */ jsx("p", { className: "py-6 text-center text-sm text-muted-foreground", children: loading ? "Loading…" : "No devices yet — provision one above." }) : /* @__PURE__ */ jsx("div", { className: "overflow-x-auto", children: /* @__PURE__ */ jsxs("table", { className: "w-full border-collapse text-sm", children: [
      /* @__PURE__ */ jsx("thead", { children: /* @__PURE__ */ jsxs("tr", { className: "border-b text-left text-xs text-muted-foreground", children: [
        /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "device_id" }),
        /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "barcode" }),
        /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "location" }),
        /* @__PURE__ */ jsx("th", { className: "py-2 pr-4 font-medium", children: "team" })
      ] }) }),
      /* @__PURE__ */ jsx("tbody", { children: rows.map((d, i) => /* @__PURE__ */ jsxs("tr", { className: "border-b last:border-0", children: [
        /* @__PURE__ */ jsx("td", { className: "py-2 pr-4 font-mono text-xs", children: d.device_id ?? "—" }),
        /* @__PURE__ */ jsx("td", { className: "py-2 pr-4 font-mono text-xs", children: d.barcode ?? "—" }),
        /* @__PURE__ */ jsx("td", { className: "py-2 pr-4", children: d.location || "—" }),
        /* @__PURE__ */ jsx("td", { className: "py-2 pr-4", children: d.team ? /* @__PURE__ */ jsx(Badge, { variant: "secondary", children: d.team }) : /* @__PURE__ */ jsx("span", { className: "text-muted-foreground", children: "tenant-wide" }) })
      ] }, d.device_id ?? i)) })
    ] }) }) })
  ] });
}
function stepState(index, done, currentStep, node, status) {
  if (index < done) return "done";
  if (status === "failed" && (currentStep === node || index === done)) return "failed";
  if (status === "running" && (currentStep === node || index === done)) return "running";
  if (status === "completed") return "done";
  return "pending";
}
function StepRow({
  index,
  title,
  detail,
  state
}) {
  return /* @__PURE__ */ jsxs("li", { className: "flex items-start gap-3 rounded-md px-2 py-2", children: [
    /* @__PURE__ */ jsx("span", { className: "mt-0.5", children: /* @__PURE__ */ jsx(StepIcon, { state }) }),
    /* @__PURE__ */ jsxs("div", { className: "flex flex-col", children: [
      /* @__PURE__ */ jsxs("span", { className: "flex items-center gap-1.5 text-sm font-medium", children: [
        /* @__PURE__ */ jsxs("span", { className: "text-muted-foreground", children: [
          index + 1,
          "."
        ] }),
        " ",
        title,
        state === "running" ? /* @__PURE__ */ jsx("span", { className: "text-xs font-normal text-muted-foreground", children: "running…" }) : null
      ] }),
      /* @__PURE__ */ jsx("span", { className: "text-xs text-muted-foreground", children: detail })
    ] })
  ] });
}
function StepIcon({ state }) {
  switch (state) {
    case "done":
      return /* @__PURE__ */ jsx(CircleCheck, { className: "size-4 text-emerald-600 dark:text-emerald-400" });
    case "running":
      return /* @__PURE__ */ jsx(LoaderCircle, { className: "size-4 animate-spin text-primary" });
    case "failed":
      return /* @__PURE__ */ jsx(CircleX, { className: "size-4 text-destructive" });
    default:
      return /* @__PURE__ */ jsx(Circle, { className: "size-4 text-muted-foreground/40" });
  }
}
function StatusBadge({ status }) {
  switch (status) {
    case "completed":
      return /* @__PURE__ */ jsxs(Badge, { variant: "success", children: [
        /* @__PURE__ */ jsx(CircleCheck, {}),
        " Done"
      ] });
    case "failed":
      return /* @__PURE__ */ jsxs(Badge, { variant: "destructive", children: [
        /* @__PURE__ */ jsx(CircleX, {}),
        " Failed"
      ] });
    case "running":
      return /* @__PURE__ */ jsxs(Badge, { variant: "secondary", children: [
        /* @__PURE__ */ jsx(LoaderCircle, { className: "animate-spin" }),
        " Running"
      ] });
    case "cancelled":
      return /* @__PURE__ */ jsxs(Badge, { variant: "outline", children: [
        /* @__PURE__ */ jsx(Ban, {}),
        " Cancelled"
      ] });
    default:
      return /* @__PURE__ */ jsxs(Badge, { variant: "outline", children: [
        /* @__PURE__ */ jsx(ChevronRight, {}),
        " Ready"
      ] });
  }
}
function friendlyError(e) {
  const msg = e instanceof Error ? e.message : String(e);
  if (/team/i.test(msg) && /(allowed|forbidden|403)/i.test(msg)) {
    return "Forbidden: your user isn't in a team this template allows (allowed_teams: hvac-ops). Add yourself to that team and retry.";
  }
  if (/403|forbidden|csrf/i.test(msg)) {
    return `${msg} — if this is a CSRF error, reload to refresh your session token.`;
  }
  return msg;
}

function Main() {
  const { route } = useSlotContext();
  const head = (route ?? "").replace(/^\/+/, "").split("/")[0];
  switch (head) {
    case "provision":
      return /* @__PURE__ */ jsx(DevicesPanel, {});
    case "dashboard":
      return /* @__PURE__ */ jsx(DevicesDashboard, {});
    case "":
    case "app":
    default:
      return /* @__PURE__ */ jsx(ConsumerOnboard, {});
  }
}

const EXTENSION_ID = "com.acme.devices";
const BASE = `/x/${EXTENSION_ID}`;
const ITEMS = [
  { href: `${BASE}/app`, icon: "📱", label: "Get started (app)" },
  { href: `${BASE}/dashboard`, icon: "📊", label: "Devices dashboard" },
  { href: `${BASE}/provision`, icon: "🔧", label: "Provision device" }
];
function DevicesNav() {
  return /* @__PURE__ */ jsx("div", { "data-ext-id": EXTENSION_ID, className: "flex flex-col gap-0.5", children: ITEMS.map((it) => /* @__PURE__ */ jsxs(
    "a",
    {
      href: it.href,
      className: "flex h-8 items-center gap-2 rounded-md px-2 text-sm text-sidebar-foreground/80 outline-none ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2",
      children: [
        /* @__PURE__ */ jsx(
          "span",
          {
            "aria-hidden": true,
            className: "grid size-5 shrink-0 place-items-center rounded bg-primary/15 text-primary",
            children: it.icon
          }
        ),
        /* @__PURE__ */ jsx("span", { className: "truncate", children: it.label })
      ]
    },
    it.href
  )) });
}

const factory = {
  singletons: {
    react: { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { DevicesMain: Main, DevicesNav }
    });
  }
};

export { factory as default };
