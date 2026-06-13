(function () {
	'use strict';

	try{if(typeof document != 'undefined'){var elementStyle = document.createElement('style');elementStyle.appendChild(document.createTextNode("/*! tailwindcss v4.3.0 | MIT License | https://tailwindcss.com */\n@layer properties {\n  @supports (((-webkit-hyphens: none)) and (not (margin-trim: inline))) or ((-moz-orient: inline) and (not (color: rgb(from red r g b)))) {\n    *, [data-ext-id=\"com.nubeio.ce\"] :before, [data-ext-id=\"com.nubeio.ce\"]:before, [data-ext-id=\"com.nubeio.ce\"] :after, [data-ext-id=\"com.nubeio.ce\"]:after, [data-ext-id=\"com.nubeio.ce\"] ::backdrop, [data-ext-id=\"com.nubeio.ce\"]::backdrop {\n      --tw-border-style: solid;\n      --tw-font-weight: initial;\n      --tw-shadow: 0 0 #0000;\n      --tw-shadow-color: initial;\n      --tw-shadow-alpha: 100%;\n      --tw-inset-shadow: 0 0 #0000;\n      --tw-inset-shadow-color: initial;\n      --tw-inset-shadow-alpha: 100%;\n      --tw-ring-color: initial;\n      --tw-ring-shadow: 0 0 #0000;\n      --tw-inset-ring-color: initial;\n      --tw-inset-ring-shadow: 0 0 #0000;\n      --tw-ring-inset: initial;\n      --tw-ring-offset-width: 0px;\n      --tw-ring-offset-color: #fff;\n      --tw-ring-offset-shadow: 0 0 #0000;\n    }\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .absolute, [data-ext-id=\"com.nubeio.ce\"].absolute {\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .fixed, [data-ext-id=\"com.nubeio.ce\"].fixed {\n  position: fixed;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .relative, [data-ext-id=\"com.nubeio.ce\"].relative {\n  position: relative;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .z-10, [data-ext-id=\"com.nubeio.ce\"].z-10 {\n  z-index: 10;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .block, [data-ext-id=\"com.nubeio.ce\"].block {\n  display: block;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .flex, [data-ext-id=\"com.nubeio.ce\"].flex {\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .grid, [data-ext-id=\"com.nubeio.ce\"].grid {\n  display: grid;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .inline-flex, [data-ext-id=\"com.nubeio.ce\"].inline-flex {\n  display: inline-flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .size-4, [data-ext-id=\"com.nubeio.ce\"].size-4 {\n  width: calc(var(--spacing, .25rem) * 4);\n  height: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .h-8, [data-ext-id=\"com.nubeio.ce\"].h-8 {\n  height: calc(var(--spacing, .25rem) * 8);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .w-full, [data-ext-id=\"com.nubeio.ce\"].w-full {\n  width: 100%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .max-w-xl, [data-ext-id=\"com.nubeio.ce\"].max-w-xl {\n  max-width: var(--container-xl, 36rem);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .min-w-0, [data-ext-id=\"com.nubeio.ce\"].min-w-0 {\n  min-width: calc(var(--spacing, .25rem) * 0);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .shrink-0, [data-ext-id=\"com.nubeio.ce\"].shrink-0 {\n  flex-shrink: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .grid-cols-\\[1fr_120px\\], [data-ext-id=\"com.nubeio.ce\"].grid-cols-\\[1fr_120px\\] {\n  grid-template-columns: 1fr 120px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .flex-col, [data-ext-id=\"com.nubeio.ce\"].flex-col {\n  flex-direction: column;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .items-center, [data-ext-id=\"com.nubeio.ce\"].items-center {\n  align-items: center;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .items-end, [data-ext-id=\"com.nubeio.ce\"].items-end {\n  align-items: flex-end;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .justify-between, [data-ext-id=\"com.nubeio.ce\"].justify-between {\n  justify-content: space-between;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .justify-end, [data-ext-id=\"com.nubeio.ce\"].justify-end {\n  justify-content: flex-end;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-0\\.5, [data-ext-id=\"com.nubeio.ce\"].gap-0\\.5 {\n  gap: calc(var(--spacing, .25rem) * .5);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-1, [data-ext-id=\"com.nubeio.ce\"].gap-1 {\n  gap: calc(var(--spacing, .25rem) * 1);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-1\\.5, [data-ext-id=\"com.nubeio.ce\"].gap-1\\.5 {\n  gap: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-2, [data-ext-id=\"com.nubeio.ce\"].gap-2 {\n  gap: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-3, [data-ext-id=\"com.nubeio.ce\"].gap-3 {\n  gap: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .gap-4, [data-ext-id=\"com.nubeio.ce\"].gap-4 {\n  gap: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .overflow-hidden, [data-ext-id=\"com.nubeio.ce\"].overflow-hidden {\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .border, [data-ext-id=\"com.nubeio.ce\"].border {\n  border-style: var(--tw-border-style);\n  border-width: 1px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .border-t, [data-ext-id=\"com.nubeio.ce\"].border-t {\n  border-top-style: var(--tw-border-style);\n  border-top-width: 1px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .p-2, [data-ext-id=\"com.nubeio.ce\"].p-2 {\n  padding: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .p-4, [data-ext-id=\"com.nubeio.ce\"].p-4 {\n  padding: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .px-2, [data-ext-id=\"com.nubeio.ce\"].px-2 {\n  padding-inline: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .px-2\\.5, [data-ext-id=\"com.nubeio.ce\"].px-2\\.5 {\n  padding-inline: calc(var(--spacing, .25rem) * 2.5);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .px-3, [data-ext-id=\"com.nubeio.ce\"].px-3 {\n  padding-inline: calc(var(--spacing, .25rem) * 3);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .py-1\\.5, [data-ext-id=\"com.nubeio.ce\"].py-1\\.5 {\n  padding-block: calc(var(--spacing, .25rem) * 1.5);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .py-2, [data-ext-id=\"com.nubeio.ce\"].py-2 {\n  padding-block: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .py-6, [data-ext-id=\"com.nubeio.ce\"].py-6 {\n  padding-block: calc(var(--spacing, .25rem) * 6);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .pt-2, [data-ext-id=\"com.nubeio.ce\"].pt-2 {\n  padding-top: calc(var(--spacing, .25rem) * 2);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-center, [data-ext-id=\"com.nubeio.ce\"].text-center {\n  text-align: center;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-left, [data-ext-id=\"com.nubeio.ce\"].text-left {\n  text-align: left;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-start, [data-ext-id=\"com.nubeio.ce\"].text-start {\n  text-align: start;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-base, [data-ext-id=\"com.nubeio.ce\"].text-base {\n  font-size: var(--text-base, 1rem);\n  line-height: var(--tw-leading, var(--text-base--line-height, calc(1.5 / 1)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-sm, [data-ext-id=\"com.nubeio.ce\"].text-sm {\n  font-size: var(--text-sm, .875rem);\n  line-height: var(--tw-leading, var(--text-sm--line-height, calc(1.25 / .875)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-xl, [data-ext-id=\"com.nubeio.ce\"].text-xl {\n  font-size: var(--text-xl, 1.25rem);\n  line-height: var(--tw-leading, var(--text-xl--line-height, calc(1.75 / 1.25)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .text-xs, [data-ext-id=\"com.nubeio.ce\"].text-xs {\n  font-size: var(--text-xs, .75rem);\n  line-height: var(--tw-leading, var(--text-xs--line-height, calc(1 / .75)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .font-medium, [data-ext-id=\"com.nubeio.ce\"].font-medium {\n  --tw-font-weight: var(--font-weight-medium, 500);\n  font-weight: var(--font-weight-medium, 500);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .font-semibold, [data-ext-id=\"com.nubeio.ce\"].font-semibold {\n  --tw-font-weight: var(--font-weight-semibold, 600);\n  font-weight: var(--font-weight-semibold, 600);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .no-underline, [data-ext-id=\"com.nubeio.ce\"].no-underline {\n  text-decoration-line: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .outline-hidden, [data-ext-id=\"com.nubeio.ce\"].outline-hidden {\n  --tw-outline-style: none;\n  outline-style: none;\n}\n\n@media (forced-colors: active) {\n  [data-ext-id=\"com.nubeio.ce\"] .outline-hidden, [data-ext-id=\"com.nubeio.ce\"].outline-hidden {\n    outline-offset: 2px;\n    outline: 2px solid #0000;\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .transition-\\[width\\,height\\,padding\\], [data-ext-id=\"com.nubeio.ce\"].transition-\\[width\\,height\\,padding\\] {\n  transition-property: width, height, padding;\n  transition-timing-function: var(--tw-ease, var(--default-transition-timing-function, cubic-bezier(.4, 0, .2, 1)));\n  transition-duration: var(--tw-duration, var(--default-transition-duration, .15s));\n}\n\n@media (hover: hover) {\n  [data-ext-id=\"com.nubeio.ce\"] .hover\\:underline:hover, [data-ext-id=\"com.nubeio.ce\"].hover\\:underline:hover {\n    text-decoration-line: underline;\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .focus-visible\\:ring-2:focus-visible, [data-ext-id=\"com.nubeio.ce\"].focus-visible\\:ring-2:focus-visible {\n  --tw-ring-shadow: var(--tw-ring-inset, ) 0 0 0 calc(2px + var(--tw-ring-offset-width)) var(--tw-ring-color, currentcolor);\n  box-shadow: var(--tw-inset-shadow), var(--tw-inset-ring-shadow), var(--tw-ring-offset-shadow), var(--tw-ring-shadow), var(--tw-shadow);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .disabled\\:pointer-events-none:disabled, [data-ext-id=\"com.nubeio.ce\"].disabled\\:pointer-events-none:disabled {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .disabled\\:opacity-50:disabled, [data-ext-id=\"com.nubeio.ce\"].disabled\\:opacity-50:disabled {\n  opacity: .5;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .disabled\\:opacity-60:disabled, [data-ext-id=\"com.nubeio.ce\"].disabled\\:opacity-60:disabled {\n  opacity: .6;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .data-\\[active\\=true\\]\\:font-medium[data-active=\"true\"], [data-ext-id=\"com.nubeio.ce\"].data-\\[active\\=true\\]\\:font-medium[data-active=\"true\"] {\n  --tw-font-weight: var(--font-weight-medium, 500);\n  font-weight: var(--font-weight-medium, 500);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .\\[\\&\\>span\\:last-child\\]\\:truncate > span:last-child, [data-ext-id=\"com.nubeio.ce\"].\\[\\&\\>span\\:last-child\\]\\:truncate > span:last-child {\n  text-overflow: ellipsis;\n  white-space: nowrap;\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .\\[\\&\\>svg\\]\\:size-4 > svg, [data-ext-id=\"com.nubeio.ce\"].\\[\\&\\>svg\\]\\:size-4 > svg {\n  width: calc(var(--spacing, .25rem) * 4);\n  height: calc(var(--spacing, .25rem) * 4);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .\\[\\&\\>svg\\]\\:shrink-0 > svg, [data-ext-id=\"com.nubeio.ce\"].\\[\\&\\>svg\\]\\:shrink-0 > svg {\n  flex-shrink: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow, [data-ext-id=\"com.nubeio.ce\"].react-flow {\n  --xy-edge-stroke-default: #b1b1b7;\n  --xy-edge-stroke-width-default: 1;\n  --xy-edge-stroke-selected-default: #555;\n  --xy-connectionline-stroke-default: #b1b1b7;\n  --xy-connectionline-stroke-width-default: 1;\n  --xy-attribution-background-color-default: #ffffff80;\n  --xy-minimap-background-color-default: #fff;\n  --xy-minimap-mask-background-color-default: #f0f0f099;\n  --xy-minimap-mask-stroke-color-default: transparent;\n  --xy-minimap-mask-stroke-width-default: 1;\n  --xy-minimap-node-background-color-default: #e2e2e2;\n  --xy-minimap-node-stroke-color-default: transparent;\n  --xy-minimap-node-stroke-width-default: 2;\n  --xy-background-color-default: transparent;\n  --xy-background-pattern-dots-color-default: #91919a;\n  --xy-background-pattern-lines-color-default: #eee;\n  --xy-background-pattern-cross-color-default: #e2e2e2;\n  background-color: var(--xy-background-color, var(--xy-background-color-default));\n  --xy-node-color-default: inherit;\n  --xy-node-border-default: 1px solid #1a192b;\n  --xy-node-background-color-default: #fff;\n  --xy-node-group-background-color-default: #f0f0f040;\n  --xy-node-boxshadow-hover-default: 0 1px 4px 1px #00000014;\n  --xy-node-boxshadow-selected-default: 0 0 0 .5px #1a192b;\n  --xy-node-border-radius-default: 3px;\n  --xy-handle-background-color-default: #1a192b;\n  --xy-handle-border-color-default: #fff;\n  --xy-selection-background-color-default: #0059dc14;\n  --xy-selection-border-default: 1px dotted #0059dccc;\n  --xy-controls-button-background-color-default: #fefefe;\n  --xy-controls-button-background-color-hover-default: #f4f4f4;\n  --xy-controls-button-color-default: inherit;\n  --xy-controls-button-color-hover-default: inherit;\n  --xy-controls-button-border-color-default: #eee;\n  --xy-controls-box-shadow-default: 0 0 2px 1px #00000014;\n  --xy-edge-label-background-color-default: #fff;\n  --xy-edge-label-color-default: inherit;\n  --xy-resize-background-color-default: #3367d9;\n  direction: ltr;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow.dark, [data-ext-id=\"com.nubeio.ce\"].react-flow.dark {\n  --xy-edge-stroke-default: #3e3e3e;\n  --xy-edge-stroke-width-default: 1;\n  --xy-edge-stroke-selected-default: #727272;\n  --xy-connectionline-stroke-default: #b1b1b7;\n  --xy-connectionline-stroke-width-default: 1;\n  --xy-attribution-background-color-default: #96969640;\n  --xy-minimap-background-color-default: #141414;\n  --xy-minimap-mask-background-color-default: #3c3c3c99;\n  --xy-minimap-mask-stroke-color-default: transparent;\n  --xy-minimap-mask-stroke-width-default: 1;\n  --xy-minimap-node-background-color-default: #2b2b2b;\n  --xy-minimap-node-stroke-color-default: transparent;\n  --xy-minimap-node-stroke-width-default: 2;\n  --xy-background-color-default: #141414;\n  --xy-background-pattern-dots-color-default: #777;\n  --xy-background-pattern-lines-color-default: #777;\n  --xy-background-pattern-cross-color-default: #777;\n  --xy-node-color-default: #f8f8f8;\n  --xy-node-border-default: 1px solid #3c3c3c;\n  --xy-node-background-color-default: #1e1e1e;\n  --xy-node-group-background-color-default: #f0f0f040;\n  --xy-node-boxshadow-hover-default: 0 1px 4px 1px #ffffff14;\n  --xy-node-boxshadow-selected-default: 0 0 0 .5px #999;\n  --xy-handle-background-color-default: #bebebe;\n  --xy-handle-border-color-default: #1e1e1e;\n  --xy-selection-background-color-default: #c8c8dc14;\n  --xy-selection-border-default: 1px dotted #c8c8dccc;\n  --xy-controls-button-background-color-default: #2b2b2b;\n  --xy-controls-button-background-color-hover-default: #3e3e3e;\n  --xy-controls-button-color-default: #f8f8f8;\n  --xy-controls-button-color-hover-default: #fff;\n  --xy-controls-button-border-color-default: #5b5b5b;\n  --xy-controls-box-shadow-default: 0 0 2px 1px #00000014;\n  --xy-edge-label-background-color-default: #141414;\n  --xy-edge-label-color-default: #f8f8f8;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background, [data-ext-id=\"com.nubeio.ce\"].react-flow__background {\n  background-color: var(--xy-background-color-props, var(--xy-background-color, var(--xy-background-color-default)));\n  pointer-events: none;\n  z-index: -1;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__container, [data-ext-id=\"com.nubeio.ce\"].react-flow__container {\n  width: 100%;\n  height: 100%;\n  position: absolute;\n  top: 0;\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane {\n  z-index: 1;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.draggable, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.draggable {\n  cursor: grab;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.dragging, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.dragging {\n  cursor: grabbing;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.selection {\n  cursor: pointer;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__viewport, [data-ext-id=\"com.nubeio.ce\"].react-flow__viewport {\n  transform-origin: 0 0;\n  z-index: 2;\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__renderer, [data-ext-id=\"com.nubeio.ce\"].react-flow__renderer {\n  z-index: 4;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection {\n  z-index: 6;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus-visible {\n  outline: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-path {\n  stroke: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n  stroke-width: var(--xy-edge-stroke-width, var(--xy-edge-stroke-width-default));\n  fill: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection-path {\n  stroke: var(--xy-connectionline-stroke, var(--xy-connectionline-stroke-default));\n  stroke-width: var(--xy-connectionline-stroke-width, var(--xy-connectionline-stroke-width-default));\n  fill: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow .react-flow__edges, [data-ext-id=\"com.nubeio.ce\"].react-flow .react-flow__edges {\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow .react-flow__edges svg, [data-ext-id=\"com.nubeio.ce\"].react-flow .react-flow__edges svg {\n  pointer-events: none;\n  position: absolute;\n  overflow: visible;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge {\n  pointer-events: visibleStroke;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable {\n  cursor: pointer;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.animated path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.animated path {\n  stroke-dasharray: 5;\n  animation: .5s linear infinite dashdraw;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.animated path.react-flow__edge-interaction, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.animated path.react-flow__edge-interaction {\n  stroke-dasharray: none;\n  animation: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.inactive, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.inactive {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge:focus-visible {\n  outline: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selected .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selected .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable:focus .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable:focus .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable:focus-visible .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable:focus-visible .react-flow__edge-path {\n  stroke: var(--xy-edge-stroke-selected, var(--xy-edge-stroke-selected-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-textwrapper, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-textwrapper {\n  pointer-events: all;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge .react-flow__edge-text {\n  pointer-events: none;\n  -webkit-user-select: none;\n  user-select: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__arrowhead polyline, [data-ext-id=\"com.nubeio.ce\"].react-flow__arrowhead polyline {\n  stroke: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__arrowhead polyline.arrowclosed, [data-ext-id=\"com.nubeio.ce\"].react-flow__arrowhead polyline.arrowclosed {\n  fill: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection .animated, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection .animated {\n  stroke-dasharray: 5;\n  animation: .5s linear infinite dashdraw;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] svg.react-flow__connectionline {\n  z-index: 1001;\n  position: absolute;\n  overflow: visible;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodes, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodes {\n  pointer-events: none;\n  transform-origin: 0 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node, [data-ext-id=\"com.nubeio.ce\"].react-flow__node {\n  -webkit-user-select: none;\n  user-select: none;\n  pointer-events: all;\n  transform-origin: 0 0;\n  box-sizing: border-box;\n  cursor: default;\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable {\n  cursor: pointer;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.draggable, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.draggable {\n  cursor: grab;\n  pointer-events: all;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.draggable.dragging, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.draggable.dragging {\n  cursor: grabbing;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection {\n  z-index: 3;\n  transform-origin: 0 0;\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect {\n  pointer-events: all;\n  cursor: grab;\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle {\n  pointer-events: none;\n  background-color: var(--xy-handle-background-color, var(--xy-handle-background-color-default));\n  border: 1px solid var(--xy-handle-border-color, var(--xy-handle-border-color-default));\n  border-radius: 100%;\n  width: 6px;\n  min-width: 5px;\n  height: 6px;\n  min-height: 5px;\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle.connectingfrom, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle.connectingfrom {\n  pointer-events: all;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle.connectionindicator, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle.connectionindicator {\n  pointer-events: all;\n  cursor: crosshair;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-bottom {\n  top: auto;\n  bottom: 0;\n  left: 50%;\n  transform: translate(-50%, 50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-top, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-top {\n  top: 0;\n  left: 50%;\n  transform: translate(-50%, -50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-left, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-left {\n  top: 50%;\n  left: 0;\n  transform: translate(-50%, -50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-right, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-right {\n  top: 50%;\n  right: 0;\n  transform: translate(50%, -50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edgeupdater, [data-ext-id=\"com.nubeio.ce\"].react-flow__edgeupdater {\n  cursor: move;\n  pointer-events: all;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.selection .react-flow__panel, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.selection .react-flow__panel {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel {\n  z-index: 5;\n  margin: 15px;\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.top {\n  top: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.bottom {\n  bottom: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.top.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.top.center, [data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.bottom.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.bottom.center {\n  left: 50%;\n  transform: translateX(-15px) translateX(-50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.left {\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.right {\n  right: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.left.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.left.center, [data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.right.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.right.center {\n  top: 50%;\n  transform: translateY(-15px) translateY(-50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__attribution, [data-ext-id=\"com.nubeio.ce\"].react-flow__attribution {\n  background: var(--xy-attribution-background-color, var(--xy-attribution-background-color-default));\n  margin: 0;\n  padding: 2px 3px;\n  font-size: 10px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__attribution a, [data-ext-id=\"com.nubeio.ce\"].react-flow__attribution a {\n  color: #999;\n  text-decoration: none;\n}\n\n@keyframes dashdraw {\n  from {\n    stroke-dashoffset: 10px;\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edgelabel-renderer, [data-ext-id=\"com.nubeio.ce\"].react-flow__edgelabel-renderer {\n  pointer-events: none;\n  -webkit-user-select: none;\n  user-select: none;\n  width: 100%;\n  height: 100%;\n  position: absolute;\n  top: 0;\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__viewport-portal, [data-ext-id=\"com.nubeio.ce\"].react-flow__viewport-portal {\n  -webkit-user-select: none;\n  user-select: none;\n  width: 100%;\n  height: 100%;\n  position: absolute;\n  top: 0;\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap {\n  background: var(--xy-minimap-background-color-props, var(--xy-minimap-background-color, var(--xy-minimap-background-color-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-svg {\n  display: block;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-mask, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-mask {\n  fill: var(--xy-minimap-mask-background-color-props, var(--xy-minimap-mask-background-color, var(--xy-minimap-mask-background-color-default)));\n  stroke: var(--xy-minimap-mask-stroke-color-props, var(--xy-minimap-mask-stroke-color, var(--xy-minimap-mask-stroke-color-default)));\n  stroke-width: var(--xy-minimap-mask-stroke-width-props, var(--xy-minimap-mask-stroke-width, var(--xy-minimap-mask-stroke-width-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-node, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-node {\n  fill: var(--xy-minimap-node-background-color-props, var(--xy-minimap-node-background-color, var(--xy-minimap-node-background-color-default)));\n  stroke: var(--xy-minimap-node-stroke-color-props, var(--xy-minimap-node-stroke-color, var(--xy-minimap-node-stroke-color-default)));\n  stroke-width: var(--xy-minimap-node-stroke-width-props, var(--xy-minimap-node-stroke-width, var(--xy-minimap-node-stroke-width-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.dots, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.dots {\n  fill: var(--xy-background-pattern-color-props, var(--xy-background-pattern-color, var(--xy-background-pattern-dots-color-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.lines, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.lines {\n  stroke: var(--xy-background-pattern-color-props, var(--xy-background-pattern-color, var(--xy-background-pattern-lines-color-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.cross, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.cross {\n  stroke: var(--xy-background-pattern-color-props, var(--xy-background-pattern-color, var(--xy-background-pattern-cross-color-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls {\n  box-shadow: var(--xy-controls-box-shadow, var(--xy-controls-box-shadow-default));\n  flex-direction: column;\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal {\n  flex-direction: row;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button {\n  background: var(--xy-controls-button-background-color, var(--xy-controls-button-background-color-default));\n  border: none;\n  border-bottom: 1px solid var(--xy-controls-button-border-color-props, var(--xy-controls-button-border-color, var(--xy-controls-button-border-color-default)));\n  width: 26px;\n  height: 26px;\n  color: var(--xy-controls-button-color-props, var(--xy-controls-button-color, var(--xy-controls-button-color-default)));\n  cursor: pointer;\n  -webkit-user-select: none;\n  user-select: none;\n  justify-content: center;\n  align-items: center;\n  padding: 4px;\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button svg {\n  fill: currentColor;\n  width: 100%;\n  max-width: 12px;\n  max-height: 12px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.updating .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.updating .react-flow__edge-path {\n  stroke: #777;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-text {\n  font-size: 10px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable:focus-visible {\n  outline: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group {\n  border-radius: var(--xy-node-border-radius, var(--xy-node-border-radius-default));\n  width: 150px;\n  color: var(--xy-node-color, var(--xy-node-color-default));\n  text-align: center;\n  border: var(--xy-node-border, var(--xy-node-border-default));\n  background-color: var(--xy-node-background-color, var(--xy-node-background-color-default));\n  padding: 10px;\n  font-size: 12px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:hover {\n  box-shadow: var(--xy-node-boxshadow-hover, var(--xy-node-boxshadow-hover-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:focus-visible {\n  box-shadow: var(--xy-node-boxshadow-selected, var(--xy-node-boxshadow-selected-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group {\n  background-color: var(--xy-node-group-background-color, var(--xy-node-group-background-color-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection {\n  background: var(--xy-selection-background-color, var(--xy-selection-background-color-default));\n  border: var(--xy-selection-border, var(--xy-selection-border-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection:focus-visible {\n  outline: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:hover {\n  background: var(--xy-controls-button-background-color-hover-props, var(--xy-controls-button-background-color-hover, var(--xy-controls-button-background-color-hover-default)));\n  color: var(--xy-controls-button-color-hover-props, var(--xy-controls-button-color-hover, var(--xy-controls-button-color-hover-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:disabled, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:disabled {\n  pointer-events: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:disabled svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:disabled svg {\n  fill-opacity: .4;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:last-child, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:last-child {\n  border-bottom: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal .react-flow__controls-button, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal .react-flow__controls-button {\n  border-bottom: none;\n  border-right: 1px solid var(--xy-controls-button-border-color-props, var(--xy-controls-button-border-color, var(--xy-controls-button-border-color-default)));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal .react-flow__controls-button:last-child, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal .react-flow__controls-button:last-child {\n  border-right: none;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control {\n  position: absolute;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.right {\n  cursor: ew-resize;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom {\n  cursor: ns-resize;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom.right {\n  cursor: nwse-resize;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top.right {\n  cursor: nesw-resize;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle {\n  background-color: var(--xy-resize-background-color, var(--xy-resize-background-color-default));\n  border: 1px solid #fff;\n  border-radius: 1px;\n  width: 5px;\n  height: 5px;\n  translate: -50% -50%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.left {\n  top: 50%;\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.right {\n  top: 50%;\n  left: 100%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top {\n  top: 0;\n  left: 50%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom {\n  top: 100%;\n  left: 50%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom.left {\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top.right, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom.right {\n  left: 100%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line {\n  border-color: var(--xy-resize-background-color, var(--xy-resize-background-color-default));\n  border-style: solid;\n  border-width: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.right {\n  width: 1px;\n  height: 100%;\n  top: 0;\n  transform: translate(-50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.left {\n  border-left-width: 1px;\n  left: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.right {\n  border-right-width: 1px;\n  left: 100%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.bottom {\n  width: 100%;\n  height: 1px;\n  left: 0;\n  transform: translate(0, -50%);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.top {\n  border-top-width: 1px;\n  top: 0;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.bottom {\n  border-bottom-width: 1px;\n  top: 100%;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-textbg, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-textbg {\n  fill: var(--xy-edge-label-background-color, var(--xy-edge-label-background-color-default));\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-text {\n  fill: var(--xy-edge-label-color, var(--xy-edge-label-color-default));\n}\n\n:root {\n  --sf-node-bg: #ffffffeb;\n  --sf-node-fg: #0f172a;\n  --sf-node-border: #e2e8f0;\n  --sf-node-divider: #eef2f7;\n  --sf-node-extra: #475569;\n  --sf-node-shadow: 0 1px 0 0 #0f172a0a inset,\n                       0 14px 32px -18px #0f172a73,\n                       0 2px 6px -2px #0f172a14;\n  --sf-handle-border: #fff;\n  --sf-handle-ring: #0f172a1a;\n  --sf-slot-label: #475569;\n  --sf-slot-value-bg: #0f172a0f;\n  --sf-slot-value-fg: #0f172a;\n  --sf-edge-label-bg: #fff;\n  --sf-palette-bg: #fff;\n  --sf-palette-border: #e2e8f0;\n  --sf-palette-hover: #f1f5f9;\n  --sf-font: ui-sans-serif, system-ui, -apple-system, \"Segoe UI\", sans-serif;\n  --sf-mono: ui-monospace, SFMono-Regular, Menlo, monospace;\n  --sf-radius: 14px;\n  --sf-radius-sm: 8px;\n  --sf-accent-default: #0ea5e9;\n  --sf-state-ready: #3b82f6;\n  --sf-state-running: #f59e0b;\n  --sf-state-ok: #10b981;\n  --sf-state-error: #ef4444;\n  --sf-state-cancelled: #64748b;\n  --sf-state-skipped: #94a3b8;\n  --sf-selected-ring: var(--sf-accent, var(--sf-accent-default));\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  :root {\n    --sf-selected-ring: color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 60%, transparent);\n  }\n}\n\n@media (prefers-color-scheme: dark) {\n  :root {\n    --sf-node-bg: #0f172ac7;\n    --sf-node-fg: #f1f5f9;\n    --sf-node-border: #1e293b;\n    --sf-node-divider: #1e293b;\n    --sf-node-extra: #cbd5e1;\n    --sf-node-shadow: 0 1px 0 0 #ffffff0a inset,\n                         0 16px 36px -14px #0009,\n                         0 2px 6px -2px #0006;\n    --sf-handle-border: #0f172a;\n    --sf-handle-ring: #ffffff14;\n    --sf-slot-label: #cbd5e1;\n    --sf-slot-value-bg: #ffffff14;\n    --sf-slot-value-fg: #f1f5f9;\n    --sf-edge-label-bg: #0f172a;\n    --sf-palette-bg: #0f172aeb;\n    --sf-palette-border: #1e293b;\n    --sf-palette-hover: #1e293b;\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node, [data-ext-id=\"com.nubeio.ce\"].sf-node {\n  font-family: var(--sf-font);\n  color: var(--sf-node-fg);\n  background: var(--sf-node-bg);\n  border: 1px solid var(--sf-node-border);\n  border-radius: var(--sf-radius);\n  width: var(--sf-node-width, 240px);\n  box-shadow: var(--sf-node-shadow);\n  -webkit-backdrop-filter: blur(10px) saturate(140%);\n  transition: box-shadow .16s, border-color .16s, transform .16s;\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node:hover, [data-ext-id=\"com.nubeio.ce\"].sf-node:hover {\n  border-color: var(--sf-accent, var(--sf-accent-default));\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node:hover, [data-ext-id=\"com.nubeio.ce\"].sf-node:hover {\n    border-color: color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 35%, var(--sf-node-border));\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--selected, [data-ext-id=\"com.nubeio.ce\"].sf-node--selected {\n  border-color: var(--sf-accent, var(--sf-accent-default));\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node--selected, [data-ext-id=\"com.nubeio.ce\"].sf-node--selected {\n    border-color: color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 70%, var(--sf-node-border));\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--selected, [data-ext-id=\"com.nubeio.ce\"].sf-node--selected {\n  box-shadow: var(--sf-node-shadow), 0 0 0 3px var(--sf-selected-ring);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node:before, [data-ext-id=\"com.nubeio.ce\"].sf-node:before {\n  content: \"\";\n  background: linear-gradient(90deg, var(--sf-accent, var(--sf-accent-default)) 0%, var(--sf-accent, var(--sf-accent-default)) 100%);\n  height: 3px;\n  display: block;\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node:before, [data-ext-id=\"com.nubeio.ce\"].sf-node:before {\n    background: linear-gradient(90deg, var(--sf-accent, var(--sf-accent-default)) 0%, color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 50%, transparent) 100%);\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__header, [data-ext-id=\"com.nubeio.ce\"].sf-node__header {\n  letter-spacing: -.005em;\n  align-items: center;\n  gap: 8px;\n  padding: 10px 12px 8px;\n  font-size: 12px;\n  font-weight: 600;\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__icon, [data-ext-id=\"com.nubeio.ce\"].sf-node__icon {\n  background: var(--sf-accent, var(--sf-accent-default));\n  border-radius: 6px;\n  justify-content: center;\n  align-items: center;\n  width: 22px;\n  height: 22px;\n  display: inline-flex;\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node__icon, [data-ext-id=\"com.nubeio.ce\"].sf-node__icon {\n    background: color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 18%, transparent);\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__icon, [data-ext-id=\"com.nubeio.ce\"].sf-node__icon {\n  color: var(--sf-accent, var(--sf-accent-default));\n  flex-shrink: 0;\n  font-size: 10px;\n  font-weight: 700;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__title, [data-ext-id=\"com.nubeio.ce\"].sf-node__title {\n  text-overflow: ellipsis;\n  white-space: nowrap;\n  min-width: 0;\n  color: var(--sf-node-fg);\n  flex: auto;\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__kind, [data-ext-id=\"com.nubeio.ce\"].sf-node__kind {\n  text-transform: uppercase;\n  letter-spacing: .08em;\n  color: var(--sf-slot-label);\n  opacity: .7;\n  font-size: 10px;\n  font-weight: 500;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state, [data-ext-id=\"com.nubeio.ce\"].sf-node__state {\n  border-radius: 50%;\n  flex-shrink: 0;\n  width: 8px;\n  height: 8px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--ready, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--ready {\n  background: var(--sf-state-ready);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--running, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--running {\n  background: var(--sf-state-running);\n  box-shadow: 0 0 0 0 var(--sf-state-running);\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node__state--running, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--running {\n    box-shadow: 0 0 0 0 color-mix(in oklab, var(--sf-state-running) 40%, transparent);\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--running, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--running {\n  animation: 1.2s ease-in-out infinite sf-pulse-dot;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--ok, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--ok {\n  background: var(--sf-state-ok);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--error, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--error {\n  background: var(--sf-state-error);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--cancelled, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--cancelled {\n  background: var(--sf-state-cancelled);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__state--skipped, [data-ext-id=\"com.nubeio.ce\"].sf-node__state--skipped {\n  background: var(--sf-state-skipped);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__body, [data-ext-id=\"com.nubeio.ce\"].sf-node__body {\n  border-top: 1px solid var(--sf-node-divider);\n  grid-template-columns: 1fr 1fr;\n  gap: 4px 12px;\n  min-width: 0;\n  padding: 6px 0 10px;\n  display: grid;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__col, [data-ext-id=\"com.nubeio.ce\"].sf-node__col {\n  flex-direction: column;\n  gap: 4px;\n  min-width: 0;\n  padding: 6px 0;\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__col--out, [data-ext-id=\"com.nubeio.ce\"].sf-node__col--out {\n  align-items: flex-end;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node__extra, [data-ext-id=\"com.nubeio.ce\"].sf-node__extra {\n  border-top: 1px solid var(--sf-node-divider);\n  color: var(--sf-node-extra);\n  padding: 8px 12px;\n  font-size: 11px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot-row, [data-ext-id=\"com.nubeio.ce\"].sf-slot-row {\n  flex-direction: column;\n  min-width: 0;\n  max-width: 100%;\n  display: flex;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot-row--input, [data-ext-id=\"com.nubeio.ce\"].sf-slot-row--input {\n  align-items: flex-start;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot-row--output, [data-ext-id=\"com.nubeio.ce\"].sf-slot-row--output {\n  align-items: flex-end;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot, [data-ext-id=\"com.nubeio.ce\"].sf-slot {\n  align-items: center;\n  gap: 8px;\n  padding: 2px 12px;\n  display: flex;\n  position: relative;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot--output, [data-ext-id=\"com.nubeio.ce\"].sf-slot--output {\n  flex-direction: row-reverse;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot__handle, [data-ext-id=\"com.nubeio.ce\"].sf-slot__handle {\n  box-shadow: 0 0 0 2px var(--sf-handle-ring);\n  transition: transform .12s, box-shadow .12s;\n  border: 2px solid var(--sf-handle-border) !important;\n  width: 10px !important;\n  height: 10px !important;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot__handle:hover, [data-ext-id=\"com.nubeio.ce\"].sf-slot__handle:hover {\n  box-shadow: 0 0 0 3px var(--sf-accent, var(--sf-accent-default));\n  transform: scale(1.25);\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-slot__handle:hover, [data-ext-id=\"com.nubeio.ce\"].sf-slot__handle:hover {\n    box-shadow: 0 0 0 3px color-mix(in oklab, var(--sf-accent, var(--sf-accent-default)) 30%, transparent);\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot__label, [data-ext-id=\"com.nubeio.ce\"].sf-slot__label {\n  color: var(--sf-slot-label);\n  white-space: nowrap;\n  font-size: 11px;\n  line-height: 1.2;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot__required, [data-ext-id=\"com.nubeio.ce\"].sf-slot__required {\n  color: var(--sf-state-error);\n  margin-left: 2px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-slot__value, [data-ext-id=\"com.nubeio.ce\"].sf-slot__value {\n  background: var(--sf-slot-value-bg);\n  color: var(--sf-slot-value-fg);\n  font-family: var(--sf-mono);\n  max-width: calc(var(--sf-node-width, 240px) - 32px);\n  text-overflow: ellipsis;\n  white-space: nowrap;\n  border-radius: 4px;\n  margin: 1px 12px 0;\n  padding: 1px 6px;\n  font-size: 10px;\n  line-height: 1.3;\n  overflow: hidden;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--ready, [data-ext-id=\"com.nubeio.ce\"].sf-node--ready {\n  border-color: var(--sf-state-ready);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--ok, [data-ext-id=\"com.nubeio.ce\"].sf-node--ok {\n  border-color: var(--sf-state-ok);\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node--ok, [data-ext-id=\"com.nubeio.ce\"].sf-node--ok {\n    border-color: color-mix(in oklab, var(--sf-state-ok) 60%, var(--sf-node-border));\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--error, [data-ext-id=\"com.nubeio.ce\"].sf-node--error {\n  border-color: var(--sf-state-error);\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node--error, [data-ext-id=\"com.nubeio.ce\"].sf-node--error {\n    border-color: color-mix(in oklab, var(--sf-state-error) 70%, var(--sf-node-border));\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--cancelled, [data-ext-id=\"com.nubeio.ce\"].sf-node--cancelled {\n  border-color: var(--sf-state-cancelled);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--skipped, [data-ext-id=\"com.nubeio.ce\"].sf-node--skipped {\n  border-color: var(--sf-state-skipped);\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--running, [data-ext-id=\"com.nubeio.ce\"].sf-node--running {\n  border-color: var(--sf-state-running);\n}\n\n@supports (color: color-mix(in lab, red, red)) {\n  [data-ext-id=\"com.nubeio.ce\"] .sf-node--running, [data-ext-id=\"com.nubeio.ce\"].sf-node--running {\n    border-color: color-mix(in oklab, var(--sf-state-running) 70%, var(--sf-node-border));\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-node--running, [data-ext-id=\"com.nubeio.ce\"].sf-node--running {\n  animation: 1.6s ease-in-out infinite sf-pulse;\n}\n\n@keyframes sf-pulse {\n  0%, 100% {\n    box-shadow: var(--sf-node-shadow), 0 0 0 0 color-mix(in oklab, var(--sf-state-running) 35%, transparent);\n  }\n\n  50% {\n    box-shadow: var(--sf-node-shadow), 0 0 0 8px color-mix(in oklab, var(--sf-state-running) 0%, transparent);\n  }\n}\n\n@keyframes sf-pulse-dot {\n  0%, 100% {\n    box-shadow: 0 0 0 0 color-mix(in oklab, var(--sf-state-running) 50%, transparent);\n  }\n\n  50% {\n    box-shadow: 0 0 0 5px color-mix(in oklab, var(--sf-state-running) 0%, transparent);\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .sf-edge--active, [data-ext-id=\"com.nubeio.ce\"].sf-edge--active {\n  animation: .6s linear infinite sf-dash;\n}\n\n@keyframes sf-dash {\n  to {\n    stroke-dashoffset: -20px;\n  }\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .ext-eyebrow, [data-ext-id=\"com.nubeio.ce\"].ext-eyebrow {\n  letter-spacing: .12em;\n  text-transform: uppercase;\n  color: var(--color-muted-foreground, #64748b);\n  font-size: .625rem;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .ext-card, [data-ext-id=\"com.nubeio.ce\"].ext-card {\n  background: var(--color-card, #fff);\n  border: 1px solid var(--color-border, #0f172a1a);\n  border-radius: 12px;\n}\n\n[data-ext-id=\"com.nubeio.ce\"] .ext-wiresheet, [data-ext-id=\"com.nubeio.ce\"].ext-wiresheet {\n  border: 1px solid var(--color-border, #0f172a1a);\n  border-radius: 12px;\n  width: 100%;\n  height: calc(100vh - 9rem);\n  min-height: 24rem;\n  position: relative;\n  overflow: hidden;\n}\n\n@property --tw-border-style {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: solid;\n}\n\n@property --tw-font-weight {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-shadow-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-shadow-alpha {\n  syntax: \"<percentage>\";\n  inherits: false;\n  initial-value: 100%;\n}\n\n@property --tw-inset-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-inset-shadow-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-inset-shadow-alpha {\n  syntax: \"<percentage>\";\n  inherits: false;\n  initial-value: 100%;\n}\n\n@property --tw-ring-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ring-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-inset-ring-color {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-inset-ring-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n\n@property --tw-ring-inset {\n  syntax: \"*\";\n  inherits: false\n}\n\n@property --tw-ring-offset-width {\n  syntax: \"<length>\";\n  inherits: false;\n  initial-value: 0;\n}\n\n@property --tw-ring-offset-color {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: #fff;\n}\n\n@property --tw-ring-offset-shadow {\n  syntax: \"*\";\n  inherits: false;\n  initial-value: 0 0 #0000;\n}\n/* this gets exported as style.css and can be used for the default theming */\n/* these are the necessary styles for React/Svelte Flow, they get used by base.css and style.css */\n[data-ext-id=\"com.nubeio.ce\"] .react-flow, [data-ext-id=\"com.nubeio.ce\"].react-flow {\n  direction: ltr;\n\n  --xy-edge-stroke-default: #b1b1b7;\n  --xy-edge-stroke-width-default: 1;\n  --xy-edge-stroke-selected-default: #555;\n\n  --xy-connectionline-stroke-default: #b1b1b7;\n  --xy-connectionline-stroke-width-default: 1;\n\n  --xy-attribution-background-color-default: rgba(255, 255, 255, 0.5);\n\n  --xy-minimap-background-color-default: #fff;\n  --xy-minimap-mask-background-color-default: rgba(240, 240, 240, 0.6);\n  --xy-minimap-mask-stroke-color-default: transparent;\n  --xy-minimap-mask-stroke-width-default: 1;\n  --xy-minimap-node-background-color-default: #e2e2e2;\n  --xy-minimap-node-stroke-color-default: transparent;\n  --xy-minimap-node-stroke-width-default: 2;\n\n  --xy-background-color-default: transparent;\n  --xy-background-pattern-dots-color-default: #91919a;\n  --xy-background-pattern-lines-color-default: #eee;\n  --xy-background-pattern-cross-color-default: #e2e2e2;\n  background-color: var(--xy-background-color, var(--xy-background-color-default));\n  --xy-node-color-default: inherit;\n  --xy-node-border-default: 1px solid #1a192b;\n  --xy-node-background-color-default: #fff;\n  --xy-node-group-background-color-default: rgba(240, 240, 240, 0.25);\n  --xy-node-boxshadow-hover-default: 0 1px 4px 1px rgba(0, 0, 0, 0.08);\n  --xy-node-boxshadow-selected-default: 0 0 0 0.5px #1a192b;\n  --xy-node-border-radius-default: 3px;\n\n  --xy-handle-background-color-default: #1a192b;\n  --xy-handle-border-color-default: #fff;\n\n  --xy-selection-background-color-default: rgba(0, 89, 220, 0.08);\n  --xy-selection-border-default: 1px dotted rgba(0, 89, 220, 0.8);\n\n  --xy-controls-button-background-color-default: #fefefe;\n  --xy-controls-button-background-color-hover-default: #f4f4f4;\n  --xy-controls-button-color-default: inherit;\n  --xy-controls-button-color-hover-default: inherit;\n  --xy-controls-button-border-color-default: #eee;\n  --xy-controls-box-shadow-default: 0 0 2px 1px rgba(0, 0, 0, 0.08);\n\n  --xy-edge-label-background-color-default: #ffffff;\n  --xy-edge-label-color-default: inherit;\n  --xy-resize-background-color-default: #3367d9;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow.dark, [data-ext-id=\"com.nubeio.ce\"].react-flow.dark {\n  --xy-edge-stroke-default: #3e3e3e;\n  --xy-edge-stroke-width-default: 1;\n  --xy-edge-stroke-selected-default: #727272;\n\n  --xy-connectionline-stroke-default: #b1b1b7;\n  --xy-connectionline-stroke-width-default: 1;\n\n  --xy-attribution-background-color-default: rgba(150, 150, 150, 0.25);\n\n  --xy-minimap-background-color-default: #141414;\n  --xy-minimap-mask-background-color-default: rgba(60, 60, 60, 0.6);\n  --xy-minimap-mask-stroke-color-default: transparent;\n  --xy-minimap-mask-stroke-width-default: 1;\n  --xy-minimap-node-background-color-default: #2b2b2b;\n  --xy-minimap-node-stroke-color-default: transparent;\n  --xy-minimap-node-stroke-width-default: 2;\n\n  --xy-background-color-default: #141414;\n  --xy-background-pattern-dots-color-default: #777;\n  --xy-background-pattern-lines-color-default: #777;\n  --xy-background-pattern-cross-color-default: #777;\n  --xy-node-color-default: #f8f8f8;\n  --xy-node-border-default: 1px solid #3c3c3c;\n  --xy-node-background-color-default: #1e1e1e;\n  --xy-node-group-background-color-default: rgba(240, 240, 240, 0.25);\n  --xy-node-boxshadow-hover-default: 0 1px 4px 1px rgba(255, 255, 255, 0.08);\n  --xy-node-boxshadow-selected-default: 0 0 0 0.5px #999;\n\n  --xy-handle-background-color-default: #bebebe;\n  --xy-handle-border-color-default: #1e1e1e;\n\n  --xy-selection-background-color-default: rgba(200, 200, 220, 0.08);\n  --xy-selection-border-default: 1px dotted rgba(200, 200, 220, 0.8);\n\n  --xy-controls-button-background-color-default: #2b2b2b;\n  --xy-controls-button-background-color-hover-default: #3e3e3e;\n  --xy-controls-button-color-default: #f8f8f8;\n  --xy-controls-button-color-hover-default: #fff;\n  --xy-controls-button-border-color-default: #5b5b5b;\n  --xy-controls-box-shadow-default: 0 0 2px 1px rgba(0, 0, 0, 0.08);\n\n  --xy-edge-label-background-color-default: #141414;\n  --xy-edge-label-color-default: #f8f8f8;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background, [data-ext-id=\"com.nubeio.ce\"].react-flow__background {\n  background-color: var(--xy-background-color-props, var(--xy-background-color, var(--xy-background-color-default)));\n  pointer-events: none;\n  z-index: -1;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__container, [data-ext-id=\"com.nubeio.ce\"].react-flow__container {\n  position: absolute;\n  width: 100%;\n  height: 100%;\n  top: 0;\n  left: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane {\n  z-index: 1;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.draggable, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.draggable {\n    cursor: grab;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.dragging, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.dragging {\n    cursor: grabbing;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.selection {\n    cursor: pointer;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__viewport, [data-ext-id=\"com.nubeio.ce\"].react-flow__viewport {\n  transform-origin: 0 0;\n  z-index: 2;\n  pointer-events: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__renderer, [data-ext-id=\"com.nubeio.ce\"].react-flow__renderer {\n  z-index: 4;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection {\n  z-index: 6;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus-visible {\n  outline: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-path {\n  stroke: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n  stroke-width: var(--xy-edge-stroke-width, var(--xy-edge-stroke-width-default));\n  fill: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection-path {\n  stroke: var(--xy-connectionline-stroke, var(--xy-connectionline-stroke-default));\n  stroke-width: var(--xy-connectionline-stroke-width, var(--xy-connectionline-stroke-width-default));\n  fill: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow .react-flow__edges, [data-ext-id=\"com.nubeio.ce\"].react-flow .react-flow__edges {\n  position: absolute;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow .react-flow__edges svg, [data-ext-id=\"com.nubeio.ce\"].react-flow .react-flow__edges svg {\n    overflow: visible;\n    position: absolute;\n    pointer-events: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge {\n  pointer-events: visibleStroke;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable {\n    cursor: pointer;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.animated path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.animated path {\n    stroke-dasharray: 5;\n    animation: dashdraw 0.5s linear infinite;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.animated path.react-flow__edge-interaction, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.animated path.react-flow__edge-interaction {\n    stroke-dasharray: none;\n    animation: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.inactive, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.inactive {\n    pointer-events: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge:focus-visible {\n    outline: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selected .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selected .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable:focus .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable:focus .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.selectable:focus-visible .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.selectable:focus-visible .react-flow__edge-path {\n    stroke: var(--xy-edge-stroke-selected, var(--xy-edge-stroke-selected-default));\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-textwrapper, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-textwrapper {\n    pointer-events: all;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge .react-flow__edge-text {\n    pointer-events: none;\n    -webkit-user-select: none;\n       -moz-user-select: none;\n            user-select: none;\n  }\n/* Arrowhead marker styles - use CSS custom properties as default */\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__arrowhead polyline, [data-ext-id=\"com.nubeio.ce\"].react-flow__arrowhead polyline {\n  stroke: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__arrowhead polyline.arrowclosed, [data-ext-id=\"com.nubeio.ce\"].react-flow__arrowhead polyline.arrowclosed {\n  fill: var(--xy-edge-stroke, var(--xy-edge-stroke-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection {\n  pointer-events: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__connection .animated, [data-ext-id=\"com.nubeio.ce\"].react-flow__connection .animated {\n    stroke-dasharray: 5;\n    animation: dashdraw 0.5s linear infinite;\n  }\n[data-ext-id=\"com.nubeio.ce\"] svg.react-flow__connectionline {\n  z-index: 1001;\n  overflow: visible;\n  position: absolute;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodes, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodes {\n  pointer-events: none;\n  transform-origin: 0 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node, [data-ext-id=\"com.nubeio.ce\"].react-flow__node {\n  position: absolute;\n  -webkit-user-select: none;\n     -moz-user-select: none;\n          user-select: none;\n  pointer-events: all;\n  transform-origin: 0 0;\n  box-sizing: border-box;\n  cursor: default;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable {\n    cursor: pointer;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.draggable, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.draggable {\n    cursor: grab;\n    pointer-events: all;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.draggable.dragging, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.draggable.dragging {\n      cursor: grabbing;\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection {\n  z-index: 3;\n  transform-origin: left top;\n  pointer-events: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect {\n    position: absolute;\n    pointer-events: all;\n    cursor: grab;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle {\n  position: absolute;\n  pointer-events: none;\n  min-width: 5px;\n  min-height: 5px;\n  width: 6px;\n  height: 6px;\n  background-color: var(--xy-handle-background-color, var(--xy-handle-background-color-default));\n  border: 1px solid var(--xy-handle-border-color, var(--xy-handle-border-color-default));\n  border-radius: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle.connectingfrom, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle.connectingfrom {\n    pointer-events: all;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle.connectionindicator, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle.connectionindicator {\n    pointer-events: all;\n    cursor: crosshair;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-bottom {\n    top: auto;\n    left: 50%;\n    bottom: 0;\n    transform: translate(-50%, 50%);\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-top, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-top {\n    top: 0;\n    left: 50%;\n    transform: translate(-50%, -50%);\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-left, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-left {\n    top: 50%;\n    left: 0;\n    transform: translate(-50%, -50%);\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__handle-right, [data-ext-id=\"com.nubeio.ce\"].react-flow__handle-right {\n    top: 50%;\n    right: 0;\n    transform: translate(50%, -50%);\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edgeupdater, [data-ext-id=\"com.nubeio.ce\"].react-flow__edgeupdater {\n  cursor: move;\n  pointer-events: all;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__pane.selection .react-flow__panel, [data-ext-id=\"com.nubeio.ce\"].react-flow__pane.selection .react-flow__panel {\n  pointer-events: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel {\n  position: absolute;\n  z-index: 5;\n  margin: 15px;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.top {\n    top: 0;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.bottom {\n    bottom: 0;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.top.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.top.center, [data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.bottom.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.bottom.center {\n      left: 50%;\n      transform: translateX(-15px) translateX(-50%);\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.left {\n    left: 0;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.right {\n    right: 0;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.left.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.left.center, [data-ext-id=\"com.nubeio.ce\"] .react-flow__panel.right.center, [data-ext-id=\"com.nubeio.ce\"].react-flow__panel.right.center {\n      top: 50%;\n      transform: translateY(-15px) translateY(-50%);\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__attribution, [data-ext-id=\"com.nubeio.ce\"].react-flow__attribution {\n  font-size: 10px;\n  background: var(--xy-attribution-background-color, var(--xy-attribution-background-color-default));\n  padding: 2px 3px;\n  margin: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__attribution a, [data-ext-id=\"com.nubeio.ce\"].react-flow__attribution a {\n    text-decoration: none;\n    color: #999;\n  }\n@keyframes dashdraw {\n  from {\n    stroke-dashoffset: 10;\n  }\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edgelabel-renderer, [data-ext-id=\"com.nubeio.ce\"].react-flow__edgelabel-renderer {\n  position: absolute;\n  width: 100%;\n  height: 100%;\n  pointer-events: none;\n  -webkit-user-select: none;\n     -moz-user-select: none;\n          user-select: none;\n  left: 0;\n  top: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__viewport-portal, [data-ext-id=\"com.nubeio.ce\"].react-flow__viewport-portal {\n  position: absolute;\n  width: 100%;\n  height: 100%;\n  left: 0;\n  top: 0;\n  -webkit-user-select: none;\n     -moz-user-select: none;\n          user-select: none;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap {\n  background: var(\n    --xy-minimap-background-color-props,\n    var(--xy-minimap-background-color, var(--xy-minimap-background-color-default))\n  );\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-svg {\n    display: block;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-mask, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-mask {\n    fill: var(\n      --xy-minimap-mask-background-color-props,\n      var(--xy-minimap-mask-background-color, var(--xy-minimap-mask-background-color-default))\n    );\n    stroke: var(\n      --xy-minimap-mask-stroke-color-props,\n      var(--xy-minimap-mask-stroke-color, var(--xy-minimap-mask-stroke-color-default))\n    );\n    stroke-width: var(\n      --xy-minimap-mask-stroke-width-props,\n      var(--xy-minimap-mask-stroke-width, var(--xy-minimap-mask-stroke-width-default))\n    );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__minimap-node, [data-ext-id=\"com.nubeio.ce\"].react-flow__minimap-node {\n    fill: var(\n      --xy-minimap-node-background-color-props,\n      var(--xy-minimap-node-background-color, var(--xy-minimap-node-background-color-default))\n    );\n    stroke: var(\n      --xy-minimap-node-stroke-color-props,\n      var(--xy-minimap-node-stroke-color, var(--xy-minimap-node-stroke-color-default))\n    );\n    stroke-width: var(\n      --xy-minimap-node-stroke-width-props,\n      var(--xy-minimap-node-stroke-width, var(--xy-minimap-node-stroke-width-default))\n    );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.dots, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.dots {\n    fill: var(\n      --xy-background-pattern-color-props,\n      var(--xy-background-pattern-color, var(--xy-background-pattern-dots-color-default))\n    );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.lines, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.lines {\n    stroke: var(\n      --xy-background-pattern-color-props,\n      var(--xy-background-pattern-color, var(--xy-background-pattern-lines-color-default))\n    );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__background-pattern.cross, [data-ext-id=\"com.nubeio.ce\"].react-flow__background-pattern.cross {\n    stroke: var(\n      --xy-background-pattern-color-props,\n      var(--xy-background-pattern-color, var(--xy-background-pattern-cross-color-default))\n    );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls {\n  display: flex;\n  flex-direction: column;\n  box-shadow: var(--xy-controls-box-shadow, var(--xy-controls-box-shadow-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal {\n    flex-direction: row;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button {\n    display: flex;\n    justify-content: center;\n    align-items: center;\n    height: 26px;\n    width: 26px;\n    padding: 4px;\n    border: none;\n    background: var(--xy-controls-button-background-color, var(--xy-controls-button-background-color-default));\n    border-bottom: 1px solid\n      var(\n        --xy-controls-button-border-color-props,\n        var(--xy-controls-button-border-color, var(--xy-controls-button-border-color-default))\n      );\n    color: var(\n      --xy-controls-button-color-props,\n      var(--xy-controls-button-color, var(--xy-controls-button-color-default))\n    );\n    cursor: pointer;\n    -webkit-user-select: none;\n       -moz-user-select: none;\n            user-select: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button svg {\n      width: 100%;\n      max-width: 12px;\n      max-height: 12px;\n      fill: currentColor;\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge.updating .react-flow__edge-path, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge.updating .react-flow__edge-path {\n      stroke: #777;\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-text {\n    font-size: 10px;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node.selectable:focus-visible {\n    outline: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group {\n  padding: 10px;\n  border-radius: var(--xy-node-border-radius, var(--xy-node-border-radius-default));\n  width: 150px;\n  font-size: 12px;\n  color: var(--xy-node-color, var(--xy-node-color-default));\n  text-align: center;\n  border: var(--xy-node-border, var(--xy-node-border-default));\n  background-color: var(--xy-node-background-color, var(--xy-node-background-color-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:hover, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:hover {\n      box-shadow: var(--xy-node-boxshadow-hover, var(--xy-node-boxshadow-hover-default));\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-input.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-input.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-default.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-default.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-output.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-output.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable.selected, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable.selected, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group.selectable:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group.selectable:focus-visible {\n      box-shadow: var(--xy-node-boxshadow-selected, var(--xy-node-boxshadow-selected-default));\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__node-group, [data-ext-id=\"com.nubeio.ce\"].react-flow__node-group {\n  background-color: var(--xy-node-group-background-color, var(--xy-node-group-background-color-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection {\n  background: var(--xy-selection-background-color, var(--xy-selection-background-color-default));\n  border: var(--xy-selection-border, var(--xy-selection-border-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__nodesselection-rect:focus-visible, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection:focus, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection:focus, [data-ext-id=\"com.nubeio.ce\"] .react-flow__selection:focus-visible, [data-ext-id=\"com.nubeio.ce\"].react-flow__selection:focus-visible {\n    outline: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:hover, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:hover {\n      background: var(\n        --xy-controls-button-background-color-hover-props,\n        var(--xy-controls-button-background-color-hover, var(--xy-controls-button-background-color-hover-default))\n      );\n      color: var(\n        --xy-controls-button-color-hover-props,\n        var(--xy-controls-button-color-hover, var(--xy-controls-button-color-hover-default))\n      );\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:disabled, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:disabled {\n      pointer-events: none;\n    }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:disabled svg, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:disabled svg {\n        fill-opacity: 0.4;\n      }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls-button:last-child, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls-button:last-child {\n    border-bottom: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal .react-flow__controls-button, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal .react-flow__controls-button {\n    border-bottom: none;\n    border-right: 1px solid\n      var(\n        --xy-controls-button-border-color-props,\n        var(--xy-controls-button-border-color, var(--xy-controls-button-border-color-default))\n      );\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__controls.horizontal .react-flow__controls-button:last-child, [data-ext-id=\"com.nubeio.ce\"].react-flow__controls.horizontal .react-flow__controls-button:last-child {\n    border-right: none;\n  }\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control {\n  position: absolute;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.right {\n  cursor: ew-resize;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom {\n  cursor: ns-resize;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom.right {\n  cursor: nwse-resize;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.bottom.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.bottom.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.top.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.top.right {\n  cursor: nesw-resize;\n}\n/* handle styles */\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle {\n  width: 5px;\n  height: 5px;\n  border: 1px solid #fff;\n  border-radius: 1px;\n  background-color: var(--xy-resize-background-color, var(--xy-resize-background-color-default));\n  translate: -50% -50%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.left {\n  left: 0;\n  top: 50%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.right {\n  left: 100%;\n  top: 50%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top {\n  left: 50%;\n  top: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom {\n  left: 50%;\n  top: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top.left {\n  left: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom.left {\n  left: 0;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.top.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.top.right {\n  left: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.handle.bottom.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.handle.bottom.right {\n  left: 100%;\n}\n/* line styles */\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line {\n  border-color: var(--xy-resize-background-color, var(--xy-resize-background-color-default));\n  border-width: 0;\n  border-style: solid;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.right {\n  width: 1px;\n  transform: translate(-50%, 0);\n  top: 0;\n  height: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.left, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.left {\n  left: 0;\n  border-left-width: 1px;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.right, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.right {\n  left: 100%;\n  border-right-width: 1px;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.bottom {\n  height: 1px;\n  transform: translate(0, -50%);\n  left: 0;\n  width: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.top, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.top {\n  top: 0;\n  border-top-width: 1px;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__resize-control.line.bottom, [data-ext-id=\"com.nubeio.ce\"].react-flow__resize-control.line.bottom {\n  border-bottom-width: 1px;\n  top: 100%;\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-textbg, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-textbg {\n  fill: var(--xy-edge-label-background-color, var(--xy-edge-label-background-color-default));\n}\n[data-ext-id=\"com.nubeio.ce\"] .react-flow__edge-text, [data-ext-id=\"com.nubeio.ce\"].react-flow__edge-text {\n  fill: var(--xy-edge-label-color, var(--xy-edge-label-color-default));\n}"));document.head.appendChild(elementStyle);}}catch(e){console.error('vite-plugin-css-injected-by-js', e);}

})();
import { jsx, jsxs, Fragment } from 'react/jsx-runtime';
import * as React from 'react';
import React__default, { forwardRef, createElement, useContext, memo, createContext, useMemo, useState, useRef, useEffect, useCallback, useLayoutEffect } from 'react';
import { createPortal } from 'react-dom';

const HOST_CLIENT_CTX_KEY = "__starterExtSdkHostClientContextV1";
globalThis[HOST_CLIENT_CTX_KEY] ?? (globalThis[HOST_CLIENT_CTX_KEY] = React.createContext(null));

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
function useExtensionRoute() {
  return useSlotContext().route;
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

const EXTENSION_ID = "com.nubeio.ce";

function Page({
  eyebrow,
  title,
  actions,
  children
}) {
  const slot = useSlotContext();
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ext-id": EXTENSION_ID,
      "data-ext-slot": slot.slotId,
      className: "flex flex-col gap-4 p-4",
      children: [
        /* @__PURE__ */ jsxs("header", { className: "flex items-end justify-between gap-3", children: [
          /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-0.5", children: [
            eyebrow ? /* @__PURE__ */ jsx("span", { className: "ext-eyebrow", children: eyebrow }) : null,
            /* @__PURE__ */ jsx("h3", { className: "text-xl font-semibold", children: title })
          ] }),
          actions ? /* @__PURE__ */ jsx("div", { className: "flex items-center gap-2", children: actions }) : null
        ] }),
        children
      ]
    }
  );
}

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */

const toKebabCase$1 = (string) => string.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
const mergeClasses$1 = (...classes) => classes.filter((className, index, array) => {
  return Boolean(className) && className.trim() !== "" && array.indexOf(className) === index;
}).join(" ").trim();

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */

var defaultAttributes$1 = {
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


const Icon$1 = forwardRef(
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
        ...defaultAttributes$1,
        width: size,
        height: size,
        stroke: color,
        strokeWidth: absoluteStrokeWidth ? Number(strokeWidth) * 24 / Number(size) : strokeWidth,
        className: mergeClasses$1("lucide", className),
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


const createLucideIcon$1 = (iconName, iconNode) => {
  const Component = forwardRef(
    ({ className, ...props }, ref) => createElement(Icon$1, {
      ref,
      iconNode,
      className: mergeClasses$1(`lucide-${toKebabCase$1(iconName)}`, className),
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


const Cpu = createLucideIcon$1("Cpu", [
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


const ExternalLink = createLucideIcon$1("ExternalLink", [
  ["path", { d: "M15 3h6v6", key: "1q9fwt" }],
  ["path", { d: "M10 14 21 3", key: "gplh6r" }],
  ["path", { d: "M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6", key: "a6xqqp" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const LayoutGrid = createLucideIcon$1("LayoutGrid", [
  ["rect", { width: "7", height: "7", x: "3", y: "3", rx: "1", key: "1g98yp" }],
  ["rect", { width: "7", height: "7", x: "14", y: "3", rx: "1", key: "6d4xhi" }],
  ["rect", { width: "7", height: "7", x: "14", y: "14", rx: "1", key: "nxv5o0" }],
  ["rect", { width: "7", height: "7", x: "3", y: "14", rx: "1", key: "1bb6yr" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Pencil = createLucideIcon$1("Pencil", [
  [
    "path",
    {
      d: "M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z",
      key: "1a8usu"
    }
  ],
  ["path", { d: "m15 5 4 4", key: "1mk7zo" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Plus = createLucideIcon$1("Plus", [
  ["path", { d: "M5 12h14", key: "1ays0h" }],
  ["path", { d: "M12 5v14", key: "s699le" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Trash2 = createLucideIcon$1("Trash2", [
  ["path", { d: "M3 6h18", key: "d0wm0j" }],
  ["path", { d: "M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6", key: "4alrt4" }],
  ["path", { d: "M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2", key: "v07s0e" }],
  ["line", { x1: "10", x2: "10", y1: "11", y2: "17", key: "1uufr5" }],
  ["line", { x1: "14", x2: "14", y1: "11", y2: "17", key: "xtxkd" }]
]);

async function callTool(toolId, params) {
  const res = await fetch(`/api/v1/tools/${toolId}`, {
    method: "POST",
    credentials: "same-origin",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(params ?? {})
  });
  return await res.json();
}
async function fetchTemplate(template, params = {}) {
  const res = await callTool(
    `${EXTENSION_ID}.warehouse_query`,
    { template, params }
  );
  return res.rows;
}

const tool = (name) => `${EXTENSION_ID}.${name}`;
function listDevices() {
  return fetchTemplate(`${EXTENSION_ID}.devices_list`, {});
}
function createDevice(input) {
  return callTool(tool("device_create"), input);
}
function updateDevice(input) {
  return callTool(tool("device_update"), input);
}
function deleteDevice(deviceId) {
  return callTool(tool("device_delete"), { device_id: deviceId });
}

function DeviceForm({
  initial,
  onDone,
  onCancel
}) {
  const isEdit = initial !== null;
  const [form, setForm] = React.useState(() => ({
    device_id: initial?.device_id ?? "",
    name: initial?.name ?? "",
    description: initial?.description ?? "",
    engine_kind: initial?.engine_kind ?? "",
    ip: initial?.ip ?? "",
    port: initial?.port ?? 443,
    username: initial?.username ?? "",
    password: ""
  }));
  const set = (key, value) => setForm((f) => ({ ...f, [key]: value }));
  const submit = (e) => {
    e.preventDefault();
    const op = isEdit ? updateDevice : createDevice;
    op(form).then(onDone).catch(() => onDone());
  };
  return /* @__PURE__ */ jsxs("form", { onSubmit: submit, className: "ext-card flex max-w-xl flex-col gap-4 p-4", children: [
    /* @__PURE__ */ jsx("h4", { className: "text-base font-semibold", children: isEdit ? `Edit ${initial?.name ?? initial?.device_id}` : "Add control engine" }),
    /* @__PURE__ */ jsx(Field, { label: "Device ID", children: /* @__PURE__ */ jsx(
      Input,
      {
        value: form.device_id,
        disabled: isEdit,
        onChange: (v) => set("device_id", v),
        placeholder: "ce-001"
      }
    ) }),
    /* @__PURE__ */ jsx(Field, { label: "Name", children: /* @__PURE__ */ jsx(Input, { value: form.name ?? "", onChange: (v) => set("name", v) }) }),
    /* @__PURE__ */ jsx(Field, { label: "Engine kind", children: /* @__PURE__ */ jsx(
      Input,
      {
        value: form.engine_kind ?? "",
        onChange: (v) => set("engine_kind", v),
        placeholder: "niagara | sedona"
      }
    ) }),
    /* @__PURE__ */ jsxs("div", { className: "grid grid-cols-[1fr_120px] gap-3", children: [
      /* @__PURE__ */ jsx(Field, { label: "IP / host", children: /* @__PURE__ */ jsx(Input, { value: form.ip ?? "", onChange: (v) => set("ip", v), placeholder: "10.0.0.5" }) }),
      /* @__PURE__ */ jsx(Field, { label: "Port", children: /* @__PURE__ */ jsx(
        Input,
        {
          value: String(form.port ?? ""),
          onChange: (v) => set("port", Number(v) || 0),
          placeholder: "443"
        }
      ) })
    ] }),
    /* @__PURE__ */ jsx(Field, { label: "Username", children: /* @__PURE__ */ jsx(Input, { value: form.username ?? "", onChange: (v) => set("username", v) }) }),
    /* @__PURE__ */ jsx(Field, { label: "Password", children: /* @__PURE__ */ jsx(
      Input,
      {
        value: form.password ?? "",
        type: "password",
        onChange: (v) => set("password", v),
        placeholder: isEdit ? "(unchanged)" : ""
      }
    ) }),
    /* @__PURE__ */ jsxs("div", { className: "flex justify-end gap-2 pt-2", children: [
      /* @__PURE__ */ jsx(
        "button",
        {
          type: "button",
          onClick: onCancel,
          className: "rounded-md border border-border px-3 py-1.5 text-sm",
          children: "Cancel"
        }
      ),
      /* @__PURE__ */ jsx(
        "button",
        {
          type: "submit",
          className: "rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground",
          children: isEdit ? "Save" : "Create"
        }
      )
    ] })
  ] });
}
function Field({
  label,
  children
}) {
  return /* @__PURE__ */ jsxs("label", { className: "flex flex-col gap-1", children: [
    /* @__PURE__ */ jsx("span", { className: "ext-eyebrow", children: label }),
    children
  ] });
}
function Input({
  value,
  onChange,
  type = "text",
  placeholder,
  disabled
}) {
  return /* @__PURE__ */ jsx(
    "input",
    {
      type,
      value,
      disabled,
      placeholder,
      onChange: (e) => onChange(e.target.value),
      className: "rounded-md border border-border bg-background px-2.5 py-1.5 text-sm disabled:opacity-60"
    }
  );
}

function DevicesPanel() {
  const [rows, setRows] = React.useState([]);
  const [editing, setEditing] = React.useState(null);
  const reload = React.useCallback(() => {
    listDevices().then(setRows).catch(() => setRows([]));
  }, []);
  React.useEffect(() => reload(), [reload]);
  if (editing) {
    return /* @__PURE__ */ jsx(
      DeviceForm,
      {
        initial: editing === "new" ? null : editing,
        onDone: () => {
          setEditing(null);
          reload();
        },
        onCancel: () => setEditing(null)
      }
    );
  }
  return /* @__PURE__ */ jsxs("div", { className: "flex flex-col gap-3", children: [
    /* @__PURE__ */ jsx("div", { className: "flex justify-end", children: /* @__PURE__ */ jsxs(
      "button",
      {
        type: "button",
        onClick: () => setEditing("new"),
        className: "inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground",
        children: [
          /* @__PURE__ */ jsx(Plus, { className: "size-4" }),
          " Add engine"
        ]
      }
    ) }),
    /* @__PURE__ */ jsx("div", { className: "ext-card overflow-hidden", children: /* @__PURE__ */ jsxs("table", { className: "w-full text-sm", children: [
      /* @__PURE__ */ jsx("thead", { className: "bg-muted/40 text-muted-foreground", children: /* @__PURE__ */ jsxs("tr", { children: [
        /* @__PURE__ */ jsx(Th, { children: "Name" }),
        /* @__PURE__ */ jsx(Th, { children: "Kind" }),
        /* @__PURE__ */ jsx(Th, { children: "Address" }),
        /* @__PURE__ */ jsx(Th, { children: "Status" }),
        /* @__PURE__ */ jsx(Th, { children: "" })
      ] }) }),
      /* @__PURE__ */ jsx("tbody", { children: rows.length === 0 ? /* @__PURE__ */ jsx("tr", { children: /* @__PURE__ */ jsx("td", { colSpan: 5, className: "px-3 py-6 text-center text-muted-foreground", children: "No engines registered yet." }) }) : rows.map((d) => /* @__PURE__ */ jsxs("tr", { className: "border-t border-border", children: [
        /* @__PURE__ */ jsx(Td$1, { children: d.name ?? d.device_id }),
        /* @__PURE__ */ jsx(Td$1, { children: d.engine_kind ?? "—" }),
        /* @__PURE__ */ jsxs(Td$1, { children: [
          d.ip,
          ":",
          d.port
        ] }),
        /* @__PURE__ */ jsx(Td$1, { children: d.status ?? "unknown" }),
        /* @__PURE__ */ jsx(Td$1, { children: /* @__PURE__ */ jsxs("div", { className: "flex items-center justify-end gap-2", children: [
          /* @__PURE__ */ jsxs(
            "a",
            {
              href: `/extensions/${EXTENSION_ID}/wiresheet/${d.device_id}`,
              className: "inline-flex items-center gap-1 text-primary hover:underline",
              title: "Open wiresheet",
              children: [
                /* @__PURE__ */ jsx(ExternalLink, { className: "size-4" }),
                " Wiresheet"
              ]
            }
          ),
          /* @__PURE__ */ jsx(
            "button",
            {
              type: "button",
              onClick: () => setEditing(d),
              className: "text-muted-foreground hover:text-foreground",
              title: "Edit",
              children: /* @__PURE__ */ jsx(Pencil, { className: "size-4" })
            }
          ),
          /* @__PURE__ */ jsx(
            "button",
            {
              type: "button",
              onClick: () => deleteDevice(d.device_id).then(reload),
              className: "text-muted-foreground hover:text-destructive",
              title: "Delete",
              children: /* @__PURE__ */ jsx(Trash2, { className: "size-4" })
            }
          )
        ] }) })
      ] }, d.device_id)) })
    ] }) })
  ] });
}
function Th({ children }) {
  return /* @__PURE__ */ jsx("th", { className: "px-3 py-2 text-left font-medium", children });
}
function Td$1({ children }) {
  return /* @__PURE__ */ jsx("td", { className: "px-3 py-2", children });
}

function cc(names) {
  if (typeof names === "string" || typeof names === "number") return "" + names

  let out = "";

  if (Array.isArray(names)) {
    for (let i = 0, tmp; i < names.length; i++) {
      if ((tmp = cc(names[i])) !== "") {
        out += (out && " ") + tmp;
      }
    }
  } else {
    for (let k in names) {
      if (names[k]) out += (out && " ") + k;
    }
  }

  return out
}

var noop = {value: () => {}};

function dispatch() {
  for (var i = 0, n = arguments.length, _ = {}, t; i < n; ++i) {
    if (!(t = arguments[i] + "") || (t in _) || /[\s.]/.test(t)) throw new Error("illegal type: " + t);
    _[t] = [];
  }
  return new Dispatch(_);
}

function Dispatch(_) {
  this._ = _;
}

function parseTypenames$1(typenames, types) {
  return typenames.trim().split(/^|\s+/).map(function(t) {
    var name = "", i = t.indexOf(".");
    if (i >= 0) name = t.slice(i + 1), t = t.slice(0, i);
    if (t && !types.hasOwnProperty(t)) throw new Error("unknown type: " + t);
    return {type: t, name: name};
  });
}

Dispatch.prototype = dispatch.prototype = {
  constructor: Dispatch,
  on: function(typename, callback) {
    var _ = this._,
        T = parseTypenames$1(typename + "", _),
        t,
        i = -1,
        n = T.length;

    // If no callback was specified, return the callback of the given type and name.
    if (arguments.length < 2) {
      while (++i < n) if ((t = (typename = T[i]).type) && (t = get$1(_[t], typename.name))) return t;
      return;
    }

    // If a type was specified, set the callback for the given type and name.
    // Otherwise, if a null callback was specified, remove callbacks of the given name.
    if (callback != null && typeof callback !== "function") throw new Error("invalid callback: " + callback);
    while (++i < n) {
      if (t = (typename = T[i]).type) _[t] = set$1(_[t], typename.name, callback);
      else if (callback == null) for (t in _) _[t] = set$1(_[t], typename.name, null);
    }

    return this;
  },
  copy: function() {
    var copy = {}, _ = this._;
    for (var t in _) copy[t] = _[t].slice();
    return new Dispatch(copy);
  },
  call: function(type, that) {
    if ((n = arguments.length - 2) > 0) for (var args = new Array(n), i = 0, n, t; i < n; ++i) args[i] = arguments[i + 2];
    if (!this._.hasOwnProperty(type)) throw new Error("unknown type: " + type);
    for (t = this._[type], i = 0, n = t.length; i < n; ++i) t[i].value.apply(that, args);
  },
  apply: function(type, that, args) {
    if (!this._.hasOwnProperty(type)) throw new Error("unknown type: " + type);
    for (var t = this._[type], i = 0, n = t.length; i < n; ++i) t[i].value.apply(that, args);
  }
};

function get$1(type, name) {
  for (var i = 0, n = type.length, c; i < n; ++i) {
    if ((c = type[i]).name === name) {
      return c.value;
    }
  }
}

function set$1(type, name, callback) {
  for (var i = 0, n = type.length; i < n; ++i) {
    if (type[i].name === name) {
      type[i] = noop, type = type.slice(0, i).concat(type.slice(i + 1));
      break;
    }
  }
  if (callback != null) type.push({name: name, value: callback});
  return type;
}

var xhtml = "http://www.w3.org/1999/xhtml";

const namespaces = {
  svg: "http://www.w3.org/2000/svg",
  xhtml: xhtml,
  xlink: "http://www.w3.org/1999/xlink",
  xml: "http://www.w3.org/XML/1998/namespace",
  xmlns: "http://www.w3.org/2000/xmlns/"
};

function namespace(name) {
  var prefix = name += "", i = prefix.indexOf(":");
  if (i >= 0 && (prefix = name.slice(0, i)) !== "xmlns") name = name.slice(i + 1);
  return namespaces.hasOwnProperty(prefix) ? {space: namespaces[prefix], local: name} : name; // eslint-disable-line no-prototype-builtins
}

function creatorInherit(name) {
  return function() {
    var document = this.ownerDocument,
        uri = this.namespaceURI;
    return uri === xhtml && document.documentElement.namespaceURI === xhtml
        ? document.createElement(name)
        : document.createElementNS(uri, name);
  };
}

function creatorFixed(fullname) {
  return function() {
    return this.ownerDocument.createElementNS(fullname.space, fullname.local);
  };
}

function creator(name) {
  var fullname = namespace(name);
  return (fullname.local
      ? creatorFixed
      : creatorInherit)(fullname);
}

function none() {}

function selector(selector) {
  return selector == null ? none : function() {
    return this.querySelector(selector);
  };
}

function selection_select(select) {
  if (typeof select !== "function") select = selector(select);

  for (var groups = this._groups, m = groups.length, subgroups = new Array(m), j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, subgroup = subgroups[j] = new Array(n), node, subnode, i = 0; i < n; ++i) {
      if ((node = group[i]) && (subnode = select.call(node, node.__data__, i, group))) {
        if ("__data__" in node) subnode.__data__ = node.__data__;
        subgroup[i] = subnode;
      }
    }
  }

  return new Selection$1(subgroups, this._parents);
}

// Given something array like (or null), returns something that is strictly an
// array. This is used to ensure that array-like objects passed to d3.selectAll
// or selection.selectAll are converted into proper arrays when creating a
// selection; we don’t ever want to create a selection backed by a live
// HTMLCollection or NodeList. However, note that selection.selectAll will use a
// static NodeList as a group, since it safely derived from querySelectorAll.
function array(x) {
  return x == null ? [] : Array.isArray(x) ? x : Array.from(x);
}

function empty() {
  return [];
}

function selectorAll(selector) {
  return selector == null ? empty : function() {
    return this.querySelectorAll(selector);
  };
}

function arrayAll(select) {
  return function() {
    return array(select.apply(this, arguments));
  };
}

function selection_selectAll(select) {
  if (typeof select === "function") select = arrayAll(select);
  else select = selectorAll(select);

  for (var groups = this._groups, m = groups.length, subgroups = [], parents = [], j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, node, i = 0; i < n; ++i) {
      if (node = group[i]) {
        subgroups.push(select.call(node, node.__data__, i, group));
        parents.push(node);
      }
    }
  }

  return new Selection$1(subgroups, parents);
}

function matcher(selector) {
  return function() {
    return this.matches(selector);
  };
}

function childMatcher(selector) {
  return function(node) {
    return node.matches(selector);
  };
}

var find = Array.prototype.find;

function childFind(match) {
  return function() {
    return find.call(this.children, match);
  };
}

function childFirst() {
  return this.firstElementChild;
}

function selection_selectChild(match) {
  return this.select(match == null ? childFirst
      : childFind(typeof match === "function" ? match : childMatcher(match)));
}

var filter = Array.prototype.filter;

function children() {
  return Array.from(this.children);
}

function childrenFilter(match) {
  return function() {
    return filter.call(this.children, match);
  };
}

function selection_selectChildren(match) {
  return this.selectAll(match == null ? children
      : childrenFilter(typeof match === "function" ? match : childMatcher(match)));
}

function selection_filter(match) {
  if (typeof match !== "function") match = matcher(match);

  for (var groups = this._groups, m = groups.length, subgroups = new Array(m), j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, subgroup = subgroups[j] = [], node, i = 0; i < n; ++i) {
      if ((node = group[i]) && match.call(node, node.__data__, i, group)) {
        subgroup.push(node);
      }
    }
  }

  return new Selection$1(subgroups, this._parents);
}

function sparse(update) {
  return new Array(update.length);
}

function selection_enter() {
  return new Selection$1(this._enter || this._groups.map(sparse), this._parents);
}

function EnterNode(parent, datum) {
  this.ownerDocument = parent.ownerDocument;
  this.namespaceURI = parent.namespaceURI;
  this._next = null;
  this._parent = parent;
  this.__data__ = datum;
}

EnterNode.prototype = {
  constructor: EnterNode,
  appendChild: function(child) { return this._parent.insertBefore(child, this._next); },
  insertBefore: function(child, next) { return this._parent.insertBefore(child, next); },
  querySelector: function(selector) { return this._parent.querySelector(selector); },
  querySelectorAll: function(selector) { return this._parent.querySelectorAll(selector); }
};

function constant$3(x) {
  return function() {
    return x;
  };
}

function bindIndex(parent, group, enter, update, exit, data) {
  var i = 0,
      node,
      groupLength = group.length,
      dataLength = data.length;

  // Put any non-null nodes that fit into update.
  // Put any null nodes into enter.
  // Put any remaining data into enter.
  for (; i < dataLength; ++i) {
    if (node = group[i]) {
      node.__data__ = data[i];
      update[i] = node;
    } else {
      enter[i] = new EnterNode(parent, data[i]);
    }
  }

  // Put any non-null nodes that don’t fit into exit.
  for (; i < groupLength; ++i) {
    if (node = group[i]) {
      exit[i] = node;
    }
  }
}

function bindKey(parent, group, enter, update, exit, data, key) {
  var i,
      node,
      nodeByKeyValue = new Map,
      groupLength = group.length,
      dataLength = data.length,
      keyValues = new Array(groupLength),
      keyValue;

  // Compute the key for each node.
  // If multiple nodes have the same key, the duplicates are added to exit.
  for (i = 0; i < groupLength; ++i) {
    if (node = group[i]) {
      keyValues[i] = keyValue = key.call(node, node.__data__, i, group) + "";
      if (nodeByKeyValue.has(keyValue)) {
        exit[i] = node;
      } else {
        nodeByKeyValue.set(keyValue, node);
      }
    }
  }

  // Compute the key for each datum.
  // If there a node associated with this key, join and add it to update.
  // If there is not (or the key is a duplicate), add it to enter.
  for (i = 0; i < dataLength; ++i) {
    keyValue = key.call(parent, data[i], i, data) + "";
    if (node = nodeByKeyValue.get(keyValue)) {
      update[i] = node;
      node.__data__ = data[i];
      nodeByKeyValue.delete(keyValue);
    } else {
      enter[i] = new EnterNode(parent, data[i]);
    }
  }

  // Add any remaining nodes that were not bound to data to exit.
  for (i = 0; i < groupLength; ++i) {
    if ((node = group[i]) && (nodeByKeyValue.get(keyValues[i]) === node)) {
      exit[i] = node;
    }
  }
}

function datum(node) {
  return node.__data__;
}

function selection_data(value, key) {
  if (!arguments.length) return Array.from(this, datum);

  var bind = key ? bindKey : bindIndex,
      parents = this._parents,
      groups = this._groups;

  if (typeof value !== "function") value = constant$3(value);

  for (var m = groups.length, update = new Array(m), enter = new Array(m), exit = new Array(m), j = 0; j < m; ++j) {
    var parent = parents[j],
        group = groups[j],
        groupLength = group.length,
        data = arraylike(value.call(parent, parent && parent.__data__, j, parents)),
        dataLength = data.length,
        enterGroup = enter[j] = new Array(dataLength),
        updateGroup = update[j] = new Array(dataLength),
        exitGroup = exit[j] = new Array(groupLength);

    bind(parent, group, enterGroup, updateGroup, exitGroup, data, key);

    // Now connect the enter nodes to their following update node, such that
    // appendChild can insert the materialized enter node before this node,
    // rather than at the end of the parent node.
    for (var i0 = 0, i1 = 0, previous, next; i0 < dataLength; ++i0) {
      if (previous = enterGroup[i0]) {
        if (i0 >= i1) i1 = i0 + 1;
        while (!(next = updateGroup[i1]) && ++i1 < dataLength);
        previous._next = next || null;
      }
    }
  }

  update = new Selection$1(update, parents);
  update._enter = enter;
  update._exit = exit;
  return update;
}

// Given some data, this returns an array-like view of it: an object that
// exposes a length property and allows numeric indexing. Note that unlike
// selectAll, this isn’t worried about “live” collections because the resulting
// array will only be used briefly while data is being bound. (It is possible to
// cause the data to change while iterating by using a key function, but please
// don’t; we’d rather avoid a gratuitous copy.)
function arraylike(data) {
  return typeof data === "object" && "length" in data
    ? data // Array, TypedArray, NodeList, array-like
    : Array.from(data); // Map, Set, iterable, string, or anything else
}

function selection_exit() {
  return new Selection$1(this._exit || this._groups.map(sparse), this._parents);
}

function selection_join(onenter, onupdate, onexit) {
  var enter = this.enter(), update = this, exit = this.exit();
  if (typeof onenter === "function") {
    enter = onenter(enter);
    if (enter) enter = enter.selection();
  } else {
    enter = enter.append(onenter + "");
  }
  if (onupdate != null) {
    update = onupdate(update);
    if (update) update = update.selection();
  }
  if (onexit == null) exit.remove(); else onexit(exit);
  return enter && update ? enter.merge(update).order() : update;
}

function selection_merge(context) {
  var selection = context.selection ? context.selection() : context;

  for (var groups0 = this._groups, groups1 = selection._groups, m0 = groups0.length, m1 = groups1.length, m = Math.min(m0, m1), merges = new Array(m0), j = 0; j < m; ++j) {
    for (var group0 = groups0[j], group1 = groups1[j], n = group0.length, merge = merges[j] = new Array(n), node, i = 0; i < n; ++i) {
      if (node = group0[i] || group1[i]) {
        merge[i] = node;
      }
    }
  }

  for (; j < m0; ++j) {
    merges[j] = groups0[j];
  }

  return new Selection$1(merges, this._parents);
}

function selection_order() {

  for (var groups = this._groups, j = -1, m = groups.length; ++j < m;) {
    for (var group = groups[j], i = group.length - 1, next = group[i], node; --i >= 0;) {
      if (node = group[i]) {
        if (next && node.compareDocumentPosition(next) ^ 4) next.parentNode.insertBefore(node, next);
        next = node;
      }
    }
  }

  return this;
}

function selection_sort(compare) {
  if (!compare) compare = ascending;

  function compareNode(a, b) {
    return a && b ? compare(a.__data__, b.__data__) : !a - !b;
  }

  for (var groups = this._groups, m = groups.length, sortgroups = new Array(m), j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, sortgroup = sortgroups[j] = new Array(n), node, i = 0; i < n; ++i) {
      if (node = group[i]) {
        sortgroup[i] = node;
      }
    }
    sortgroup.sort(compareNode);
  }

  return new Selection$1(sortgroups, this._parents).order();
}

function ascending(a, b) {
  return a < b ? -1 : a > b ? 1 : a >= b ? 0 : NaN;
}

function selection_call() {
  var callback = arguments[0];
  arguments[0] = this;
  callback.apply(null, arguments);
  return this;
}

function selection_nodes() {
  return Array.from(this);
}

function selection_node() {

  for (var groups = this._groups, j = 0, m = groups.length; j < m; ++j) {
    for (var group = groups[j], i = 0, n = group.length; i < n; ++i) {
      var node = group[i];
      if (node) return node;
    }
  }

  return null;
}

function selection_size() {
  let size = 0;
  for (const node of this) ++size; // eslint-disable-line no-unused-vars
  return size;
}

function selection_empty() {
  return !this.node();
}

function selection_each(callback) {

  for (var groups = this._groups, j = 0, m = groups.length; j < m; ++j) {
    for (var group = groups[j], i = 0, n = group.length, node; i < n; ++i) {
      if (node = group[i]) callback.call(node, node.__data__, i, group);
    }
  }

  return this;
}

function attrRemove$1(name) {
  return function() {
    this.removeAttribute(name);
  };
}

function attrRemoveNS$1(fullname) {
  return function() {
    this.removeAttributeNS(fullname.space, fullname.local);
  };
}

function attrConstant$1(name, value) {
  return function() {
    this.setAttribute(name, value);
  };
}

function attrConstantNS$1(fullname, value) {
  return function() {
    this.setAttributeNS(fullname.space, fullname.local, value);
  };
}

function attrFunction$1(name, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null) this.removeAttribute(name);
    else this.setAttribute(name, v);
  };
}

function attrFunctionNS$1(fullname, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null) this.removeAttributeNS(fullname.space, fullname.local);
    else this.setAttributeNS(fullname.space, fullname.local, v);
  };
}

function selection_attr(name, value) {
  var fullname = namespace(name);

  if (arguments.length < 2) {
    var node = this.node();
    return fullname.local
        ? node.getAttributeNS(fullname.space, fullname.local)
        : node.getAttribute(fullname);
  }

  return this.each((value == null
      ? (fullname.local ? attrRemoveNS$1 : attrRemove$1) : (typeof value === "function"
      ? (fullname.local ? attrFunctionNS$1 : attrFunction$1)
      : (fullname.local ? attrConstantNS$1 : attrConstant$1)))(fullname, value));
}

function defaultView(node) {
  return (node.ownerDocument && node.ownerDocument.defaultView) // node is a Node
      || (node.document && node) // node is a Window
      || node.defaultView; // node is a Document
}

function styleRemove$1(name) {
  return function() {
    this.style.removeProperty(name);
  };
}

function styleConstant$1(name, value, priority) {
  return function() {
    this.style.setProperty(name, value, priority);
  };
}

function styleFunction$1(name, value, priority) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null) this.style.removeProperty(name);
    else this.style.setProperty(name, v, priority);
  };
}

function selection_style(name, value, priority) {
  return arguments.length > 1
      ? this.each((value == null
            ? styleRemove$1 : typeof value === "function"
            ? styleFunction$1
            : styleConstant$1)(name, value, priority == null ? "" : priority))
      : styleValue(this.node(), name);
}

function styleValue(node, name) {
  return node.style.getPropertyValue(name)
      || defaultView(node).getComputedStyle(node, null).getPropertyValue(name);
}

function propertyRemove(name) {
  return function() {
    delete this[name];
  };
}

function propertyConstant(name, value) {
  return function() {
    this[name] = value;
  };
}

function propertyFunction(name, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (v == null) delete this[name];
    else this[name] = v;
  };
}

function selection_property(name, value) {
  return arguments.length > 1
      ? this.each((value == null
          ? propertyRemove : typeof value === "function"
          ? propertyFunction
          : propertyConstant)(name, value))
      : this.node()[name];
}

function classArray(string) {
  return string.trim().split(/^|\s+/);
}

function classList(node) {
  return node.classList || new ClassList(node);
}

function ClassList(node) {
  this._node = node;
  this._names = classArray(node.getAttribute("class") || "");
}

ClassList.prototype = {
  add: function(name) {
    var i = this._names.indexOf(name);
    if (i < 0) {
      this._names.push(name);
      this._node.setAttribute("class", this._names.join(" "));
    }
  },
  remove: function(name) {
    var i = this._names.indexOf(name);
    if (i >= 0) {
      this._names.splice(i, 1);
      this._node.setAttribute("class", this._names.join(" "));
    }
  },
  contains: function(name) {
    return this._names.indexOf(name) >= 0;
  }
};

function classedAdd(node, names) {
  var list = classList(node), i = -1, n = names.length;
  while (++i < n) list.add(names[i]);
}

function classedRemove(node, names) {
  var list = classList(node), i = -1, n = names.length;
  while (++i < n) list.remove(names[i]);
}

function classedTrue(names) {
  return function() {
    classedAdd(this, names);
  };
}

function classedFalse(names) {
  return function() {
    classedRemove(this, names);
  };
}

function classedFunction(names, value) {
  return function() {
    (value.apply(this, arguments) ? classedAdd : classedRemove)(this, names);
  };
}

function selection_classed(name, value) {
  var names = classArray(name + "");

  if (arguments.length < 2) {
    var list = classList(this.node()), i = -1, n = names.length;
    while (++i < n) if (!list.contains(names[i])) return false;
    return true;
  }

  return this.each((typeof value === "function"
      ? classedFunction : value
      ? classedTrue
      : classedFalse)(names, value));
}

function textRemove() {
  this.textContent = "";
}

function textConstant$1(value) {
  return function() {
    this.textContent = value;
  };
}

function textFunction$1(value) {
  return function() {
    var v = value.apply(this, arguments);
    this.textContent = v == null ? "" : v;
  };
}

function selection_text(value) {
  return arguments.length
      ? this.each(value == null
          ? textRemove : (typeof value === "function"
          ? textFunction$1
          : textConstant$1)(value))
      : this.node().textContent;
}

function htmlRemove() {
  this.innerHTML = "";
}

function htmlConstant(value) {
  return function() {
    this.innerHTML = value;
  };
}

function htmlFunction(value) {
  return function() {
    var v = value.apply(this, arguments);
    this.innerHTML = v == null ? "" : v;
  };
}

function selection_html(value) {
  return arguments.length
      ? this.each(value == null
          ? htmlRemove : (typeof value === "function"
          ? htmlFunction
          : htmlConstant)(value))
      : this.node().innerHTML;
}

function raise() {
  if (this.nextSibling) this.parentNode.appendChild(this);
}

function selection_raise() {
  return this.each(raise);
}

function lower() {
  if (this.previousSibling) this.parentNode.insertBefore(this, this.parentNode.firstChild);
}

function selection_lower() {
  return this.each(lower);
}

function selection_append(name) {
  var create = typeof name === "function" ? name : creator(name);
  return this.select(function() {
    return this.appendChild(create.apply(this, arguments));
  });
}

function constantNull() {
  return null;
}

function selection_insert(name, before) {
  var create = typeof name === "function" ? name : creator(name),
      select = before == null ? constantNull : typeof before === "function" ? before : selector(before);
  return this.select(function() {
    return this.insertBefore(create.apply(this, arguments), select.apply(this, arguments) || null);
  });
}

function remove() {
  var parent = this.parentNode;
  if (parent) parent.removeChild(this);
}

function selection_remove() {
  return this.each(remove);
}

function selection_cloneShallow() {
  var clone = this.cloneNode(false), parent = this.parentNode;
  return parent ? parent.insertBefore(clone, this.nextSibling) : clone;
}

function selection_cloneDeep() {
  var clone = this.cloneNode(true), parent = this.parentNode;
  return parent ? parent.insertBefore(clone, this.nextSibling) : clone;
}

function selection_clone(deep) {
  return this.select(deep ? selection_cloneDeep : selection_cloneShallow);
}

function selection_datum(value) {
  return arguments.length
      ? this.property("__data__", value)
      : this.node().__data__;
}

function contextListener(listener) {
  return function(event) {
    listener.call(this, event, this.__data__);
  };
}

function parseTypenames(typenames) {
  return typenames.trim().split(/^|\s+/).map(function(t) {
    var name = "", i = t.indexOf(".");
    if (i >= 0) name = t.slice(i + 1), t = t.slice(0, i);
    return {type: t, name: name};
  });
}

function onRemove(typename) {
  return function() {
    var on = this.__on;
    if (!on) return;
    for (var j = 0, i = -1, m = on.length, o; j < m; ++j) {
      if (o = on[j], (!typename.type || o.type === typename.type) && o.name === typename.name) {
        this.removeEventListener(o.type, o.listener, o.options);
      } else {
        on[++i] = o;
      }
    }
    if (++i) on.length = i;
    else delete this.__on;
  };
}

function onAdd(typename, value, options) {
  return function() {
    var on = this.__on, o, listener = contextListener(value);
    if (on) for (var j = 0, m = on.length; j < m; ++j) {
      if ((o = on[j]).type === typename.type && o.name === typename.name) {
        this.removeEventListener(o.type, o.listener, o.options);
        this.addEventListener(o.type, o.listener = listener, o.options = options);
        o.value = value;
        return;
      }
    }
    this.addEventListener(typename.type, listener, options);
    o = {type: typename.type, name: typename.name, value: value, listener: listener, options: options};
    if (!on) this.__on = [o];
    else on.push(o);
  };
}

function selection_on(typename, value, options) {
  var typenames = parseTypenames(typename + ""), i, n = typenames.length, t;

  if (arguments.length < 2) {
    var on = this.node().__on;
    if (on) for (var j = 0, m = on.length, o; j < m; ++j) {
      for (i = 0, o = on[j]; i < n; ++i) {
        if ((t = typenames[i]).type === o.type && t.name === o.name) {
          return o.value;
        }
      }
    }
    return;
  }

  on = value ? onAdd : onRemove;
  for (i = 0; i < n; ++i) this.each(on(typenames[i], value, options));
  return this;
}

function dispatchEvent(node, type, params) {
  var window = defaultView(node),
      event = window.CustomEvent;

  if (typeof event === "function") {
    event = new event(type, params);
  } else {
    event = window.document.createEvent("Event");
    if (params) event.initEvent(type, params.bubbles, params.cancelable), event.detail = params.detail;
    else event.initEvent(type, false, false);
  }

  node.dispatchEvent(event);
}

function dispatchConstant(type, params) {
  return function() {
    return dispatchEvent(this, type, params);
  };
}

function dispatchFunction(type, params) {
  return function() {
    return dispatchEvent(this, type, params.apply(this, arguments));
  };
}

function selection_dispatch(type, params) {
  return this.each((typeof params === "function"
      ? dispatchFunction
      : dispatchConstant)(type, params));
}

function* selection_iterator() {
  for (var groups = this._groups, j = 0, m = groups.length; j < m; ++j) {
    for (var group = groups[j], i = 0, n = group.length, node; i < n; ++i) {
      if (node = group[i]) yield node;
    }
  }
}

var root = [null];

function Selection$1(groups, parents) {
  this._groups = groups;
  this._parents = parents;
}

function selection() {
  return new Selection$1([[document.documentElement]], root);
}

function selection_selection() {
  return this;
}

Selection$1.prototype = selection.prototype = {
  constructor: Selection$1,
  select: selection_select,
  selectAll: selection_selectAll,
  selectChild: selection_selectChild,
  selectChildren: selection_selectChildren,
  filter: selection_filter,
  data: selection_data,
  enter: selection_enter,
  exit: selection_exit,
  join: selection_join,
  merge: selection_merge,
  selection: selection_selection,
  order: selection_order,
  sort: selection_sort,
  call: selection_call,
  nodes: selection_nodes,
  node: selection_node,
  size: selection_size,
  empty: selection_empty,
  each: selection_each,
  attr: selection_attr,
  style: selection_style,
  property: selection_property,
  classed: selection_classed,
  text: selection_text,
  html: selection_html,
  raise: selection_raise,
  lower: selection_lower,
  append: selection_append,
  insert: selection_insert,
  remove: selection_remove,
  clone: selection_clone,
  datum: selection_datum,
  on: selection_on,
  dispatch: selection_dispatch,
  [Symbol.iterator]: selection_iterator
};

function select(selector) {
  return typeof selector === "string"
      ? new Selection$1([[document.querySelector(selector)]], [document.documentElement])
      : new Selection$1([[selector]], root);
}

function sourceEvent(event) {
  let sourceEvent;
  while (sourceEvent = event.sourceEvent) event = sourceEvent;
  return event;
}

function pointer(event, node) {
  event = sourceEvent(event);
  if (node === undefined) node = event.currentTarget;
  if (node) {
    var svg = node.ownerSVGElement || node;
    if (svg.createSVGPoint) {
      var point = svg.createSVGPoint();
      point.x = event.clientX, point.y = event.clientY;
      point = point.matrixTransform(node.getScreenCTM().inverse());
      return [point.x, point.y];
    }
    if (node.getBoundingClientRect) {
      var rect = node.getBoundingClientRect();
      return [event.clientX - rect.left - node.clientLeft, event.clientY - rect.top - node.clientTop];
    }
  }
  return [event.pageX, event.pageY];
}

// These are typically used in conjunction with noevent to ensure that we can
// preventDefault on the event.
const nonpassive = {passive: false};
const nonpassivecapture = {capture: true, passive: false};

function nopropagation$1(event) {
  event.stopImmediatePropagation();
}

function noevent$1(event) {
  event.preventDefault();
  event.stopImmediatePropagation();
}

function dragDisable(view) {
  var root = view.document.documentElement,
      selection = select(view).on("dragstart.drag", noevent$1, nonpassivecapture);
  if ("onselectstart" in root) {
    selection.on("selectstart.drag", noevent$1, nonpassivecapture);
  } else {
    root.__noselect = root.style.MozUserSelect;
    root.style.MozUserSelect = "none";
  }
}

function yesdrag(view, noclick) {
  var root = view.document.documentElement,
      selection = select(view).on("dragstart.drag", null);
  if (noclick) {
    selection.on("click.drag", noevent$1, nonpassivecapture);
    setTimeout(function() { selection.on("click.drag", null); }, 0);
  }
  if ("onselectstart" in root) {
    selection.on("selectstart.drag", null);
  } else {
    root.style.MozUserSelect = root.__noselect;
    delete root.__noselect;
  }
}

const constant$2 = x => () => x;

function DragEvent(type, {
  sourceEvent,
  subject,
  target,
  identifier,
  active,
  x, y, dx, dy,
  dispatch
}) {
  Object.defineProperties(this, {
    type: {value: type, enumerable: true, configurable: true},
    sourceEvent: {value: sourceEvent, enumerable: true, configurable: true},
    subject: {value: subject, enumerable: true, configurable: true},
    target: {value: target, enumerable: true, configurable: true},
    identifier: {value: identifier, enumerable: true, configurable: true},
    active: {value: active, enumerable: true, configurable: true},
    x: {value: x, enumerable: true, configurable: true},
    y: {value: y, enumerable: true, configurable: true},
    dx: {value: dx, enumerable: true, configurable: true},
    dy: {value: dy, enumerable: true, configurable: true},
    _: {value: dispatch}
  });
}

DragEvent.prototype.on = function() {
  var value = this._.on.apply(this._, arguments);
  return value === this._ ? this : value;
};

// Ignore right-click, since that should open the context menu.
function defaultFilter$1(event) {
  return !event.ctrlKey && !event.button;
}

function defaultContainer() {
  return this.parentNode;
}

function defaultSubject(event, d) {
  return d == null ? {x: event.x, y: event.y} : d;
}

function defaultTouchable$1() {
  return navigator.maxTouchPoints || ("ontouchstart" in this);
}

function drag() {
  var filter = defaultFilter$1,
      container = defaultContainer,
      subject = defaultSubject,
      touchable = defaultTouchable$1,
      gestures = {},
      listeners = dispatch("start", "drag", "end"),
      active = 0,
      mousedownx,
      mousedowny,
      mousemoving,
      touchending,
      clickDistance2 = 0;

  function drag(selection) {
    selection
        .on("mousedown.drag", mousedowned)
      .filter(touchable)
        .on("touchstart.drag", touchstarted)
        .on("touchmove.drag", touchmoved, nonpassive)
        .on("touchend.drag touchcancel.drag", touchended)
        .style("touch-action", "none")
        .style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
  }

  function mousedowned(event, d) {
    if (touchending || !filter.call(this, event, d)) return;
    var gesture = beforestart(this, container.call(this, event, d), event, d, "mouse");
    if (!gesture) return;
    select(event.view)
      .on("mousemove.drag", mousemoved, nonpassivecapture)
      .on("mouseup.drag", mouseupped, nonpassivecapture);
    dragDisable(event.view);
    nopropagation$1(event);
    mousemoving = false;
    mousedownx = event.clientX;
    mousedowny = event.clientY;
    gesture("start", event);
  }

  function mousemoved(event) {
    noevent$1(event);
    if (!mousemoving) {
      var dx = event.clientX - mousedownx, dy = event.clientY - mousedowny;
      mousemoving = dx * dx + dy * dy > clickDistance2;
    }
    gestures.mouse("drag", event);
  }

  function mouseupped(event) {
    select(event.view).on("mousemove.drag mouseup.drag", null);
    yesdrag(event.view, mousemoving);
    noevent$1(event);
    gestures.mouse("end", event);
  }

  function touchstarted(event, d) {
    if (!filter.call(this, event, d)) return;
    var touches = event.changedTouches,
        c = container.call(this, event, d),
        n = touches.length, i, gesture;

    for (i = 0; i < n; ++i) {
      if (gesture = beforestart(this, c, event, d, touches[i].identifier, touches[i])) {
        nopropagation$1(event);
        gesture("start", event, touches[i]);
      }
    }
  }

  function touchmoved(event) {
    var touches = event.changedTouches,
        n = touches.length, i, gesture;

    for (i = 0; i < n; ++i) {
      if (gesture = gestures[touches[i].identifier]) {
        noevent$1(event);
        gesture("drag", event, touches[i]);
      }
    }
  }

  function touchended(event) {
    var touches = event.changedTouches,
        n = touches.length, i, gesture;

    if (touchending) clearTimeout(touchending);
    touchending = setTimeout(function() { touchending = null; }, 500); // Ghost clicks are delayed!
    for (i = 0; i < n; ++i) {
      if (gesture = gestures[touches[i].identifier]) {
        nopropagation$1(event);
        gesture("end", event, touches[i]);
      }
    }
  }

  function beforestart(that, container, event, d, identifier, touch) {
    var dispatch = listeners.copy(),
        p = pointer(touch || event, container), dx, dy,
        s;

    if ((s = subject.call(that, new DragEvent("beforestart", {
        sourceEvent: event,
        target: drag,
        identifier,
        active,
        x: p[0],
        y: p[1],
        dx: 0,
        dy: 0,
        dispatch
      }), d)) == null) return;

    dx = s.x - p[0] || 0;
    dy = s.y - p[1] || 0;

    return function gesture(type, event, touch) {
      var p0 = p, n;
      switch (type) {
        case "start": gestures[identifier] = gesture, n = active++; break;
        case "end": delete gestures[identifier], --active; // falls through
        case "drag": p = pointer(touch || event, container), n = active; break;
      }
      dispatch.call(
        type,
        that,
        new DragEvent(type, {
          sourceEvent: event,
          subject: s,
          target: drag,
          identifier,
          active: n,
          x: p[0] + dx,
          y: p[1] + dy,
          dx: p[0] - p0[0],
          dy: p[1] - p0[1],
          dispatch
        }),
        d
      );
    };
  }

  drag.filter = function(_) {
    return arguments.length ? (filter = typeof _ === "function" ? _ : constant$2(!!_), drag) : filter;
  };

  drag.container = function(_) {
    return arguments.length ? (container = typeof _ === "function" ? _ : constant$2(_), drag) : container;
  };

  drag.subject = function(_) {
    return arguments.length ? (subject = typeof _ === "function" ? _ : constant$2(_), drag) : subject;
  };

  drag.touchable = function(_) {
    return arguments.length ? (touchable = typeof _ === "function" ? _ : constant$2(!!_), drag) : touchable;
  };

  drag.on = function() {
    var value = listeners.on.apply(listeners, arguments);
    return value === listeners ? drag : value;
  };

  drag.clickDistance = function(_) {
    return arguments.length ? (clickDistance2 = (_ = +_) * _, drag) : Math.sqrt(clickDistance2);
  };

  return drag;
}

function define(constructor, factory, prototype) {
  constructor.prototype = factory.prototype = prototype;
  prototype.constructor = constructor;
}

function extend(parent, definition) {
  var prototype = Object.create(parent.prototype);
  for (var key in definition) prototype[key] = definition[key];
  return prototype;
}

function Color() {}

var darker = 0.7;
var brighter = 1 / darker;

var reI = "\\s*([+-]?\\d+)\\s*",
    reN = "\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)\\s*",
    reP = "\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)%\\s*",
    reHex = /^#([0-9a-f]{3,8})$/,
    reRgbInteger = new RegExp(`^rgb\\(${reI},${reI},${reI}\\)$`),
    reRgbPercent = new RegExp(`^rgb\\(${reP},${reP},${reP}\\)$`),
    reRgbaInteger = new RegExp(`^rgba\\(${reI},${reI},${reI},${reN}\\)$`),
    reRgbaPercent = new RegExp(`^rgba\\(${reP},${reP},${reP},${reN}\\)$`),
    reHslPercent = new RegExp(`^hsl\\(${reN},${reP},${reP}\\)$`),
    reHslaPercent = new RegExp(`^hsla\\(${reN},${reP},${reP},${reN}\\)$`);

var named = {
  aliceblue: 0xf0f8ff,
  antiquewhite: 0xfaebd7,
  aqua: 0x00ffff,
  aquamarine: 0x7fffd4,
  azure: 0xf0ffff,
  beige: 0xf5f5dc,
  bisque: 0xffe4c4,
  black: 0x000000,
  blanchedalmond: 0xffebcd,
  blue: 0x0000ff,
  blueviolet: 0x8a2be2,
  brown: 0xa52a2a,
  burlywood: 0xdeb887,
  cadetblue: 0x5f9ea0,
  chartreuse: 0x7fff00,
  chocolate: 0xd2691e,
  coral: 0xff7f50,
  cornflowerblue: 0x6495ed,
  cornsilk: 0xfff8dc,
  crimson: 0xdc143c,
  cyan: 0x00ffff,
  darkblue: 0x00008b,
  darkcyan: 0x008b8b,
  darkgoldenrod: 0xb8860b,
  darkgray: 0xa9a9a9,
  darkgreen: 0x006400,
  darkgrey: 0xa9a9a9,
  darkkhaki: 0xbdb76b,
  darkmagenta: 0x8b008b,
  darkolivegreen: 0x556b2f,
  darkorange: 0xff8c00,
  darkorchid: 0x9932cc,
  darkred: 0x8b0000,
  darksalmon: 0xe9967a,
  darkseagreen: 0x8fbc8f,
  darkslateblue: 0x483d8b,
  darkslategray: 0x2f4f4f,
  darkslategrey: 0x2f4f4f,
  darkturquoise: 0x00ced1,
  darkviolet: 0x9400d3,
  deeppink: 0xff1493,
  deepskyblue: 0x00bfff,
  dimgray: 0x696969,
  dimgrey: 0x696969,
  dodgerblue: 0x1e90ff,
  firebrick: 0xb22222,
  floralwhite: 0xfffaf0,
  forestgreen: 0x228b22,
  fuchsia: 0xff00ff,
  gainsboro: 0xdcdcdc,
  ghostwhite: 0xf8f8ff,
  gold: 0xffd700,
  goldenrod: 0xdaa520,
  gray: 0x808080,
  green: 0x008000,
  greenyellow: 0xadff2f,
  grey: 0x808080,
  honeydew: 0xf0fff0,
  hotpink: 0xff69b4,
  indianred: 0xcd5c5c,
  indigo: 0x4b0082,
  ivory: 0xfffff0,
  khaki: 0xf0e68c,
  lavender: 0xe6e6fa,
  lavenderblush: 0xfff0f5,
  lawngreen: 0x7cfc00,
  lemonchiffon: 0xfffacd,
  lightblue: 0xadd8e6,
  lightcoral: 0xf08080,
  lightcyan: 0xe0ffff,
  lightgoldenrodyellow: 0xfafad2,
  lightgray: 0xd3d3d3,
  lightgreen: 0x90ee90,
  lightgrey: 0xd3d3d3,
  lightpink: 0xffb6c1,
  lightsalmon: 0xffa07a,
  lightseagreen: 0x20b2aa,
  lightskyblue: 0x87cefa,
  lightslategray: 0x778899,
  lightslategrey: 0x778899,
  lightsteelblue: 0xb0c4de,
  lightyellow: 0xffffe0,
  lime: 0x00ff00,
  limegreen: 0x32cd32,
  linen: 0xfaf0e6,
  magenta: 0xff00ff,
  maroon: 0x800000,
  mediumaquamarine: 0x66cdaa,
  mediumblue: 0x0000cd,
  mediumorchid: 0xba55d3,
  mediumpurple: 0x9370db,
  mediumseagreen: 0x3cb371,
  mediumslateblue: 0x7b68ee,
  mediumspringgreen: 0x00fa9a,
  mediumturquoise: 0x48d1cc,
  mediumvioletred: 0xc71585,
  midnightblue: 0x191970,
  mintcream: 0xf5fffa,
  mistyrose: 0xffe4e1,
  moccasin: 0xffe4b5,
  navajowhite: 0xffdead,
  navy: 0x000080,
  oldlace: 0xfdf5e6,
  olive: 0x808000,
  olivedrab: 0x6b8e23,
  orange: 0xffa500,
  orangered: 0xff4500,
  orchid: 0xda70d6,
  palegoldenrod: 0xeee8aa,
  palegreen: 0x98fb98,
  paleturquoise: 0xafeeee,
  palevioletred: 0xdb7093,
  papayawhip: 0xffefd5,
  peachpuff: 0xffdab9,
  peru: 0xcd853f,
  pink: 0xffc0cb,
  plum: 0xdda0dd,
  powderblue: 0xb0e0e6,
  purple: 0x800080,
  rebeccapurple: 0x663399,
  red: 0xff0000,
  rosybrown: 0xbc8f8f,
  royalblue: 0x4169e1,
  saddlebrown: 0x8b4513,
  salmon: 0xfa8072,
  sandybrown: 0xf4a460,
  seagreen: 0x2e8b57,
  seashell: 0xfff5ee,
  sienna: 0xa0522d,
  silver: 0xc0c0c0,
  skyblue: 0x87ceeb,
  slateblue: 0x6a5acd,
  slategray: 0x708090,
  slategrey: 0x708090,
  snow: 0xfffafa,
  springgreen: 0x00ff7f,
  steelblue: 0x4682b4,
  tan: 0xd2b48c,
  teal: 0x008080,
  thistle: 0xd8bfd8,
  tomato: 0xff6347,
  turquoise: 0x40e0d0,
  violet: 0xee82ee,
  wheat: 0xf5deb3,
  white: 0xffffff,
  whitesmoke: 0xf5f5f5,
  yellow: 0xffff00,
  yellowgreen: 0x9acd32
};

define(Color, color, {
  copy(channels) {
    return Object.assign(new this.constructor, this, channels);
  },
  displayable() {
    return this.rgb().displayable();
  },
  hex: color_formatHex, // Deprecated! Use color.formatHex.
  formatHex: color_formatHex,
  formatHex8: color_formatHex8,
  formatHsl: color_formatHsl,
  formatRgb: color_formatRgb,
  toString: color_formatRgb
});

function color_formatHex() {
  return this.rgb().formatHex();
}

function color_formatHex8() {
  return this.rgb().formatHex8();
}

function color_formatHsl() {
  return hslConvert(this).formatHsl();
}

function color_formatRgb() {
  return this.rgb().formatRgb();
}

function color(format) {
  var m, l;
  format = (format + "").trim().toLowerCase();
  return (m = reHex.exec(format)) ? (l = m[1].length, m = parseInt(m[1], 16), l === 6 ? rgbn(m) // #ff0000
      : l === 3 ? new Rgb((m >> 8 & 0xf) | (m >> 4 & 0xf0), (m >> 4 & 0xf) | (m & 0xf0), ((m & 0xf) << 4) | (m & 0xf), 1) // #f00
      : l === 8 ? rgba(m >> 24 & 0xff, m >> 16 & 0xff, m >> 8 & 0xff, (m & 0xff) / 0xff) // #ff000000
      : l === 4 ? rgba((m >> 12 & 0xf) | (m >> 8 & 0xf0), (m >> 8 & 0xf) | (m >> 4 & 0xf0), (m >> 4 & 0xf) | (m & 0xf0), (((m & 0xf) << 4) | (m & 0xf)) / 0xff) // #f000
      : null) // invalid hex
      : (m = reRgbInteger.exec(format)) ? new Rgb(m[1], m[2], m[3], 1) // rgb(255, 0, 0)
      : (m = reRgbPercent.exec(format)) ? new Rgb(m[1] * 255 / 100, m[2] * 255 / 100, m[3] * 255 / 100, 1) // rgb(100%, 0%, 0%)
      : (m = reRgbaInteger.exec(format)) ? rgba(m[1], m[2], m[3], m[4]) // rgba(255, 0, 0, 1)
      : (m = reRgbaPercent.exec(format)) ? rgba(m[1] * 255 / 100, m[2] * 255 / 100, m[3] * 255 / 100, m[4]) // rgb(100%, 0%, 0%, 1)
      : (m = reHslPercent.exec(format)) ? hsla(m[1], m[2] / 100, m[3] / 100, 1) // hsl(120, 50%, 50%)
      : (m = reHslaPercent.exec(format)) ? hsla(m[1], m[2] / 100, m[3] / 100, m[4]) // hsla(120, 50%, 50%, 1)
      : named.hasOwnProperty(format) ? rgbn(named[format]) // eslint-disable-line no-prototype-builtins
      : format === "transparent" ? new Rgb(NaN, NaN, NaN, 0)
      : null;
}

function rgbn(n) {
  return new Rgb(n >> 16 & 0xff, n >> 8 & 0xff, n & 0xff, 1);
}

function rgba(r, g, b, a) {
  if (a <= 0) r = g = b = NaN;
  return new Rgb(r, g, b, a);
}

function rgbConvert(o) {
  if (!(o instanceof Color)) o = color(o);
  if (!o) return new Rgb;
  o = o.rgb();
  return new Rgb(o.r, o.g, o.b, o.opacity);
}

function rgb(r, g, b, opacity) {
  return arguments.length === 1 ? rgbConvert(r) : new Rgb(r, g, b, opacity == null ? 1 : opacity);
}

function Rgb(r, g, b, opacity) {
  this.r = +r;
  this.g = +g;
  this.b = +b;
  this.opacity = +opacity;
}

define(Rgb, rgb, extend(Color, {
  brighter(k) {
    k = k == null ? brighter : Math.pow(brighter, k);
    return new Rgb(this.r * k, this.g * k, this.b * k, this.opacity);
  },
  darker(k) {
    k = k == null ? darker : Math.pow(darker, k);
    return new Rgb(this.r * k, this.g * k, this.b * k, this.opacity);
  },
  rgb() {
    return this;
  },
  clamp() {
    return new Rgb(clampi(this.r), clampi(this.g), clampi(this.b), clampa(this.opacity));
  },
  displayable() {
    return (-0.5 <= this.r && this.r < 255.5)
        && (-0.5 <= this.g && this.g < 255.5)
        && (-0.5 <= this.b && this.b < 255.5)
        && (0 <= this.opacity && this.opacity <= 1);
  },
  hex: rgb_formatHex, // Deprecated! Use color.formatHex.
  formatHex: rgb_formatHex,
  formatHex8: rgb_formatHex8,
  formatRgb: rgb_formatRgb,
  toString: rgb_formatRgb
}));

function rgb_formatHex() {
  return `#${hex(this.r)}${hex(this.g)}${hex(this.b)}`;
}

function rgb_formatHex8() {
  return `#${hex(this.r)}${hex(this.g)}${hex(this.b)}${hex((isNaN(this.opacity) ? 1 : this.opacity) * 255)}`;
}

function rgb_formatRgb() {
  const a = clampa(this.opacity);
  return `${a === 1 ? "rgb(" : "rgba("}${clampi(this.r)}, ${clampi(this.g)}, ${clampi(this.b)}${a === 1 ? ")" : `, ${a})`}`;
}

function clampa(opacity) {
  return isNaN(opacity) ? 1 : Math.max(0, Math.min(1, opacity));
}

function clampi(value) {
  return Math.max(0, Math.min(255, Math.round(value) || 0));
}

function hex(value) {
  value = clampi(value);
  return (value < 16 ? "0" : "") + value.toString(16);
}

function hsla(h, s, l, a) {
  if (a <= 0) h = s = l = NaN;
  else if (l <= 0 || l >= 1) h = s = NaN;
  else if (s <= 0) h = NaN;
  return new Hsl(h, s, l, a);
}

function hslConvert(o) {
  if (o instanceof Hsl) return new Hsl(o.h, o.s, o.l, o.opacity);
  if (!(o instanceof Color)) o = color(o);
  if (!o) return new Hsl;
  if (o instanceof Hsl) return o;
  o = o.rgb();
  var r = o.r / 255,
      g = o.g / 255,
      b = o.b / 255,
      min = Math.min(r, g, b),
      max = Math.max(r, g, b),
      h = NaN,
      s = max - min,
      l = (max + min) / 2;
  if (s) {
    if (r === max) h = (g - b) / s + (g < b) * 6;
    else if (g === max) h = (b - r) / s + 2;
    else h = (r - g) / s + 4;
    s /= l < 0.5 ? max + min : 2 - max - min;
    h *= 60;
  } else {
    s = l > 0 && l < 1 ? 0 : h;
  }
  return new Hsl(h, s, l, o.opacity);
}

function hsl(h, s, l, opacity) {
  return arguments.length === 1 ? hslConvert(h) : new Hsl(h, s, l, opacity == null ? 1 : opacity);
}

function Hsl(h, s, l, opacity) {
  this.h = +h;
  this.s = +s;
  this.l = +l;
  this.opacity = +opacity;
}

define(Hsl, hsl, extend(Color, {
  brighter(k) {
    k = k == null ? brighter : Math.pow(brighter, k);
    return new Hsl(this.h, this.s, this.l * k, this.opacity);
  },
  darker(k) {
    k = k == null ? darker : Math.pow(darker, k);
    return new Hsl(this.h, this.s, this.l * k, this.opacity);
  },
  rgb() {
    var h = this.h % 360 + (this.h < 0) * 360,
        s = isNaN(h) || isNaN(this.s) ? 0 : this.s,
        l = this.l,
        m2 = l + (l < 0.5 ? l : 1 - l) * s,
        m1 = 2 * l - m2;
    return new Rgb(
      hsl2rgb(h >= 240 ? h - 240 : h + 120, m1, m2),
      hsl2rgb(h, m1, m2),
      hsl2rgb(h < 120 ? h + 240 : h - 120, m1, m2),
      this.opacity
    );
  },
  clamp() {
    return new Hsl(clamph(this.h), clampt(this.s), clampt(this.l), clampa(this.opacity));
  },
  displayable() {
    return (0 <= this.s && this.s <= 1 || isNaN(this.s))
        && (0 <= this.l && this.l <= 1)
        && (0 <= this.opacity && this.opacity <= 1);
  },
  formatHsl() {
    const a = clampa(this.opacity);
    return `${a === 1 ? "hsl(" : "hsla("}${clamph(this.h)}, ${clampt(this.s) * 100}%, ${clampt(this.l) * 100}%${a === 1 ? ")" : `, ${a})`}`;
  }
}));

function clamph(value) {
  value = (value || 0) % 360;
  return value < 0 ? value + 360 : value;
}

function clampt(value) {
  return Math.max(0, Math.min(1, value || 0));
}

/* From FvD 13.37, CSS Color Module Level 3 */
function hsl2rgb(h, m1, m2) {
  return (h < 60 ? m1 + (m2 - m1) * h / 60
      : h < 180 ? m2
      : h < 240 ? m1 + (m2 - m1) * (240 - h) / 60
      : m1) * 255;
}

const constant$1 = x => () => x;

function linear(a, d) {
  return function(t) {
    return a + t * d;
  };
}

function exponential(a, b, y) {
  return a = Math.pow(a, y), b = Math.pow(b, y) - a, y = 1 / y, function(t) {
    return Math.pow(a + t * b, y);
  };
}

function gamma(y) {
  return (y = +y) === 1 ? nogamma : function(a, b) {
    return b - a ? exponential(a, b, y) : constant$1(isNaN(a) ? b : a);
  };
}

function nogamma(a, b) {
  var d = b - a;
  return d ? linear(a, d) : constant$1(isNaN(a) ? b : a);
}

const interpolateRgb = (function rgbGamma(y) {
  var color = gamma(y);

  function rgb$1(start, end) {
    var r = color((start = rgb(start)).r, (end = rgb(end)).r),
        g = color(start.g, end.g),
        b = color(start.b, end.b),
        opacity = nogamma(start.opacity, end.opacity);
    return function(t) {
      start.r = r(t);
      start.g = g(t);
      start.b = b(t);
      start.opacity = opacity(t);
      return start + "";
    };
  }

  rgb$1.gamma = rgbGamma;

  return rgb$1;
})(1);

function numberArray(a, b) {
  if (!b) b = [];
  var n = a ? Math.min(b.length, a.length) : 0,
      c = b.slice(),
      i;
  return function(t) {
    for (i = 0; i < n; ++i) c[i] = a[i] * (1 - t) + b[i] * t;
    return c;
  };
}

function isNumberArray(x) {
  return ArrayBuffer.isView(x) && !(x instanceof DataView);
}

function genericArray(a, b) {
  var nb = b ? b.length : 0,
      na = a ? Math.min(nb, a.length) : 0,
      x = new Array(na),
      c = new Array(nb),
      i;

  for (i = 0; i < na; ++i) x[i] = interpolate$1(a[i], b[i]);
  for (; i < nb; ++i) c[i] = b[i];

  return function(t) {
    for (i = 0; i < na; ++i) c[i] = x[i](t);
    return c;
  };
}

function date(a, b) {
  var d = new Date;
  return a = +a, b = +b, function(t) {
    return d.setTime(a * (1 - t) + b * t), d;
  };
}

function interpolateNumber(a, b) {
  return a = +a, b = +b, function(t) {
    return a * (1 - t) + b * t;
  };
}

function object(a, b) {
  var i = {},
      c = {},
      k;

  if (a === null || typeof a !== "object") a = {};
  if (b === null || typeof b !== "object") b = {};

  for (k in b) {
    if (k in a) {
      i[k] = interpolate$1(a[k], b[k]);
    } else {
      c[k] = b[k];
    }
  }

  return function(t) {
    for (k in i) c[k] = i[k](t);
    return c;
  };
}

var reA = /[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?/g,
    reB = new RegExp(reA.source, "g");

function zero(b) {
  return function() {
    return b;
  };
}

function one(b) {
  return function(t) {
    return b(t) + "";
  };
}

function interpolateString(a, b) {
  var bi = reA.lastIndex = reB.lastIndex = 0, // scan index for next number in b
      am, // current match in a
      bm, // current match in b
      bs, // string preceding current number in b, if any
      i = -1, // index in s
      s = [], // string constants and placeholders
      q = []; // number interpolators

  // Coerce inputs to strings.
  a = a + "", b = b + "";

  // Interpolate pairs of numbers in a & b.
  while ((am = reA.exec(a))
      && (bm = reB.exec(b))) {
    if ((bs = bm.index) > bi) { // a string precedes the next number in b
      bs = b.slice(bi, bs);
      if (s[i]) s[i] += bs; // coalesce with previous string
      else s[++i] = bs;
    }
    if ((am = am[0]) === (bm = bm[0])) { // numbers in a & b match
      if (s[i]) s[i] += bm; // coalesce with previous string
      else s[++i] = bm;
    } else { // interpolate non-matching numbers
      s[++i] = null;
      q.push({i: i, x: interpolateNumber(am, bm)});
    }
    bi = reB.lastIndex;
  }

  // Add remains of b.
  if (bi < b.length) {
    bs = b.slice(bi);
    if (s[i]) s[i] += bs; // coalesce with previous string
    else s[++i] = bs;
  }

  // Special optimization for only a single match.
  // Otherwise, interpolate each of the numbers and rejoin the string.
  return s.length < 2 ? (q[0]
      ? one(q[0].x)
      : zero(b))
      : (b = q.length, function(t) {
          for (var i = 0, o; i < b; ++i) s[(o = q[i]).i] = o.x(t);
          return s.join("");
        });
}

function interpolate$1(a, b) {
  var t = typeof b, c;
  return b == null || t === "boolean" ? constant$1(b)
      : (t === "number" ? interpolateNumber
      : t === "string" ? ((c = color(b)) ? (b = c, interpolateRgb) : interpolateString)
      : b instanceof color ? interpolateRgb
      : b instanceof Date ? date
      : isNumberArray(b) ? numberArray
      : Array.isArray(b) ? genericArray
      : typeof b.valueOf !== "function" && typeof b.toString !== "function" || isNaN(b) ? object
      : interpolateNumber)(a, b);
}

var degrees = 180 / Math.PI;

var identity$3 = {
  translateX: 0,
  translateY: 0,
  rotate: 0,
  skewX: 0,
  scaleX: 1,
  scaleY: 1
};

function decompose(a, b, c, d, e, f) {
  var scaleX, scaleY, skewX;
  if (scaleX = Math.sqrt(a * a + b * b)) a /= scaleX, b /= scaleX;
  if (skewX = a * c + b * d) c -= a * skewX, d -= b * skewX;
  if (scaleY = Math.sqrt(c * c + d * d)) c /= scaleY, d /= scaleY, skewX /= scaleY;
  if (a * d < b * c) a = -a, b = -b, skewX = -skewX, scaleX = -scaleX;
  return {
    translateX: e,
    translateY: f,
    rotate: Math.atan2(b, a) * degrees,
    skewX: Math.atan(skewX) * degrees,
    scaleX: scaleX,
    scaleY: scaleY
  };
}

var svgNode;

/* eslint-disable no-undef */
function parseCss(value) {
  const m = new (typeof DOMMatrix === "function" ? DOMMatrix : WebKitCSSMatrix)(value + "");
  return m.isIdentity ? identity$3 : decompose(m.a, m.b, m.c, m.d, m.e, m.f);
}

function parseSvg(value) {
  if (value == null) return identity$3;
  if (!svgNode) svgNode = document.createElementNS("http://www.w3.org/2000/svg", "g");
  svgNode.setAttribute("transform", value);
  if (!(value = svgNode.transform.baseVal.consolidate())) return identity$3;
  value = value.matrix;
  return decompose(value.a, value.b, value.c, value.d, value.e, value.f);
}

function interpolateTransform(parse, pxComma, pxParen, degParen) {

  function pop(s) {
    return s.length ? s.pop() + " " : "";
  }

  function translate(xa, ya, xb, yb, s, q) {
    if (xa !== xb || ya !== yb) {
      var i = s.push("translate(", null, pxComma, null, pxParen);
      q.push({i: i - 4, x: interpolateNumber(xa, xb)}, {i: i - 2, x: interpolateNumber(ya, yb)});
    } else if (xb || yb) {
      s.push("translate(" + xb + pxComma + yb + pxParen);
    }
  }

  function rotate(a, b, s, q) {
    if (a !== b) {
      if (a - b > 180) b += 360; else if (b - a > 180) a += 360; // shortest path
      q.push({i: s.push(pop(s) + "rotate(", null, degParen) - 2, x: interpolateNumber(a, b)});
    } else if (b) {
      s.push(pop(s) + "rotate(" + b + degParen);
    }
  }

  function skewX(a, b, s, q) {
    if (a !== b) {
      q.push({i: s.push(pop(s) + "skewX(", null, degParen) - 2, x: interpolateNumber(a, b)});
    } else if (b) {
      s.push(pop(s) + "skewX(" + b + degParen);
    }
  }

  function scale(xa, ya, xb, yb, s, q) {
    if (xa !== xb || ya !== yb) {
      var i = s.push(pop(s) + "scale(", null, ",", null, ")");
      q.push({i: i - 4, x: interpolateNumber(xa, xb)}, {i: i - 2, x: interpolateNumber(ya, yb)});
    } else if (xb !== 1 || yb !== 1) {
      s.push(pop(s) + "scale(" + xb + "," + yb + ")");
    }
  }

  return function(a, b) {
    var s = [], // string constants and placeholders
        q = []; // number interpolators
    a = parse(a), b = parse(b);
    translate(a.translateX, a.translateY, b.translateX, b.translateY, s, q);
    rotate(a.rotate, b.rotate, s, q);
    skewX(a.skewX, b.skewX, s, q);
    scale(a.scaleX, a.scaleY, b.scaleX, b.scaleY, s, q);
    a = b = null; // gc
    return function(t) {
      var i = -1, n = q.length, o;
      while (++i < n) s[(o = q[i]).i] = o.x(t);
      return s.join("");
    };
  };
}

var interpolateTransformCss = interpolateTransform(parseCss, "px, ", "px)", "deg)");
var interpolateTransformSvg = interpolateTransform(parseSvg, ", ", ")", ")");

var epsilon2 = 1e-12;

function cosh(x) {
  return ((x = Math.exp(x)) + 1 / x) / 2;
}

function sinh(x) {
  return ((x = Math.exp(x)) - 1 / x) / 2;
}

function tanh(x) {
  return ((x = Math.exp(2 * x)) - 1) / (x + 1);
}

const interpolateZoom = (function zoomRho(rho, rho2, rho4) {

  // p0 = [ux0, uy0, w0]
  // p1 = [ux1, uy1, w1]
  function zoom(p0, p1) {
    var ux0 = p0[0], uy0 = p0[1], w0 = p0[2],
        ux1 = p1[0], uy1 = p1[1], w1 = p1[2],
        dx = ux1 - ux0,
        dy = uy1 - uy0,
        d2 = dx * dx + dy * dy,
        i,
        S;

    // Special case for u0 ≅ u1.
    if (d2 < epsilon2) {
      S = Math.log(w1 / w0) / rho;
      i = function(t) {
        return [
          ux0 + t * dx,
          uy0 + t * dy,
          w0 * Math.exp(rho * t * S)
        ];
      };
    }

    // General case.
    else {
      var d1 = Math.sqrt(d2),
          b0 = (w1 * w1 - w0 * w0 + rho4 * d2) / (2 * w0 * rho2 * d1),
          b1 = (w1 * w1 - w0 * w0 - rho4 * d2) / (2 * w1 * rho2 * d1),
          r0 = Math.log(Math.sqrt(b0 * b0 + 1) - b0),
          r1 = Math.log(Math.sqrt(b1 * b1 + 1) - b1);
      S = (r1 - r0) / rho;
      i = function(t) {
        var s = t * S,
            coshr0 = cosh(r0),
            u = w0 / (rho2 * d1) * (coshr0 * tanh(rho * s + r0) - sinh(r0));
        return [
          ux0 + u * dx,
          uy0 + u * dy,
          w0 * coshr0 / cosh(rho * s + r0)
        ];
      };
    }

    i.duration = S * 1000 * rho / Math.SQRT2;

    return i;
  }

  zoom.rho = function(_) {
    var _1 = Math.max(1e-3, +_), _2 = _1 * _1, _4 = _2 * _2;
    return zoomRho(_1, _2, _4);
  };

  return zoom;
})(Math.SQRT2, 2, 4);

var frame = 0, // is an animation frame pending?
    timeout$1 = 0, // is a timeout pending?
    interval = 0, // are any timers active?
    pokeDelay = 1000, // how frequently we check for clock skew
    taskHead,
    taskTail,
    clockLast = 0,
    clockNow = 0,
    clockSkew = 0,
    clock = typeof performance === "object" && performance.now ? performance : Date,
    setFrame = typeof window === "object" && window.requestAnimationFrame ? window.requestAnimationFrame.bind(window) : function(f) { setTimeout(f, 17); };

function now() {
  return clockNow || (setFrame(clearNow), clockNow = clock.now() + clockSkew);
}

function clearNow() {
  clockNow = 0;
}

function Timer() {
  this._call =
  this._time =
  this._next = null;
}

Timer.prototype = timer.prototype = {
  constructor: Timer,
  restart: function(callback, delay, time) {
    if (typeof callback !== "function") throw new TypeError("callback is not a function");
    time = (time == null ? now() : +time) + (delay == null ? 0 : +delay);
    if (!this._next && taskTail !== this) {
      if (taskTail) taskTail._next = this;
      else taskHead = this;
      taskTail = this;
    }
    this._call = callback;
    this._time = time;
    sleep();
  },
  stop: function() {
    if (this._call) {
      this._call = null;
      this._time = Infinity;
      sleep();
    }
  }
};

function timer(callback, delay, time) {
  var t = new Timer;
  t.restart(callback, delay, time);
  return t;
}

function timerFlush() {
  now(); // Get the current time, if not already set.
  ++frame; // Pretend we’ve set an alarm, if we haven’t already.
  var t = taskHead, e;
  while (t) {
    if ((e = clockNow - t._time) >= 0) t._call.call(undefined, e);
    t = t._next;
  }
  --frame;
}

function wake() {
  clockNow = (clockLast = clock.now()) + clockSkew;
  frame = timeout$1 = 0;
  try {
    timerFlush();
  } finally {
    frame = 0;
    nap();
    clockNow = 0;
  }
}

function poke() {
  var now = clock.now(), delay = now - clockLast;
  if (delay > pokeDelay) clockSkew -= delay, clockLast = now;
}

function nap() {
  var t0, t1 = taskHead, t2, time = Infinity;
  while (t1) {
    if (t1._call) {
      if (time > t1._time) time = t1._time;
      t0 = t1, t1 = t1._next;
    } else {
      t2 = t1._next, t1._next = null;
      t1 = t0 ? t0._next = t2 : taskHead = t2;
    }
  }
  taskTail = t0;
  sleep(time);
}

function sleep(time) {
  if (frame) return; // Soonest alarm already set, or will be.
  if (timeout$1) timeout$1 = clearTimeout(timeout$1);
  var delay = time - clockNow; // Strictly less than if we recomputed clockNow.
  if (delay > 24) {
    if (time < Infinity) timeout$1 = setTimeout(wake, time - clock.now() - clockSkew);
    if (interval) interval = clearInterval(interval);
  } else {
    if (!interval) clockLast = clock.now(), interval = setInterval(poke, pokeDelay);
    frame = 1, setFrame(wake);
  }
}

function timeout(callback, delay, time) {
  var t = new Timer;
  delay = delay == null ? 0 : +delay;
  t.restart(elapsed => {
    t.stop();
    callback(elapsed + delay);
  }, delay, time);
  return t;
}

var emptyOn = dispatch("start", "end", "cancel", "interrupt");
var emptyTween = [];

var CREATED = 0;
var SCHEDULED = 1;
var STARTING = 2;
var STARTED = 3;
var RUNNING = 4;
var ENDING = 5;
var ENDED = 6;

function schedule(node, name, id, index, group, timing) {
  var schedules = node.__transition;
  if (!schedules) node.__transition = {};
  else if (id in schedules) return;
  create$1(node, id, {
    name: name,
    index: index, // For context during callback.
    group: group, // For context during callback.
    on: emptyOn,
    tween: emptyTween,
    time: timing.time,
    delay: timing.delay,
    duration: timing.duration,
    ease: timing.ease,
    timer: null,
    state: CREATED
  });
}

function init(node, id) {
  var schedule = get(node, id);
  if (schedule.state > CREATED) throw new Error("too late; already scheduled");
  return schedule;
}

function set(node, id) {
  var schedule = get(node, id);
  if (schedule.state > STARTED) throw new Error("too late; already running");
  return schedule;
}

function get(node, id) {
  var schedule = node.__transition;
  if (!schedule || !(schedule = schedule[id])) throw new Error("transition not found");
  return schedule;
}

function create$1(node, id, self) {
  var schedules = node.__transition,
      tween;

  // Initialize the self timer when the transition is created.
  // Note the actual delay is not known until the first callback!
  schedules[id] = self;
  self.timer = timer(schedule, 0, self.time);

  function schedule(elapsed) {
    self.state = SCHEDULED;
    self.timer.restart(start, self.delay, self.time);

    // If the elapsed delay is less than our first sleep, start immediately.
    if (self.delay <= elapsed) start(elapsed - self.delay);
  }

  function start(elapsed) {
    var i, j, n, o;

    // If the state is not SCHEDULED, then we previously errored on start.
    if (self.state !== SCHEDULED) return stop();

    for (i in schedules) {
      o = schedules[i];
      if (o.name !== self.name) continue;

      // While this element already has a starting transition during this frame,
      // defer starting an interrupting transition until that transition has a
      // chance to tick (and possibly end); see d3/d3-transition#54!
      if (o.state === STARTED) return timeout(start);

      // Interrupt the active transition, if any.
      if (o.state === RUNNING) {
        o.state = ENDED;
        o.timer.stop();
        o.on.call("interrupt", node, node.__data__, o.index, o.group);
        delete schedules[i];
      }

      // Cancel any pre-empted transitions.
      else if (+i < id) {
        o.state = ENDED;
        o.timer.stop();
        o.on.call("cancel", node, node.__data__, o.index, o.group);
        delete schedules[i];
      }
    }

    // Defer the first tick to end of the current frame; see d3/d3#1576.
    // Note the transition may be canceled after start and before the first tick!
    // Note this must be scheduled before the start event; see d3/d3-transition#16!
    // Assuming this is successful, subsequent callbacks go straight to tick.
    timeout(function() {
      if (self.state === STARTED) {
        self.state = RUNNING;
        self.timer.restart(tick, self.delay, self.time);
        tick(elapsed);
      }
    });

    // Dispatch the start event.
    // Note this must be done before the tween are initialized.
    self.state = STARTING;
    self.on.call("start", node, node.__data__, self.index, self.group);
    if (self.state !== STARTING) return; // interrupted
    self.state = STARTED;

    // Initialize the tween, deleting null tween.
    tween = new Array(n = self.tween.length);
    for (i = 0, j = -1; i < n; ++i) {
      if (o = self.tween[i].value.call(node, node.__data__, self.index, self.group)) {
        tween[++j] = o;
      }
    }
    tween.length = j + 1;
  }

  function tick(elapsed) {
    var t = elapsed < self.duration ? self.ease.call(null, elapsed / self.duration) : (self.timer.restart(stop), self.state = ENDING, 1),
        i = -1,
        n = tween.length;

    while (++i < n) {
      tween[i].call(node, t);
    }

    // Dispatch the end event.
    if (self.state === ENDING) {
      self.on.call("end", node, node.__data__, self.index, self.group);
      stop();
    }
  }

  function stop() {
    self.state = ENDED;
    self.timer.stop();
    delete schedules[id];
    for (var i in schedules) return; // eslint-disable-line no-unused-vars
    delete node.__transition;
  }
}

function interrupt(node, name) {
  var schedules = node.__transition,
      schedule,
      active,
      empty = true,
      i;

  if (!schedules) return;

  name = name == null ? null : name + "";

  for (i in schedules) {
    if ((schedule = schedules[i]).name !== name) { empty = false; continue; }
    active = schedule.state > STARTING && schedule.state < ENDING;
    schedule.state = ENDED;
    schedule.timer.stop();
    schedule.on.call(active ? "interrupt" : "cancel", node, node.__data__, schedule.index, schedule.group);
    delete schedules[i];
  }

  if (empty) delete node.__transition;
}

function selection_interrupt(name) {
  return this.each(function() {
    interrupt(this, name);
  });
}

function tweenRemove(id, name) {
  var tween0, tween1;
  return function() {
    var schedule = set(this, id),
        tween = schedule.tween;

    // If this node shared tween with the previous node,
    // just assign the updated shared tween and we’re done!
    // Otherwise, copy-on-write.
    if (tween !== tween0) {
      tween1 = tween0 = tween;
      for (var i = 0, n = tween1.length; i < n; ++i) {
        if (tween1[i].name === name) {
          tween1 = tween1.slice();
          tween1.splice(i, 1);
          break;
        }
      }
    }

    schedule.tween = tween1;
  };
}

function tweenFunction(id, name, value) {
  var tween0, tween1;
  if (typeof value !== "function") throw new Error;
  return function() {
    var schedule = set(this, id),
        tween = schedule.tween;

    // If this node shared tween with the previous node,
    // just assign the updated shared tween and we’re done!
    // Otherwise, copy-on-write.
    if (tween !== tween0) {
      tween1 = (tween0 = tween).slice();
      for (var t = {name: name, value: value}, i = 0, n = tween1.length; i < n; ++i) {
        if (tween1[i].name === name) {
          tween1[i] = t;
          break;
        }
      }
      if (i === n) tween1.push(t);
    }

    schedule.tween = tween1;
  };
}

function transition_tween(name, value) {
  var id = this._id;

  name += "";

  if (arguments.length < 2) {
    var tween = get(this.node(), id).tween;
    for (var i = 0, n = tween.length, t; i < n; ++i) {
      if ((t = tween[i]).name === name) {
        return t.value;
      }
    }
    return null;
  }

  return this.each((value == null ? tweenRemove : tweenFunction)(id, name, value));
}

function tweenValue(transition, name, value) {
  var id = transition._id;

  transition.each(function() {
    var schedule = set(this, id);
    (schedule.value || (schedule.value = {}))[name] = value.apply(this, arguments);
  });

  return function(node) {
    return get(node, id).value[name];
  };
}

function interpolate(a, b) {
  var c;
  return (typeof b === "number" ? interpolateNumber
      : b instanceof color ? interpolateRgb
      : (c = color(b)) ? (b = c, interpolateRgb)
      : interpolateString)(a, b);
}

function attrRemove(name) {
  return function() {
    this.removeAttribute(name);
  };
}

function attrRemoveNS(fullname) {
  return function() {
    this.removeAttributeNS(fullname.space, fullname.local);
  };
}

function attrConstant(name, interpolate, value1) {
  var string00,
      string1 = value1 + "",
      interpolate0;
  return function() {
    var string0 = this.getAttribute(name);
    return string0 === string1 ? null
        : string0 === string00 ? interpolate0
        : interpolate0 = interpolate(string00 = string0, value1);
  };
}

function attrConstantNS(fullname, interpolate, value1) {
  var string00,
      string1 = value1 + "",
      interpolate0;
  return function() {
    var string0 = this.getAttributeNS(fullname.space, fullname.local);
    return string0 === string1 ? null
        : string0 === string00 ? interpolate0
        : interpolate0 = interpolate(string00 = string0, value1);
  };
}

function attrFunction(name, interpolate, value) {
  var string00,
      string10,
      interpolate0;
  return function() {
    var string0, value1 = value(this), string1;
    if (value1 == null) return void this.removeAttribute(name);
    string0 = this.getAttribute(name);
    string1 = value1 + "";
    return string0 === string1 ? null
        : string0 === string00 && string1 === string10 ? interpolate0
        : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}

function attrFunctionNS(fullname, interpolate, value) {
  var string00,
      string10,
      interpolate0;
  return function() {
    var string0, value1 = value(this), string1;
    if (value1 == null) return void this.removeAttributeNS(fullname.space, fullname.local);
    string0 = this.getAttributeNS(fullname.space, fullname.local);
    string1 = value1 + "";
    return string0 === string1 ? null
        : string0 === string00 && string1 === string10 ? interpolate0
        : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}

function transition_attr(name, value) {
  var fullname = namespace(name), i = fullname === "transform" ? interpolateTransformSvg : interpolate;
  return this.attrTween(name, typeof value === "function"
      ? (fullname.local ? attrFunctionNS : attrFunction)(fullname, i, tweenValue(this, "attr." + name, value))
      : value == null ? (fullname.local ? attrRemoveNS : attrRemove)(fullname)
      : (fullname.local ? attrConstantNS : attrConstant)(fullname, i, value));
}

function attrInterpolate(name, i) {
  return function(t) {
    this.setAttribute(name, i.call(this, t));
  };
}

function attrInterpolateNS(fullname, i) {
  return function(t) {
    this.setAttributeNS(fullname.space, fullname.local, i.call(this, t));
  };
}

function attrTweenNS(fullname, value) {
  var t0, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0) t0 = (i0 = i) && attrInterpolateNS(fullname, i);
    return t0;
  }
  tween._value = value;
  return tween;
}

function attrTween(name, value) {
  var t0, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0) t0 = (i0 = i) && attrInterpolate(name, i);
    return t0;
  }
  tween._value = value;
  return tween;
}

function transition_attrTween(name, value) {
  var key = "attr." + name;
  if (arguments.length < 2) return (key = this.tween(key)) && key._value;
  if (value == null) return this.tween(key, null);
  if (typeof value !== "function") throw new Error;
  var fullname = namespace(name);
  return this.tween(key, (fullname.local ? attrTweenNS : attrTween)(fullname, value));
}

function delayFunction(id, value) {
  return function() {
    init(this, id).delay = +value.apply(this, arguments);
  };
}

function delayConstant(id, value) {
  return value = +value, function() {
    init(this, id).delay = value;
  };
}

function transition_delay(value) {
  var id = this._id;

  return arguments.length
      ? this.each((typeof value === "function"
          ? delayFunction
          : delayConstant)(id, value))
      : get(this.node(), id).delay;
}

function durationFunction(id, value) {
  return function() {
    set(this, id).duration = +value.apply(this, arguments);
  };
}

function durationConstant(id, value) {
  return value = +value, function() {
    set(this, id).duration = value;
  };
}

function transition_duration(value) {
  var id = this._id;

  return arguments.length
      ? this.each((typeof value === "function"
          ? durationFunction
          : durationConstant)(id, value))
      : get(this.node(), id).duration;
}

function easeConstant(id, value) {
  if (typeof value !== "function") throw new Error;
  return function() {
    set(this, id).ease = value;
  };
}

function transition_ease(value) {
  var id = this._id;

  return arguments.length
      ? this.each(easeConstant(id, value))
      : get(this.node(), id).ease;
}

function easeVarying(id, value) {
  return function() {
    var v = value.apply(this, arguments);
    if (typeof v !== "function") throw new Error;
    set(this, id).ease = v;
  };
}

function transition_easeVarying(value) {
  if (typeof value !== "function") throw new Error;
  return this.each(easeVarying(this._id, value));
}

function transition_filter(match) {
  if (typeof match !== "function") match = matcher(match);

  for (var groups = this._groups, m = groups.length, subgroups = new Array(m), j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, subgroup = subgroups[j] = [], node, i = 0; i < n; ++i) {
      if ((node = group[i]) && match.call(node, node.__data__, i, group)) {
        subgroup.push(node);
      }
    }
  }

  return new Transition(subgroups, this._parents, this._name, this._id);
}

function transition_merge(transition) {
  if (transition._id !== this._id) throw new Error;

  for (var groups0 = this._groups, groups1 = transition._groups, m0 = groups0.length, m1 = groups1.length, m = Math.min(m0, m1), merges = new Array(m0), j = 0; j < m; ++j) {
    for (var group0 = groups0[j], group1 = groups1[j], n = group0.length, merge = merges[j] = new Array(n), node, i = 0; i < n; ++i) {
      if (node = group0[i] || group1[i]) {
        merge[i] = node;
      }
    }
  }

  for (; j < m0; ++j) {
    merges[j] = groups0[j];
  }

  return new Transition(merges, this._parents, this._name, this._id);
}

function start(name) {
  return (name + "").trim().split(/^|\s+/).every(function(t) {
    var i = t.indexOf(".");
    if (i >= 0) t = t.slice(0, i);
    return !t || t === "start";
  });
}

function onFunction(id, name, listener) {
  var on0, on1, sit = start(name) ? init : set;
  return function() {
    var schedule = sit(this, id),
        on = schedule.on;

    // If this node shared a dispatch with the previous node,
    // just assign the updated shared dispatch and we’re done!
    // Otherwise, copy-on-write.
    if (on !== on0) (on1 = (on0 = on).copy()).on(name, listener);

    schedule.on = on1;
  };
}

function transition_on(name, listener) {
  var id = this._id;

  return arguments.length < 2
      ? get(this.node(), id).on.on(name)
      : this.each(onFunction(id, name, listener));
}

function removeFunction(id) {
  return function() {
    var parent = this.parentNode;
    for (var i in this.__transition) if (+i !== id) return;
    if (parent) parent.removeChild(this);
  };
}

function transition_remove() {
  return this.on("end.remove", removeFunction(this._id));
}

function transition_select(select) {
  var name = this._name,
      id = this._id;

  if (typeof select !== "function") select = selector(select);

  for (var groups = this._groups, m = groups.length, subgroups = new Array(m), j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, subgroup = subgroups[j] = new Array(n), node, subnode, i = 0; i < n; ++i) {
      if ((node = group[i]) && (subnode = select.call(node, node.__data__, i, group))) {
        if ("__data__" in node) subnode.__data__ = node.__data__;
        subgroup[i] = subnode;
        schedule(subgroup[i], name, id, i, subgroup, get(node, id));
      }
    }
  }

  return new Transition(subgroups, this._parents, name, id);
}

function transition_selectAll(select) {
  var name = this._name,
      id = this._id;

  if (typeof select !== "function") select = selectorAll(select);

  for (var groups = this._groups, m = groups.length, subgroups = [], parents = [], j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, node, i = 0; i < n; ++i) {
      if (node = group[i]) {
        for (var children = select.call(node, node.__data__, i, group), child, inherit = get(node, id), k = 0, l = children.length; k < l; ++k) {
          if (child = children[k]) {
            schedule(child, name, id, k, children, inherit);
          }
        }
        subgroups.push(children);
        parents.push(node);
      }
    }
  }

  return new Transition(subgroups, parents, name, id);
}

var Selection = selection.prototype.constructor;

function transition_selection() {
  return new Selection(this._groups, this._parents);
}

function styleNull(name, interpolate) {
  var string00,
      string10,
      interpolate0;
  return function() {
    var string0 = styleValue(this, name),
        string1 = (this.style.removeProperty(name), styleValue(this, name));
    return string0 === string1 ? null
        : string0 === string00 && string1 === string10 ? interpolate0
        : interpolate0 = interpolate(string00 = string0, string10 = string1);
  };
}

function styleRemove(name) {
  return function() {
    this.style.removeProperty(name);
  };
}

function styleConstant(name, interpolate, value1) {
  var string00,
      string1 = value1 + "",
      interpolate0;
  return function() {
    var string0 = styleValue(this, name);
    return string0 === string1 ? null
        : string0 === string00 ? interpolate0
        : interpolate0 = interpolate(string00 = string0, value1);
  };
}

function styleFunction(name, interpolate, value) {
  var string00,
      string10,
      interpolate0;
  return function() {
    var string0 = styleValue(this, name),
        value1 = value(this),
        string1 = value1 + "";
    if (value1 == null) string1 = value1 = (this.style.removeProperty(name), styleValue(this, name));
    return string0 === string1 ? null
        : string0 === string00 && string1 === string10 ? interpolate0
        : (string10 = string1, interpolate0 = interpolate(string00 = string0, value1));
  };
}

function styleMaybeRemove(id, name) {
  var on0, on1, listener0, key = "style." + name, event = "end." + key, remove;
  return function() {
    var schedule = set(this, id),
        on = schedule.on,
        listener = schedule.value[key] == null ? remove || (remove = styleRemove(name)) : undefined;

    // If this node shared a dispatch with the previous node,
    // just assign the updated shared dispatch and we’re done!
    // Otherwise, copy-on-write.
    if (on !== on0 || listener0 !== listener) (on1 = (on0 = on).copy()).on(event, listener0 = listener);

    schedule.on = on1;
  };
}

function transition_style(name, value, priority) {
  var i = (name += "") === "transform" ? interpolateTransformCss : interpolate;
  return value == null ? this
      .styleTween(name, styleNull(name, i))
      .on("end.style." + name, styleRemove(name))
    : typeof value === "function" ? this
      .styleTween(name, styleFunction(name, i, tweenValue(this, "style." + name, value)))
      .each(styleMaybeRemove(this._id, name))
    : this
      .styleTween(name, styleConstant(name, i, value), priority)
      .on("end.style." + name, null);
}

function styleInterpolate(name, i, priority) {
  return function(t) {
    this.style.setProperty(name, i.call(this, t), priority);
  };
}

function styleTween(name, value, priority) {
  var t, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0) t = (i0 = i) && styleInterpolate(name, i, priority);
    return t;
  }
  tween._value = value;
  return tween;
}

function transition_styleTween(name, value, priority) {
  var key = "style." + (name += "");
  if (arguments.length < 2) return (key = this.tween(key)) && key._value;
  if (value == null) return this.tween(key, null);
  if (typeof value !== "function") throw new Error;
  return this.tween(key, styleTween(name, value, priority == null ? "" : priority));
}

function textConstant(value) {
  return function() {
    this.textContent = value;
  };
}

function textFunction(value) {
  return function() {
    var value1 = value(this);
    this.textContent = value1 == null ? "" : value1;
  };
}

function transition_text(value) {
  return this.tween("text", typeof value === "function"
      ? textFunction(tweenValue(this, "text", value))
      : textConstant(value == null ? "" : value + ""));
}

function textInterpolate(i) {
  return function(t) {
    this.textContent = i.call(this, t);
  };
}

function textTween(value) {
  var t0, i0;
  function tween() {
    var i = value.apply(this, arguments);
    if (i !== i0) t0 = (i0 = i) && textInterpolate(i);
    return t0;
  }
  tween._value = value;
  return tween;
}

function transition_textTween(value) {
  var key = "text";
  if (arguments.length < 1) return (key = this.tween(key)) && key._value;
  if (value == null) return this.tween(key, null);
  if (typeof value !== "function") throw new Error;
  return this.tween(key, textTween(value));
}

function transition_transition() {
  var name = this._name,
      id0 = this._id,
      id1 = newId();

  for (var groups = this._groups, m = groups.length, j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, node, i = 0; i < n; ++i) {
      if (node = group[i]) {
        var inherit = get(node, id0);
        schedule(node, name, id1, i, group, {
          time: inherit.time + inherit.delay + inherit.duration,
          delay: 0,
          duration: inherit.duration,
          ease: inherit.ease
        });
      }
    }
  }

  return new Transition(groups, this._parents, name, id1);
}

function transition_end() {
  var on0, on1, that = this, id = that._id, size = that.size();
  return new Promise(function(resolve, reject) {
    var cancel = {value: reject},
        end = {value: function() { if (--size === 0) resolve(); }};

    that.each(function() {
      var schedule = set(this, id),
          on = schedule.on;

      // If this node shared a dispatch with the previous node,
      // just assign the updated shared dispatch and we’re done!
      // Otherwise, copy-on-write.
      if (on !== on0) {
        on1 = (on0 = on).copy();
        on1._.cancel.push(cancel);
        on1._.interrupt.push(cancel);
        on1._.end.push(end);
      }

      schedule.on = on1;
    });

    // The selection was empty, resolve end immediately
    if (size === 0) resolve();
  });
}

var id = 0;

function Transition(groups, parents, name, id) {
  this._groups = groups;
  this._parents = parents;
  this._name = name;
  this._id = id;
}

function newId() {
  return ++id;
}

var selection_prototype = selection.prototype;

Transition.prototype = {
  constructor: Transition,
  select: transition_select,
  selectAll: transition_selectAll,
  selectChild: selection_prototype.selectChild,
  selectChildren: selection_prototype.selectChildren,
  filter: transition_filter,
  merge: transition_merge,
  selection: transition_selection,
  transition: transition_transition,
  call: selection_prototype.call,
  nodes: selection_prototype.nodes,
  node: selection_prototype.node,
  size: selection_prototype.size,
  empty: selection_prototype.empty,
  each: selection_prototype.each,
  on: transition_on,
  attr: transition_attr,
  attrTween: transition_attrTween,
  style: transition_style,
  styleTween: transition_styleTween,
  text: transition_text,
  textTween: transition_textTween,
  remove: transition_remove,
  tween: transition_tween,
  delay: transition_delay,
  duration: transition_duration,
  ease: transition_ease,
  easeVarying: transition_easeVarying,
  end: transition_end,
  [Symbol.iterator]: selection_prototype[Symbol.iterator]
};

function cubicInOut(t) {
  return ((t *= 2) <= 1 ? t * t * t : (t -= 2) * t * t + 2) / 2;
}

var defaultTiming = {
  time: null, // Set on use.
  delay: 0,
  duration: 250,
  ease: cubicInOut
};

function inherit(node, id) {
  var timing;
  while (!(timing = node.__transition) || !(timing = timing[id])) {
    if (!(node = node.parentNode)) {
      throw new Error(`transition ${id} not found`);
    }
  }
  return timing;
}

function selection_transition(name) {
  var id,
      timing;

  if (name instanceof Transition) {
    id = name._id, name = name._name;
  } else {
    id = newId(), (timing = defaultTiming).time = now(), name = name == null ? null : name + "";
  }

  for (var groups = this._groups, m = groups.length, j = 0; j < m; ++j) {
    for (var group = groups[j], n = group.length, node, i = 0; i < n; ++i) {
      if (node = group[i]) {
        schedule(node, name, id, i, group, timing || inherit(node, id));
      }
    }
  }

  return new Transition(groups, this._parents, name, id);
}

selection.prototype.interrupt = selection_interrupt;
selection.prototype.transition = selection_transition;

const constant = x => () => x;

function ZoomEvent(type, {
  sourceEvent,
  target,
  transform,
  dispatch
}) {
  Object.defineProperties(this, {
    type: {value: type, enumerable: true, configurable: true},
    sourceEvent: {value: sourceEvent, enumerable: true, configurable: true},
    target: {value: target, enumerable: true, configurable: true},
    transform: {value: transform, enumerable: true, configurable: true},
    _: {value: dispatch}
  });
}

function Transform(k, x, y) {
  this.k = k;
  this.x = x;
  this.y = y;
}

Transform.prototype = {
  constructor: Transform,
  scale: function(k) {
    return k === 1 ? this : new Transform(this.k * k, this.x, this.y);
  },
  translate: function(x, y) {
    return x === 0 & y === 0 ? this : new Transform(this.k, this.x + this.k * x, this.y + this.k * y);
  },
  apply: function(point) {
    return [point[0] * this.k + this.x, point[1] * this.k + this.y];
  },
  applyX: function(x) {
    return x * this.k + this.x;
  },
  applyY: function(y) {
    return y * this.k + this.y;
  },
  invert: function(location) {
    return [(location[0] - this.x) / this.k, (location[1] - this.y) / this.k];
  },
  invertX: function(x) {
    return (x - this.x) / this.k;
  },
  invertY: function(y) {
    return (y - this.y) / this.k;
  },
  rescaleX: function(x) {
    return x.copy().domain(x.range().map(this.invertX, this).map(x.invert, x));
  },
  rescaleY: function(y) {
    return y.copy().domain(y.range().map(this.invertY, this).map(y.invert, y));
  },
  toString: function() {
    return "translate(" + this.x + "," + this.y + ") scale(" + this.k + ")";
  }
};

var identity$2 = new Transform(1, 0, 0);

transform.prototype = Transform.prototype;

function transform(node) {
  while (!node.__zoom) if (!(node = node.parentNode)) return identity$2;
  return node.__zoom;
}

function nopropagation(event) {
  event.stopImmediatePropagation();
}

function noevent(event) {
  event.preventDefault();
  event.stopImmediatePropagation();
}

// Ignore right-click, since that should open the context menu.
// except for pinch-to-zoom, which is sent as a wheel+ctrlKey event
function defaultFilter(event) {
  return (!event.ctrlKey || event.type === 'wheel') && !event.button;
}

function defaultExtent() {
  var e = this;
  if (e instanceof SVGElement) {
    e = e.ownerSVGElement || e;
    if (e.hasAttribute("viewBox")) {
      e = e.viewBox.baseVal;
      return [[e.x, e.y], [e.x + e.width, e.y + e.height]];
    }
    return [[0, 0], [e.width.baseVal.value, e.height.baseVal.value]];
  }
  return [[0, 0], [e.clientWidth, e.clientHeight]];
}

function defaultTransform() {
  return this.__zoom || identity$2;
}

function defaultWheelDelta(event) {
  return -event.deltaY * (event.deltaMode === 1 ? 0.05 : event.deltaMode ? 1 : 0.002) * (event.ctrlKey ? 10 : 1);
}

function defaultTouchable() {
  return navigator.maxTouchPoints || ("ontouchstart" in this);
}

function defaultConstrain(transform, extent, translateExtent) {
  var dx0 = transform.invertX(extent[0][0]) - translateExtent[0][0],
      dx1 = transform.invertX(extent[1][0]) - translateExtent[1][0],
      dy0 = transform.invertY(extent[0][1]) - translateExtent[0][1],
      dy1 = transform.invertY(extent[1][1]) - translateExtent[1][1];
  return transform.translate(
    dx1 > dx0 ? (dx0 + dx1) / 2 : Math.min(0, dx0) || Math.max(0, dx1),
    dy1 > dy0 ? (dy0 + dy1) / 2 : Math.min(0, dy0) || Math.max(0, dy1)
  );
}

function zoom() {
  var filter = defaultFilter,
      extent = defaultExtent,
      constrain = defaultConstrain,
      wheelDelta = defaultWheelDelta,
      touchable = defaultTouchable,
      scaleExtent = [0, Infinity],
      translateExtent = [[-Infinity, -Infinity], [Infinity, Infinity]],
      duration = 250,
      interpolate = interpolateZoom,
      listeners = dispatch("start", "zoom", "end"),
      touchstarting,
      touchfirst,
      touchending,
      touchDelay = 500,
      wheelDelay = 150,
      clickDistance2 = 0,
      tapDistance = 10;

  function zoom(selection) {
    selection
        .property("__zoom", defaultTransform)
        .on("wheel.zoom", wheeled, {passive: false})
        .on("mousedown.zoom", mousedowned)
        .on("dblclick.zoom", dblclicked)
      .filter(touchable)
        .on("touchstart.zoom", touchstarted)
        .on("touchmove.zoom", touchmoved)
        .on("touchend.zoom touchcancel.zoom", touchended)
        .style("-webkit-tap-highlight-color", "rgba(0,0,0,0)");
  }

  zoom.transform = function(collection, transform, point, event) {
    var selection = collection.selection ? collection.selection() : collection;
    selection.property("__zoom", defaultTransform);
    if (collection !== selection) {
      schedule(collection, transform, point, event);
    } else {
      selection.interrupt().each(function() {
        gesture(this, arguments)
          .event(event)
          .start()
          .zoom(null, typeof transform === "function" ? transform.apply(this, arguments) : transform)
          .end();
      });
    }
  };

  zoom.scaleBy = function(selection, k, p, event) {
    zoom.scaleTo(selection, function() {
      var k0 = this.__zoom.k,
          k1 = typeof k === "function" ? k.apply(this, arguments) : k;
      return k0 * k1;
    }, p, event);
  };

  zoom.scaleTo = function(selection, k, p, event) {
    zoom.transform(selection, function() {
      var e = extent.apply(this, arguments),
          t0 = this.__zoom,
          p0 = p == null ? centroid(e) : typeof p === "function" ? p.apply(this, arguments) : p,
          p1 = t0.invert(p0),
          k1 = typeof k === "function" ? k.apply(this, arguments) : k;
      return constrain(translate(scale(t0, k1), p0, p1), e, translateExtent);
    }, p, event);
  };

  zoom.translateBy = function(selection, x, y, event) {
    zoom.transform(selection, function() {
      return constrain(this.__zoom.translate(
        typeof x === "function" ? x.apply(this, arguments) : x,
        typeof y === "function" ? y.apply(this, arguments) : y
      ), extent.apply(this, arguments), translateExtent);
    }, null, event);
  };

  zoom.translateTo = function(selection, x, y, p, event) {
    zoom.transform(selection, function() {
      var e = extent.apply(this, arguments),
          t = this.__zoom,
          p0 = p == null ? centroid(e) : typeof p === "function" ? p.apply(this, arguments) : p;
      return constrain(identity$2.translate(p0[0], p0[1]).scale(t.k).translate(
        typeof x === "function" ? -x.apply(this, arguments) : -x,
        typeof y === "function" ? -y.apply(this, arguments) : -y
      ), e, translateExtent);
    }, p, event);
  };

  function scale(transform, k) {
    k = Math.max(scaleExtent[0], Math.min(scaleExtent[1], k));
    return k === transform.k ? transform : new Transform(k, transform.x, transform.y);
  }

  function translate(transform, p0, p1) {
    var x = p0[0] - p1[0] * transform.k, y = p0[1] - p1[1] * transform.k;
    return x === transform.x && y === transform.y ? transform : new Transform(transform.k, x, y);
  }

  function centroid(extent) {
    return [(+extent[0][0] + +extent[1][0]) / 2, (+extent[0][1] + +extent[1][1]) / 2];
  }

  function schedule(transition, transform, point, event) {
    transition
        .on("start.zoom", function() { gesture(this, arguments).event(event).start(); })
        .on("interrupt.zoom end.zoom", function() { gesture(this, arguments).event(event).end(); })
        .tween("zoom", function() {
          var that = this,
              args = arguments,
              g = gesture(that, args).event(event),
              e = extent.apply(that, args),
              p = point == null ? centroid(e) : typeof point === "function" ? point.apply(that, args) : point,
              w = Math.max(e[1][0] - e[0][0], e[1][1] - e[0][1]),
              a = that.__zoom,
              b = typeof transform === "function" ? transform.apply(that, args) : transform,
              i = interpolate(a.invert(p).concat(w / a.k), b.invert(p).concat(w / b.k));
          return function(t) {
            if (t === 1) t = b; // Avoid rounding error on end.
            else { var l = i(t), k = w / l[2]; t = new Transform(k, p[0] - l[0] * k, p[1] - l[1] * k); }
            g.zoom(null, t);
          };
        });
  }

  function gesture(that, args, clean) {
    return (!clean && that.__zooming) || new Gesture(that, args);
  }

  function Gesture(that, args) {
    this.that = that;
    this.args = args;
    this.active = 0;
    this.sourceEvent = null;
    this.extent = extent.apply(that, args);
    this.taps = 0;
  }

  Gesture.prototype = {
    event: function(event) {
      if (event) this.sourceEvent = event;
      return this;
    },
    start: function() {
      if (++this.active === 1) {
        this.that.__zooming = this;
        this.emit("start");
      }
      return this;
    },
    zoom: function(key, transform) {
      if (this.mouse && key !== "mouse") this.mouse[1] = transform.invert(this.mouse[0]);
      if (this.touch0 && key !== "touch") this.touch0[1] = transform.invert(this.touch0[0]);
      if (this.touch1 && key !== "touch") this.touch1[1] = transform.invert(this.touch1[0]);
      this.that.__zoom = transform;
      this.emit("zoom");
      return this;
    },
    end: function() {
      if (--this.active === 0) {
        delete this.that.__zooming;
        this.emit("end");
      }
      return this;
    },
    emit: function(type) {
      var d = select(this.that).datum();
      listeners.call(
        type,
        this.that,
        new ZoomEvent(type, {
          sourceEvent: this.sourceEvent,
          target: zoom,
          transform: this.that.__zoom,
          dispatch: listeners
        }),
        d
      );
    }
  };

  function wheeled(event, ...args) {
    if (!filter.apply(this, arguments)) return;
    var g = gesture(this, args).event(event),
        t = this.__zoom,
        k = Math.max(scaleExtent[0], Math.min(scaleExtent[1], t.k * Math.pow(2, wheelDelta.apply(this, arguments)))),
        p = pointer(event);

    // If the mouse is in the same location as before, reuse it.
    // If there were recent wheel events, reset the wheel idle timeout.
    if (g.wheel) {
      if (g.mouse[0][0] !== p[0] || g.mouse[0][1] !== p[1]) {
        g.mouse[1] = t.invert(g.mouse[0] = p);
      }
      clearTimeout(g.wheel);
    }

    // If this wheel event won’t trigger a transform change, ignore it.
    else if (t.k === k) return;

    // Otherwise, capture the mouse point and location at the start.
    else {
      g.mouse = [p, t.invert(p)];
      interrupt(this);
      g.start();
    }

    noevent(event);
    g.wheel = setTimeout(wheelidled, wheelDelay);
    g.zoom("mouse", constrain(translate(scale(t, k), g.mouse[0], g.mouse[1]), g.extent, translateExtent));

    function wheelidled() {
      g.wheel = null;
      g.end();
    }
  }

  function mousedowned(event, ...args) {
    if (touchending || !filter.apply(this, arguments)) return;
    var currentTarget = event.currentTarget,
        g = gesture(this, args, true).event(event),
        v = select(event.view).on("mousemove.zoom", mousemoved, true).on("mouseup.zoom", mouseupped, true),
        p = pointer(event, currentTarget),
        x0 = event.clientX,
        y0 = event.clientY;

    dragDisable(event.view);
    nopropagation(event);
    g.mouse = [p, this.__zoom.invert(p)];
    interrupt(this);
    g.start();

    function mousemoved(event) {
      noevent(event);
      if (!g.moved) {
        var dx = event.clientX - x0, dy = event.clientY - y0;
        g.moved = dx * dx + dy * dy > clickDistance2;
      }
      g.event(event)
       .zoom("mouse", constrain(translate(g.that.__zoom, g.mouse[0] = pointer(event, currentTarget), g.mouse[1]), g.extent, translateExtent));
    }

    function mouseupped(event) {
      v.on("mousemove.zoom mouseup.zoom", null);
      yesdrag(event.view, g.moved);
      noevent(event);
      g.event(event).end();
    }
  }

  function dblclicked(event, ...args) {
    if (!filter.apply(this, arguments)) return;
    var t0 = this.__zoom,
        p0 = pointer(event.changedTouches ? event.changedTouches[0] : event, this),
        p1 = t0.invert(p0),
        k1 = t0.k * (event.shiftKey ? 0.5 : 2),
        t1 = constrain(translate(scale(t0, k1), p0, p1), extent.apply(this, args), translateExtent);

    noevent(event);
    if (duration > 0) select(this).transition().duration(duration).call(schedule, t1, p0, event);
    else select(this).call(zoom.transform, t1, p0, event);
  }

  function touchstarted(event, ...args) {
    if (!filter.apply(this, arguments)) return;
    var touches = event.touches,
        n = touches.length,
        g = gesture(this, args, event.changedTouches.length === n).event(event),
        started, i, t, p;

    nopropagation(event);
    for (i = 0; i < n; ++i) {
      t = touches[i], p = pointer(t, this);
      p = [p, this.__zoom.invert(p), t.identifier];
      if (!g.touch0) g.touch0 = p, started = true, g.taps = 1 + !!touchstarting;
      else if (!g.touch1 && g.touch0[2] !== p[2]) g.touch1 = p, g.taps = 0;
    }

    if (touchstarting) touchstarting = clearTimeout(touchstarting);

    if (started) {
      if (g.taps < 2) touchfirst = p[0], touchstarting = setTimeout(function() { touchstarting = null; }, touchDelay);
      interrupt(this);
      g.start();
    }
  }

  function touchmoved(event, ...args) {
    if (!this.__zooming) return;
    var g = gesture(this, args).event(event),
        touches = event.changedTouches,
        n = touches.length, i, t, p, l;

    noevent(event);
    for (i = 0; i < n; ++i) {
      t = touches[i], p = pointer(t, this);
      if (g.touch0 && g.touch0[2] === t.identifier) g.touch0[0] = p;
      else if (g.touch1 && g.touch1[2] === t.identifier) g.touch1[0] = p;
    }
    t = g.that.__zoom;
    if (g.touch1) {
      var p0 = g.touch0[0], l0 = g.touch0[1],
          p1 = g.touch1[0], l1 = g.touch1[1],
          dp = (dp = p1[0] - p0[0]) * dp + (dp = p1[1] - p0[1]) * dp,
          dl = (dl = l1[0] - l0[0]) * dl + (dl = l1[1] - l0[1]) * dl;
      t = scale(t, Math.sqrt(dp / dl));
      p = [(p0[0] + p1[0]) / 2, (p0[1] + p1[1]) / 2];
      l = [(l0[0] + l1[0]) / 2, (l0[1] + l1[1]) / 2];
    }
    else if (g.touch0) p = g.touch0[0], l = g.touch0[1];
    else return;

    g.zoom("touch", constrain(translate(t, p, l), g.extent, translateExtent));
  }

  function touchended(event, ...args) {
    if (!this.__zooming) return;
    var g = gesture(this, args).event(event),
        touches = event.changedTouches,
        n = touches.length, i, t;

    nopropagation(event);
    if (touchending) clearTimeout(touchending);
    touchending = setTimeout(function() { touchending = null; }, touchDelay);
    for (i = 0; i < n; ++i) {
      t = touches[i];
      if (g.touch0 && g.touch0[2] === t.identifier) delete g.touch0;
      else if (g.touch1 && g.touch1[2] === t.identifier) delete g.touch1;
    }
    if (g.touch1 && !g.touch0) g.touch0 = g.touch1, delete g.touch1;
    if (g.touch0) g.touch0[1] = this.__zoom.invert(g.touch0[0]);
    else {
      g.end();
      // If this was a dbltap, reroute to the (optional) dblclick.zoom handler.
      if (g.taps === 2) {
        t = pointer(t, this);
        if (Math.hypot(touchfirst[0] - t[0], touchfirst[1] - t[1]) < tapDistance) {
          var p = select(this).on("dblclick.zoom");
          if (p) p.apply(this, arguments);
        }
      }
    }
  }

  zoom.wheelDelta = function(_) {
    return arguments.length ? (wheelDelta = typeof _ === "function" ? _ : constant(+_), zoom) : wheelDelta;
  };

  zoom.filter = function(_) {
    return arguments.length ? (filter = typeof _ === "function" ? _ : constant(!!_), zoom) : filter;
  };

  zoom.touchable = function(_) {
    return arguments.length ? (touchable = typeof _ === "function" ? _ : constant(!!_), zoom) : touchable;
  };

  zoom.extent = function(_) {
    return arguments.length ? (extent = typeof _ === "function" ? _ : constant([[+_[0][0], +_[0][1]], [+_[1][0], +_[1][1]]]), zoom) : extent;
  };

  zoom.scaleExtent = function(_) {
    return arguments.length ? (scaleExtent[0] = +_[0], scaleExtent[1] = +_[1], zoom) : [scaleExtent[0], scaleExtent[1]];
  };

  zoom.translateExtent = function(_) {
    return arguments.length ? (translateExtent[0][0] = +_[0][0], translateExtent[1][0] = +_[1][0], translateExtent[0][1] = +_[0][1], translateExtent[1][1] = +_[1][1], zoom) : [[translateExtent[0][0], translateExtent[0][1]], [translateExtent[1][0], translateExtent[1][1]]];
  };

  zoom.constrain = function(_) {
    return arguments.length ? (constrain = _, zoom) : constrain;
  };

  zoom.duration = function(_) {
    return arguments.length ? (duration = +_, zoom) : duration;
  };

  zoom.interpolate = function(_) {
    return arguments.length ? (interpolate = _, zoom) : interpolate;
  };

  zoom.on = function() {
    var value = listeners.on.apply(listeners, arguments);
    return value === listeners ? zoom : value;
  };

  zoom.clickDistance = function(_) {
    return arguments.length ? (clickDistance2 = (_ = +_) * _, zoom) : Math.sqrt(clickDistance2);
  };

  zoom.tapDistance = function(_) {
    return arguments.length ? (tapDistance = +_, zoom) : tapDistance;
  };

  return zoom;
}

const errorMessages = {
    error001: () => '[React Flow]: Seems like you have not used zustand provider as an ancestor. Help: https://reactflow.dev/error#001',
    error002: () => "It looks like you've created a new nodeTypes or edgeTypes object. If this wasn't on purpose please define the nodeTypes/edgeTypes outside of the component or memoize them.",
    error003: (nodeType) => `Node type "${nodeType}" not found. Using fallback type "default".`,
    error004: () => 'The React Flow parent container needs a width and a height to render the graph.',
    error005: () => 'Only child nodes can use a parent extent.',
    error006: () => "Can't create edge. An edge needs a source and a target.",
    error007: (id) => `The old edge with id=${id} does not exist.`,
    error009: (type) => `Marker type "${type}" doesn't exist.`,
    error008: (handleType, { id, sourceHandle, targetHandle }) => `Couldn't create edge for ${handleType} handle id: "${handleType === 'source' ? sourceHandle : targetHandle}", edge id: ${id}.`,
    error010: () => 'Handle: No node id found. Make sure to only use a Handle inside a custom Node.',
    error011: (edgeType) => `Edge type "${edgeType}" not found. Using fallback type "default".`,
    error012: (id) => `Node with id "${id}" does not exist, it may have been removed. This can happen when a node is deleted before the "onNodeClick" handler is called.`,
    error013: (lib = 'react') => `It seems that you haven't loaded the styles. Please import '@xyflow/${lib}/dist/style.css' or base.css to make sure everything is working properly.`,
    error014: () => 'useNodeConnections: No node ID found. Call useNodeConnections inside a custom Node or provide a node ID.',
    error015: () => 'It seems that you are trying to drag a node that is not initialized. Please use onNodesChange as explained in the docs.',
};
const infiniteExtent = [
    [Number.NEGATIVE_INFINITY, Number.NEGATIVE_INFINITY],
    [Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY],
];
const elementSelectionKeys = ['Enter', ' ', 'Escape'];
const defaultAriaLabelConfig = {
    'node.a11yDescription.default': 'Press enter or space to select a node. Press delete to remove it and escape to cancel.',
    'node.a11yDescription.keyboardDisabled': 'Press enter or space to select a node. You can then use the arrow keys to move the node around. Press delete to remove it and escape to cancel.',
    'node.a11yDescription.ariaLiveMessage': ({ direction, x, y }) => `Moved selected node ${direction}. New position, x: ${x}, y: ${y}`,
    'edge.a11yDescription.default': 'Press enter or space to select an edge. You can then press delete to remove it or escape to cancel.',
    // Control elements
    'controls.ariaLabel': 'Control Panel',
    'controls.zoomIn.ariaLabel': 'Zoom In',
    'controls.zoomOut.ariaLabel': 'Zoom Out',
    'controls.fitView.ariaLabel': 'Fit View',
    'controls.interactive.ariaLabel': 'Toggle Interactivity',
    // Mini map
    'minimap.ariaLabel': 'Mini Map',
    // Handle
    'handle.ariaLabel': 'Handle',
};

/**
 * The `ConnectionMode` is used to set the mode of connection between nodes.
 * The `Strict` mode is the default one and only allows source to target edges.
 * `Loose` mode allows source to source and target to target edges as well.
 *
 * @public
 */
var ConnectionMode;
(function (ConnectionMode) {
    ConnectionMode["Strict"] = "strict";
    ConnectionMode["Loose"] = "loose";
})(ConnectionMode || (ConnectionMode = {}));
/**
 * This enum is used to set the different modes of panning the viewport when the
 * user scrolls. The `Free` mode allows the user to pan in any direction by scrolling
 * with a device like a trackpad. The `Vertical` and `Horizontal` modes restrict
 * scroll panning to only the vertical or horizontal axis, respectively.
 *
 * @public
 */
var PanOnScrollMode;
(function (PanOnScrollMode) {
    PanOnScrollMode["Free"] = "free";
    PanOnScrollMode["Vertical"] = "vertical";
    PanOnScrollMode["Horizontal"] = "horizontal";
})(PanOnScrollMode || (PanOnScrollMode = {}));
var SelectionMode;
(function (SelectionMode) {
    SelectionMode["Partial"] = "partial";
    SelectionMode["Full"] = "full";
})(SelectionMode || (SelectionMode = {}));
const initialConnection = {
    inProgress: false,
    isValid: null,
    from: null,
    fromHandle: null,
    fromPosition: null,
    fromNode: null,
    to: null,
    toHandle: null,
    toPosition: null,
    toNode: null,
    pointer: null,
};

/**
 * If you set the `connectionLineType` prop on your [`<ReactFlow />`](/api-reference/react-flow#connection-connectionLineType)
 *component, it will dictate the style of connection line rendered when creating
 *new edges.
 *
 * @public
 *
 * @remarks If you choose to render a custom connection line component, this value will be
 *passed to your component as part of its [`ConnectionLineComponentProps`](/api-reference/types/connection-line-component-props).
 */
var ConnectionLineType;
(function (ConnectionLineType) {
    ConnectionLineType["Bezier"] = "default";
    ConnectionLineType["Straight"] = "straight";
    ConnectionLineType["Step"] = "step";
    ConnectionLineType["SmoothStep"] = "smoothstep";
    ConnectionLineType["SimpleBezier"] = "simplebezier";
})(ConnectionLineType || (ConnectionLineType = {}));
/**
 * Edges may optionally have a marker on either end. The MarkerType type enumerates
 * the options available to you when configuring a given marker.
 *
 * @public
 */
var MarkerType;
(function (MarkerType) {
    MarkerType["Arrow"] = "arrow";
    MarkerType["ArrowClosed"] = "arrowclosed";
})(MarkerType || (MarkerType = {}));

/**
 * While [`PanelPosition`](/api-reference/types/panel-position) can be used to place a
 * component in the corners of a container, the `Position` enum is less precise and used
 * primarily in relation to edges and handles.
 *
 * @public
 */
var Position;
(function (Position) {
    Position["Left"] = "left";
    Position["Top"] = "top";
    Position["Right"] = "right";
    Position["Bottom"] = "bottom";
})(Position || (Position = {}));
const oppositePosition = {
    [Position.Left]: Position.Right,
    [Position.Right]: Position.Left,
    [Position.Top]: Position.Bottom,
    [Position.Bottom]: Position.Top,
};
function getConnectionStatus(isValid) {
    return isValid === null ? null : isValid ? 'valid' : 'invalid';
}

/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Test whether an object is usable as an Edge
 * @public
 * @remarks In TypeScript this is a type guard that will narrow the type of whatever you pass in to Edge if it returns true
 * @param element - The element to test
 * @returns A boolean indicating whether the element is an Edge
 */
const isEdgeBase = (element) => 'id' in element && 'source' in element && 'target' in element;
/**
 * Test whether an object is usable as a Node
 * @public
 * @remarks In TypeScript this is a type guard that will narrow the type of whatever you pass in to Node if it returns true
 * @param element - The element to test
 * @returns A boolean indicating whether the element is an Node
 */
const isNodeBase = (element) => 'id' in element && 'position' in element && !('source' in element) && !('target' in element);
const isInternalNodeBase = (element) => 'id' in element && 'internals' in element && !('source' in element) && !('target' in element);
const getNodePositionWithOrigin = (node, nodeOrigin = [0, 0]) => {
    const { width, height } = getNodeDimensions(node);
    const origin = node.origin ?? nodeOrigin;
    const offsetX = width * origin[0];
    const offsetY = height * origin[1];
    return {
        x: node.position.x - offsetX,
        y: node.position.y - offsetY,
    };
};
/**
 * Returns the bounding box that contains all the given nodes in an array. This can
 * be useful when combined with [`getViewportForBounds`](/api-reference/utils/get-viewport-for-bounds)
 * to calculate the correct transform to fit the given nodes in a viewport.
 * @public
 * @remarks Useful when combined with {@link getViewportForBounds} to calculate the correct transform to fit the given nodes in a viewport.
 * @param nodes - Nodes to calculate the bounds for.
 * @returns Bounding box enclosing all nodes.
 *
 * @remarks This function was previously called `getRectOfNodes`
 *
 * @example
 * ```js
 *import { getNodesBounds } from '@xyflow/react';
 *
 *const nodes = [
 *  {
 *    id: 'a',
 *    position: { x: 0, y: 0 },
 *    data: { label: 'a' },
 *    width: 50,
 *    height: 25,
 *  },
 *  {
 *    id: 'b',
 *    position: { x: 100, y: 100 },
 *    data: { label: 'b' },
 *    width: 50,
 *    height: 25,
 *  },
 *];
 *
 *const bounds = getNodesBounds(nodes);
 *```
 */
const getNodesBounds = (nodes, params = { nodeOrigin: [0, 0] }) => {
    if (process.env.NODE_ENV === 'development' && !params.nodeLookup) {
        console.warn('Please use `getNodesBounds` from `useReactFlow`/`useSvelteFlow` hook to ensure correct values for sub flows. If not possible, you have to provide a nodeLookup to support sub flows.');
    }
    if (nodes.length === 0) {
        return { x: 0, y: 0, width: 0, height: 0 };
    }
    const box = nodes.reduce((currBox, nodeOrId) => {
        const isId = typeof nodeOrId === 'string';
        let currentNode = !params.nodeLookup && !isId ? nodeOrId : undefined;
        if (params.nodeLookup) {
            currentNode = isId
                ? params.nodeLookup.get(nodeOrId)
                : !isInternalNodeBase(nodeOrId)
                    ? params.nodeLookup.get(nodeOrId.id)
                    : nodeOrId;
        }
        const nodeBox = currentNode ? nodeToBox(currentNode, params.nodeOrigin) : { x: 0, y: 0, x2: 0, y2: 0 };
        return getBoundsOfBoxes(currBox, nodeBox);
    }, { x: Infinity, y: Infinity, x2: -Infinity, y2: -Infinity });
    return boxToRect(box);
};
/**
 * Determines a bounding box that contains all given nodes in an array
 * @internal
 */
const getInternalNodesBounds = (nodeLookup, params = {}) => {
    let box = { x: Infinity, y: Infinity, x2: -Infinity, y2: -Infinity };
    let hasVisibleNodes = false;
    nodeLookup.forEach((node) => {
        if (params.filter === undefined || params.filter(node)) {
            box = getBoundsOfBoxes(box, nodeToBox(node));
            hasVisibleNodes = true;
        }
    });
    return hasVisibleNodes ? boxToRect(box) : { x: 0, y: 0, width: 0, height: 0 };
};
const getNodesInside = (nodes, rect, [tx, ty, tScale] = [0, 0, 1], partially = false, 
// set excludeNonSelectableNodes if you want to pay attention to the nodes "selectable" attribute
excludeNonSelectableNodes = false) => {
    const paneRect = {
        ...pointToRendererPoint(rect, [tx, ty, tScale]),
        width: rect.width / tScale,
        height: rect.height / tScale,
    };
    const visibleNodes = [];
    for (const node of nodes.values()) {
        const { measured, selectable = true, hidden = false } = node;
        if ((excludeNonSelectableNodes && !selectable) || hidden) {
            continue;
        }
        const width = measured.width ?? node.width ?? node.initialWidth ?? null;
        const height = measured.height ?? node.height ?? node.initialHeight ?? null;
        const overlappingArea = getOverlappingArea(paneRect, nodeToRect(node));
        const area = (width ?? 0) * (height ?? 0);
        const partiallyVisible = partially && overlappingArea > 0;
        const forceInitialRender = !node.internals.handleBounds;
        const isVisible = forceInitialRender || partiallyVisible || overlappingArea >= area;
        if (isVisible || node.dragging) {
            visibleNodes.push(node);
        }
    }
    return visibleNodes;
};
/**
 * This utility filters an array of edges, keeping only those where either the source or target
 * node is present in the given array of nodes.
 * @public
 * @param nodes - Nodes you want to get the connected edges for.
 * @param edges - All edges.
 * @returns Array of edges that connect any of the given nodes with each other.
 *
 * @example
 * ```js
 *import { getConnectedEdges } from '@xyflow/react';
 *
 *const nodes = [
 *  { id: 'a', position: { x: 0, y: 0 } },
 *  { id: 'b', position: { x: 100, y: 0 } },
 *];
 *
 *const edges = [
 *  { id: 'a->c', source: 'a', target: 'c' },
 *  { id: 'c->d', source: 'c', target: 'd' },
 *];
 *
 *const connectedEdges = getConnectedEdges(nodes, edges);
 * // => [{ id: 'a->c', source: 'a', target: 'c' }]
 *```
 */
const getConnectedEdges = (nodes, edges) => {
    const nodeIds = new Set();
    nodes.forEach((node) => {
        nodeIds.add(node.id);
    });
    return edges.filter((edge) => nodeIds.has(edge.source) || nodeIds.has(edge.target));
};
function getFitViewNodes(nodeLookup, options) {
    const fitViewNodes = new Map();
    const optionNodeIds = options?.nodes ? new Set(options.nodes.map((node) => node.id)) : null;
    nodeLookup.forEach((n) => {
        const isVisible = n.measured.width && n.measured.height && (options?.includeHiddenNodes || !n.hidden);
        if (isVisible && (!optionNodeIds || optionNodeIds.has(n.id))) {
            fitViewNodes.set(n.id, n);
        }
    });
    return fitViewNodes;
}
async function fitViewport({ nodes, width, height, panZoom, minZoom, maxZoom }, options) {
    if (nodes.size === 0) {
        return Promise.resolve(true);
    }
    const nodesToFit = getFitViewNodes(nodes, options);
    const bounds = getInternalNodesBounds(nodesToFit);
    const viewport = getViewportForBounds(bounds, width, height, options?.minZoom ?? minZoom, options?.maxZoom ?? maxZoom, options?.padding ?? 0.1);
    await panZoom.setViewport(viewport, {
        duration: options?.duration,
        ease: options?.ease,
        interpolate: options?.interpolate,
    });
    return Promise.resolve(true);
}
/**
 * This function calculates the next position of a node, taking into account the node's extent, parent node, and origin.
 *
 * @internal
 * @returns position, positionAbsolute
 */
function calculateNodePosition({ nodeId, nextPosition, nodeLookup, nodeOrigin = [0, 0], nodeExtent, onError, }) {
    const node = nodeLookup.get(nodeId);
    const parentNode = node.parentId ? nodeLookup.get(node.parentId) : undefined;
    const { x: parentX, y: parentY } = parentNode ? parentNode.internals.positionAbsolute : { x: 0, y: 0 };
    const origin = node.origin ?? nodeOrigin;
    let extent = node.extent || nodeExtent;
    if (node.extent === 'parent' && !node.expandParent) {
        if (!parentNode) {
            onError?.('005', errorMessages['error005']());
        }
        else {
            const parentWidth = parentNode.measured.width;
            const parentHeight = parentNode.measured.height;
            if (parentWidth && parentHeight) {
                extent = [
                    [parentX, parentY],
                    [parentX + parentWidth, parentY + parentHeight],
                ];
            }
        }
    }
    else if (parentNode && isCoordinateExtent(node.extent)) {
        extent = [
            [node.extent[0][0] + parentX, node.extent[0][1] + parentY],
            [node.extent[1][0] + parentX, node.extent[1][1] + parentY],
        ];
    }
    const positionAbsolute = isCoordinateExtent(extent)
        ? clampPosition(nextPosition, extent, node.measured)
        : nextPosition;
    if (node.measured.width === undefined || node.measured.height === undefined) {
        onError?.('015', errorMessages['error015']());
    }
    return {
        position: {
            x: positionAbsolute.x - parentX + (node.measured.width ?? 0) * origin[0],
            y: positionAbsolute.y - parentY + (node.measured.height ?? 0) * origin[1],
        },
        positionAbsolute,
    };
}
/**
 * Pass in nodes & edges to delete, get arrays of nodes and edges that actually can be deleted
 * @internal
 * @param param.nodesToRemove - The nodes to remove
 * @param param.edgesToRemove - The edges to remove
 * @param param.nodes - All nodes
 * @param param.edges - All edges
 * @param param.onBeforeDelete - Callback to check which nodes and edges can be deleted
 * @returns nodes: nodes that can be deleted, edges: edges that can be deleted
 */
async function getElementsToRemove({ nodesToRemove = [], edgesToRemove = [], nodes, edges, onBeforeDelete, }) {
    const nodeIds = new Set(nodesToRemove.map((node) => node.id));
    const matchingNodes = [];
    for (const node of nodes) {
        if (node.deletable === false) {
            continue;
        }
        const isIncluded = nodeIds.has(node.id);
        const parentHit = !isIncluded && node.parentId && matchingNodes.find((n) => n.id === node.parentId);
        if (isIncluded || parentHit) {
            matchingNodes.push(node);
        }
    }
    const edgeIds = new Set(edgesToRemove.map((edge) => edge.id));
    const deletableEdges = edges.filter((edge) => edge.deletable !== false);
    const connectedEdges = getConnectedEdges(matchingNodes, deletableEdges);
    const matchingEdges = connectedEdges;
    for (const edge of deletableEdges) {
        const isIncluded = edgeIds.has(edge.id);
        if (isIncluded && !matchingEdges.find((e) => e.id === edge.id)) {
            matchingEdges.push(edge);
        }
    }
    if (!onBeforeDelete) {
        return {
            edges: matchingEdges,
            nodes: matchingNodes,
        };
    }
    const onBeforeDeleteResult = await onBeforeDelete({
        nodes: matchingNodes,
        edges: matchingEdges,
    });
    if (typeof onBeforeDeleteResult === 'boolean') {
        return onBeforeDeleteResult ? { edges: matchingEdges, nodes: matchingNodes } : { edges: [], nodes: [] };
    }
    return onBeforeDeleteResult;
}

const clamp = (val, min = 0, max = 1) => Math.min(Math.max(val, min), max);
const clampPosition = (position = { x: 0, y: 0 }, extent, dimensions) => ({
    x: clamp(position.x, extent[0][0], extent[1][0] - (dimensions?.width ?? 0)),
    y: clamp(position.y, extent[0][1], extent[1][1] - (dimensions?.height ?? 0)),
});
function clampPositionToParent(childPosition, childDimensions, parent) {
    const { width: parentWidth, height: parentHeight } = getNodeDimensions(parent);
    const { x: parentX, y: parentY } = parent.internals.positionAbsolute;
    return clampPosition(childPosition, [
        [parentX, parentY],
        [parentX + parentWidth, parentY + parentHeight],
    ], childDimensions);
}
/**
 * Calculates the velocity of panning when the mouse is close to the edge of the canvas
 * @internal
 * @param value - One dimensional poition of the mouse (x or y)
 * @param min - Minimal position on canvas before panning starts
 * @param max - Maximal position on canvas before panning starts
 * @returns - A number between 0 and 1 that represents the velocity of panning
 */
const calcAutoPanVelocity = (value, min, max) => {
    if (value < min) {
        return clamp(Math.abs(value - min), 1, min) / min;
    }
    else if (value > max) {
        return -clamp(Math.abs(value - max), 1, min) / min;
    }
    return 0;
};
const calcAutoPan = (pos, bounds, speed = 15, distance = 40) => {
    const xMovement = calcAutoPanVelocity(pos.x, distance, bounds.width - distance) * speed;
    const yMovement = calcAutoPanVelocity(pos.y, distance, bounds.height - distance) * speed;
    return [xMovement, yMovement];
};
const getBoundsOfBoxes = (box1, box2) => ({
    x: Math.min(box1.x, box2.x),
    y: Math.min(box1.y, box2.y),
    x2: Math.max(box1.x2, box2.x2),
    y2: Math.max(box1.y2, box2.y2),
});
const rectToBox = ({ x, y, width, height }) => ({
    x,
    y,
    x2: x + width,
    y2: y + height,
});
const boxToRect = ({ x, y, x2, y2 }) => ({
    x,
    y,
    width: x2 - x,
    height: y2 - y,
});
const nodeToRect = (node, nodeOrigin = [0, 0]) => {
    const { x, y } = isInternalNodeBase(node)
        ? node.internals.positionAbsolute
        : getNodePositionWithOrigin(node, nodeOrigin);
    return {
        x,
        y,
        width: node.measured?.width ?? node.width ?? node.initialWidth ?? 0,
        height: node.measured?.height ?? node.height ?? node.initialHeight ?? 0,
    };
};
const nodeToBox = (node, nodeOrigin = [0, 0]) => {
    const { x, y } = isInternalNodeBase(node)
        ? node.internals.positionAbsolute
        : getNodePositionWithOrigin(node, nodeOrigin);
    return {
        x,
        y,
        x2: x + (node.measured?.width ?? node.width ?? node.initialWidth ?? 0),
        y2: y + (node.measured?.height ?? node.height ?? node.initialHeight ?? 0),
    };
};
const getBoundsOfRects = (rect1, rect2) => boxToRect(getBoundsOfBoxes(rectToBox(rect1), rectToBox(rect2)));
const getOverlappingArea = (rectA, rectB) => {
    const xOverlap = Math.max(0, Math.min(rectA.x + rectA.width, rectB.x + rectB.width) - Math.max(rectA.x, rectB.x));
    const yOverlap = Math.max(0, Math.min(rectA.y + rectA.height, rectB.y + rectB.height) - Math.max(rectA.y, rectB.y));
    return Math.ceil(xOverlap * yOverlap);
};
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const isRectObject = (obj) => isNumeric(obj.width) && isNumeric(obj.height) && isNumeric(obj.x) && isNumeric(obj.y);
/* eslint-disable-next-line @typescript-eslint/no-explicit-any */
const isNumeric = (n) => !isNaN(n) && isFinite(n);
// used for a11y key board controls for nodes and edges
const devWarn = (id, message) => {
    if (process.env.NODE_ENV === 'development') {
        console.warn(`[React Flow]: ${message} Help: https://reactflow.dev/error#${id}`);
    }
};
const snapPosition = (position, snapGrid = [1, 1]) => {
    return {
        x: snapGrid[0] * Math.round(position.x / snapGrid[0]),
        y: snapGrid[1] * Math.round(position.y / snapGrid[1]),
    };
};
const pointToRendererPoint = ({ x, y }, [tx, ty, tScale], snapToGrid = false, snapGrid = [1, 1]) => {
    const position = {
        x: (x - tx) / tScale,
        y: (y - ty) / tScale,
    };
    return snapToGrid ? snapPosition(position, snapGrid) : position;
};
const rendererPointToPoint = ({ x, y }, [tx, ty, tScale]) => {
    return {
        x: x * tScale + tx,
        y: y * tScale + ty,
    };
};
/**
 * Parses a single padding value to a number
 * @internal
 * @param padding - Padding to parse
 * @param viewport - Width or height of the viewport
 * @returns The padding in pixels
 */
function parsePadding(padding, viewport) {
    if (typeof padding === 'number') {
        return Math.floor((viewport - viewport / (1 + padding)) * 0.5);
    }
    if (typeof padding === 'string' && padding.endsWith('px')) {
        const paddingValue = parseFloat(padding);
        if (!Number.isNaN(paddingValue)) {
            return Math.floor(paddingValue);
        }
    }
    if (typeof padding === 'string' && padding.endsWith('%')) {
        const paddingValue = parseFloat(padding);
        if (!Number.isNaN(paddingValue)) {
            return Math.floor(viewport * paddingValue * 0.01);
        }
    }
    console.error(`[React Flow] The padding value "${padding}" is invalid. Please provide a number or a string with a valid unit (px or %).`);
    return 0;
}
/**
 * Parses the paddings to an object with top, right, bottom, left, x and y paddings
 * @internal
 * @param padding - Padding to parse
 * @param width - Width of the viewport
 * @param height - Height of the viewport
 * @returns An object with the paddings in pixels
 */
function parsePaddings(padding, width, height) {
    if (typeof padding === 'string' || typeof padding === 'number') {
        const paddingY = parsePadding(padding, height);
        const paddingX = parsePadding(padding, width);
        return {
            top: paddingY,
            right: paddingX,
            bottom: paddingY,
            left: paddingX,
            x: paddingX * 2,
            y: paddingY * 2,
        };
    }
    if (typeof padding === 'object') {
        const top = parsePadding(padding.top ?? padding.y ?? 0, height);
        const bottom = parsePadding(padding.bottom ?? padding.y ?? 0, height);
        const left = parsePadding(padding.left ?? padding.x ?? 0, width);
        const right = parsePadding(padding.right ?? padding.x ?? 0, width);
        return { top, right, bottom, left, x: left + right, y: top + bottom };
    }
    return { top: 0, right: 0, bottom: 0, left: 0, x: 0, y: 0 };
}
/**
 * Calculates the resulting paddings if the new viewport is applied
 * @internal
 * @param bounds - Bounds to fit inside viewport
 * @param x - X position of the viewport
 * @param y - Y position of the viewport
 * @param zoom - Zoom level of the viewport
 * @param width - Width of the viewport
 * @param height - Height of the viewport
 * @returns An object with the minimum padding required to fit the bounds inside the viewport
 */
function calculateAppliedPaddings(bounds, x, y, zoom, width, height) {
    const { x: left, y: top } = rendererPointToPoint(bounds, [x, y, zoom]);
    const { x: boundRight, y: boundBottom } = rendererPointToPoint({ x: bounds.x + bounds.width, y: bounds.y + bounds.height }, [x, y, zoom]);
    const right = width - boundRight;
    const bottom = height - boundBottom;
    return {
        left: Math.floor(left),
        top: Math.floor(top),
        right: Math.floor(right),
        bottom: Math.floor(bottom),
    };
}
/**
 * Returns a viewport that encloses the given bounds with padding.
 * @public
 * @remarks You can determine bounds of nodes with {@link getNodesBounds} and {@link getBoundsOfRects}
 * @param bounds - Bounds to fit inside viewport.
 * @param width - Width of the viewport.
 * @param height  - Height of the viewport.
 * @param minZoom - Minimum zoom level of the resulting viewport.
 * @param maxZoom - Maximum zoom level of the resulting viewport.
 * @param padding - Padding around the bounds.
 * @returns A transformed {@link Viewport} that encloses the given bounds which you can pass to e.g. {@link setViewport}.
 * @example
 * const { x, y, zoom } = getViewportForBounds(
 * { x: 0, y: 0, width: 100, height: 100},
 * 1200, 800, 0.5, 2);
 */
const getViewportForBounds = (bounds, width, height, minZoom, maxZoom, padding) => {
    // First we resolve all the paddings to actual pixel values
    const p = parsePaddings(padding, width, height);
    const xZoom = (width - p.x) / bounds.width;
    const yZoom = (height - p.y) / bounds.height;
    // We calculate the new x, y, zoom for a centered view
    const zoom = Math.min(xZoom, yZoom);
    const clampedZoom = clamp(zoom, minZoom, maxZoom);
    const boundsCenterX = bounds.x + bounds.width / 2;
    const boundsCenterY = bounds.y + bounds.height / 2;
    const x = width / 2 - boundsCenterX * clampedZoom;
    const y = height / 2 - boundsCenterY * clampedZoom;
    // Then we calculate the minimum padding, to respect asymmetric paddings
    const newPadding = calculateAppliedPaddings(bounds, x, y, clampedZoom, width, height);
    // We only want to have an offset if the newPadding is smaller than the required padding
    const offset = {
        left: Math.min(newPadding.left - p.left, 0),
        top: Math.min(newPadding.top - p.top, 0),
        right: Math.min(newPadding.right - p.right, 0),
        bottom: Math.min(newPadding.bottom - p.bottom, 0),
    };
    return {
        x: x - offset.left + offset.right,
        y: y - offset.top + offset.bottom,
        zoom: clampedZoom,
    };
};
const isMacOs = () => typeof navigator !== 'undefined' && navigator?.userAgent?.indexOf('Mac') >= 0;
function isCoordinateExtent(extent) {
    return extent !== undefined && extent !== null && extent !== 'parent';
}
function getNodeDimensions(node) {
    return {
        width: node.measured?.width ?? node.width ?? node.initialWidth ?? 0,
        height: node.measured?.height ?? node.height ?? node.initialHeight ?? 0,
    };
}
function nodeHasDimensions(node) {
    return ((node.measured?.width ?? node.width ?? node.initialWidth) !== undefined &&
        (node.measured?.height ?? node.height ?? node.initialHeight) !== undefined);
}
/**
 * Convert child position to absolute position
 *
 * @internal
 * @param position
 * @param parentId
 * @param nodeLookup
 * @param nodeOrigin
 * @returns an internal node with an absolute position
 */
function evaluateAbsolutePosition(position, dimensions = { width: 0, height: 0 }, parentId, nodeLookup, nodeOrigin) {
    const positionAbsolute = { ...position };
    const parent = nodeLookup.get(parentId);
    if (parent) {
        const origin = parent.origin || nodeOrigin;
        positionAbsolute.x += parent.internals.positionAbsolute.x - (dimensions.width ?? 0) * origin[0];
        positionAbsolute.y += parent.internals.positionAbsolute.y - (dimensions.height ?? 0) * origin[1];
    }
    return positionAbsolute;
}
function areSetsEqual(a, b) {
    if (a.size !== b.size) {
        return false;
    }
    for (const item of a) {
        if (!b.has(item)) {
            return false;
        }
    }
    return true;
}
/**
 * Polyfill for Promise.withResolvers until we can use it in all browsers
 * @internal
 */
function withResolvers() {
    let resolve;
    let reject;
    const promise = new Promise((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return { promise, resolve, reject };
}
function mergeAriaLabelConfig(partial) {
    return { ...defaultAriaLabelConfig, ...(partial || {}) };
}

function getPointerPosition(event, { snapGrid = [0, 0], snapToGrid = false, transform, containerBounds }) {
    const { x, y } = getEventPosition(event);
    const pointerPos = pointToRendererPoint({ x: x - (containerBounds?.left ?? 0), y: y - (containerBounds?.top ?? 0) }, transform);
    const { x: xSnapped, y: ySnapped } = snapToGrid ? snapPosition(pointerPos, snapGrid) : pointerPos;
    // we need the snapped position in order to be able to skip unnecessary drag events
    return {
        xSnapped,
        ySnapped,
        ...pointerPos,
    };
}
const getDimensions = (node) => ({
    width: node.offsetWidth,
    height: node.offsetHeight,
});
const getHostForElement = (element) => element?.getRootNode?.() || window?.document;
const inputTags = ['INPUT', 'SELECT', 'TEXTAREA'];
function isInputDOMNode(event) {
    // using composed path for handling shadow dom
    const target = (event.composedPath?.()?.[0] || event.target);
    if (target?.nodeType !== 1 /* Node.ELEMENT_NODE */)
        return false;
    const isInput = inputTags.includes(target.nodeName) || target.hasAttribute('contenteditable');
    // when an input field is focused we don't want to trigger deletion or movement of nodes
    return isInput || !!target.closest('.nokey');
}
const isMouseEvent = (event) => 'clientX' in event;
const getEventPosition = (event, bounds) => {
    const isMouse = isMouseEvent(event);
    const evtX = isMouse ? event.clientX : event.touches?.[0].clientX;
    const evtY = isMouse ? event.clientY : event.touches?.[0].clientY;
    return {
        x: evtX - (bounds?.left ?? 0),
        y: evtY - (bounds?.top ?? 0),
    };
};
/*
 * The handle bounds are calculated relative to the node element.
 * We store them in the internals object of the node in order to avoid
 * unnecessary recalculations.
 */
const getHandleBounds = (type, nodeElement, nodeBounds, zoom, nodeId) => {
    const handles = nodeElement.querySelectorAll(`.${type}`);
    if (!handles || !handles.length) {
        return null;
    }
    return Array.from(handles).map((handle) => {
        const handleBounds = handle.getBoundingClientRect();
        return {
            id: handle.getAttribute('data-handleid'),
            type,
            nodeId,
            position: handle.getAttribute('data-handlepos'),
            x: (handleBounds.left - nodeBounds.left) / zoom,
            y: (handleBounds.top - nodeBounds.top) / zoom,
            ...getDimensions(handle),
        };
    });
};

function getBezierEdgeCenter({ sourceX, sourceY, targetX, targetY, sourceControlX, sourceControlY, targetControlX, targetControlY, }) {
    /*
     * cubic bezier t=0.5 mid point, not the actual mid point, but easy to calculate
     * https://stackoverflow.com/questions/67516101/how-to-find-distance-mid-point-of-bezier-curve
     */
    const centerX = sourceX * 0.125 + sourceControlX * 0.375 + targetControlX * 0.375 + targetX * 0.125;
    const centerY = sourceY * 0.125 + sourceControlY * 0.375 + targetControlY * 0.375 + targetY * 0.125;
    const offsetX = Math.abs(centerX - sourceX);
    const offsetY = Math.abs(centerY - sourceY);
    return [centerX, centerY, offsetX, offsetY];
}
function calculateControlOffset(distance, curvature) {
    if (distance >= 0) {
        return 0.5 * distance;
    }
    return curvature * 25 * Math.sqrt(-distance);
}
function getControlWithCurvature({ pos, x1, y1, x2, y2, c }) {
    switch (pos) {
        case Position.Left:
            return [x1 - calculateControlOffset(x1 - x2, c), y1];
        case Position.Right:
            return [x1 + calculateControlOffset(x2 - x1, c), y1];
        case Position.Top:
            return [x1, y1 - calculateControlOffset(y1 - y2, c)];
        case Position.Bottom:
            return [x1, y1 + calculateControlOffset(y2 - y1, c)];
    }
}
/**
 * The `getBezierPath` util returns everything you need to render a bezier edge
 *between two nodes.
 * @public
 * @returns A path string you can use in an SVG, the `labelX` and `labelY` position (center of path)
 * and `offsetX`, `offsetY` between source handle and label.
 * - `path`: the path to use in an SVG `<path>` element.
 * - `labelX`: the `x` position you can use to render a label for this edge.
 * - `labelY`: the `y` position you can use to render a label for this edge.
 * - `offsetX`: the absolute difference between the source `x` position and the `x` position of the
 * middle of this path.
 * - `offsetY`: the absolute difference between the source `y` position and the `y` position of the
 * middle of this path.
 * @example
 * ```js
 *  const source = { x: 0, y: 20 };
 *  const target = { x: 150, y: 100 };
 *
 *  const [path, labelX, labelY, offsetX, offsetY] = getBezierPath({
 *    sourceX: source.x,
 *    sourceY: source.y,
 *    sourcePosition: Position.Right,
 *    targetX: target.x,
 *    targetY: target.y,
 *    targetPosition: Position.Left,
 *});
 *```
 *
 * @remarks This function returns a tuple (aka a fixed-size array) to make it easier to
 *work with multiple edge paths at once.
 */
function getBezierPath({ sourceX, sourceY, sourcePosition = Position.Bottom, targetX, targetY, targetPosition = Position.Top, curvature = 0.25, }) {
    const [sourceControlX, sourceControlY] = getControlWithCurvature({
        pos: sourcePosition,
        x1: sourceX,
        y1: sourceY,
        x2: targetX,
        y2: targetY,
        c: curvature,
    });
    const [targetControlX, targetControlY] = getControlWithCurvature({
        pos: targetPosition,
        x1: targetX,
        y1: targetY,
        x2: sourceX,
        y2: sourceY,
        c: curvature,
    });
    const [labelX, labelY, offsetX, offsetY] = getBezierEdgeCenter({
        sourceX,
        sourceY,
        targetX,
        targetY,
        sourceControlX,
        sourceControlY,
        targetControlX,
        targetControlY,
    });
    return [
        `M${sourceX},${sourceY} C${sourceControlX},${sourceControlY} ${targetControlX},${targetControlY} ${targetX},${targetY}`,
        labelX,
        labelY,
        offsetX,
        offsetY,
    ];
}

// this is used for straight edges and simple smoothstep edges (LTR, RTL, BTT, TTB)
function getEdgeCenter({ sourceX, sourceY, targetX, targetY, }) {
    const xOffset = Math.abs(targetX - sourceX) / 2;
    const centerX = targetX < sourceX ? targetX + xOffset : targetX - xOffset;
    const yOffset = Math.abs(targetY - sourceY) / 2;
    const centerY = targetY < sourceY ? targetY + yOffset : targetY - yOffset;
    return [centerX, centerY, xOffset, yOffset];
}
/**
 * Returns the z-index for an edge based on the node it connects and whether it is selected.
 * By default, edges are rendered below nodes. This behaviour is different for edges that are
 * connected to nodes with a parent, as they are rendered above the parent node.
 */
function getElevatedEdgeZIndex({ sourceNode, targetNode, selected = false, zIndex = 0, elevateOnSelect = false, zIndexMode = 'basic', }) {
    if (zIndexMode === 'manual') {
        return zIndex;
    }
    const edgeZ = elevateOnSelect && selected ? zIndex + 1000 : zIndex;
    const nodeZ = Math.max(sourceNode.parentId || (elevateOnSelect && sourceNode.selected) ? sourceNode.internals.z : 0, targetNode.parentId || (elevateOnSelect && targetNode.selected) ? targetNode.internals.z : 0);
    return edgeZ + nodeZ;
}
function isEdgeVisible({ sourceNode, targetNode, width, height, transform }) {
    const edgeBox = getBoundsOfBoxes(nodeToBox(sourceNode), nodeToBox(targetNode));
    if (edgeBox.x === edgeBox.x2) {
        edgeBox.x2 += 1;
    }
    if (edgeBox.y === edgeBox.y2) {
        edgeBox.y2 += 1;
    }
    const viewRect = {
        x: -transform[0] / transform[2],
        y: -transform[1] / transform[2],
        width: width / transform[2],
        height: height / transform[2],
    };
    return getOverlappingArea(viewRect, boxToRect(edgeBox)) > 0;
}
/**
 * The default edge ID generator function. Generates an ID based on the source, target, and handles.
 * @public
 * @param params - The connection or edge to generate an ID for.
 * @returns The generated edge ID.
 */
const getEdgeId = ({ source, sourceHandle, target, targetHandle }) => `xy-edge__${source}${sourceHandle || ''}-${target}${targetHandle || ''}`;
const connectionExists = (edge, edges) => {
    return edges.some((el) => el.source === edge.source &&
        el.target === edge.target &&
        (el.sourceHandle === edge.sourceHandle || (!el.sourceHandle && !edge.sourceHandle)) &&
        (el.targetHandle === edge.targetHandle || (!el.targetHandle && !edge.targetHandle)));
};
/**
 * This util is a convenience function to add a new Edge to an array of edges. It also performs some validation to make sure you don't add an invalid edge or duplicate an existing one.
 * @public
 * @param edgeParams - Either an `Edge` or a `Connection` you want to add.
 * @param edges - The array of all current edges.
 * @param options - Optional configuration object.
 * @returns A new array of edges with the new edge added.
 *
 * @remarks If an edge with the same `target` and `source` already exists (and the same
 *`targetHandle` and `sourceHandle` if those are set), then this util won't add
 *a new edge even if the `id` property is different.
 *
 */
const addEdge$1 = (edgeParams, edges, options = {}) => {
    if (!edgeParams.source || !edgeParams.target) {
        devWarn('006', errorMessages['error006']());
        return edges;
    }
    const edgeIdGenerator = options.getEdgeId || getEdgeId;
    let edge;
    if (isEdgeBase(edgeParams)) {
        edge = { ...edgeParams };
    }
    else {
        edge = {
            ...edgeParams,
            id: edgeIdGenerator(edgeParams),
        };
    }
    if (connectionExists(edge, edges)) {
        return edges;
    }
    if (edge.sourceHandle === null) {
        delete edge.sourceHandle;
    }
    if (edge.targetHandle === null) {
        delete edge.targetHandle;
    }
    return edges.concat(edge);
};

/**
 * Calculates the straight line path between two points.
 * @public
 * @returns A path string you can use in an SVG, the `labelX` and `labelY` position (center of path)
 * and `offsetX`, `offsetY` between source handle and label.
 *
 * - `path`: the path to use in an SVG `<path>` element.
 * - `labelX`: the `x` position you can use to render a label for this edge.
 * - `labelY`: the `y` position you can use to render a label for this edge.
 * - `offsetX`: the absolute difference between the source `x` position and the `x` position of the
 * middle of this path.
 * - `offsetY`: the absolute difference between the source `y` position and the `y` position of the
 * middle of this path.
 * @example
 * ```js
 *  const source = { x: 0, y: 20 };
 *  const target = { x: 150, y: 100 };
 *
 *  const [path, labelX, labelY, offsetX, offsetY] = getStraightPath({
 *    sourceX: source.x,
 *    sourceY: source.y,
 *    sourcePosition: Position.Right,
 *    targetX: target.x,
 *    targetY: target.y,
 *    targetPosition: Position.Left,
 *  });
 * ```
 * @remarks This function returns a tuple (aka a fixed-size array) to make it easier to work with multiple edge paths at once.
 */
function getStraightPath({ sourceX, sourceY, targetX, targetY, }) {
    const [labelX, labelY, offsetX, offsetY] = getEdgeCenter({
        sourceX,
        sourceY,
        targetX,
        targetY,
    });
    return [`M ${sourceX},${sourceY}L ${targetX},${targetY}`, labelX, labelY, offsetX, offsetY];
}

const handleDirections = {
    [Position.Left]: { x: -1, y: 0 },
    [Position.Right]: { x: 1, y: 0 },
    [Position.Top]: { x: 0, y: -1 },
    [Position.Bottom]: { x: 0, y: 1 },
};
const getDirection = ({ source, sourcePosition = Position.Bottom, target, }) => {
    if (sourcePosition === Position.Left || sourcePosition === Position.Right) {
        return source.x < target.x ? { x: 1, y: 0 } : { x: -1, y: 0 };
    }
    return source.y < target.y ? { x: 0, y: 1 } : { x: 0, y: -1 };
};
const distance = (a, b) => Math.sqrt(Math.pow(b.x - a.x, 2) + Math.pow(b.y - a.y, 2));
/*
 * With this function we try to mimic an orthogonal edge routing behaviour
 * It's not as good as a real orthogonal edge routing, but it's faster and good enough as a default for step and smooth step edges
 */
function getPoints({ source, sourcePosition = Position.Bottom, target, targetPosition = Position.Top, center, offset, stepPosition, }) {
    const sourceDir = handleDirections[sourcePosition];
    const targetDir = handleDirections[targetPosition];
    const sourceGapped = { x: source.x + sourceDir.x * offset, y: source.y + sourceDir.y * offset };
    const targetGapped = { x: target.x + targetDir.x * offset, y: target.y + targetDir.y * offset };
    const dir = getDirection({
        source: sourceGapped,
        sourcePosition,
        target: targetGapped,
    });
    const dirAccessor = dir.x !== 0 ? 'x' : 'y';
    const currDir = dir[dirAccessor];
    let points = [];
    let centerX, centerY;
    const sourceGapOffset = { x: 0, y: 0 };
    const targetGapOffset = { x: 0, y: 0 };
    const [, , defaultOffsetX, defaultOffsetY] = getEdgeCenter({
        sourceX: source.x,
        sourceY: source.y,
        targetX: target.x,
        targetY: target.y,
    });
    // opposite handle positions, default case
    if (sourceDir[dirAccessor] * targetDir[dirAccessor] === -1) {
        if (dirAccessor === 'x') {
            // Primary direction is horizontal, so stepPosition affects X coordinate
            centerX = center.x ?? sourceGapped.x + (targetGapped.x - sourceGapped.x) * stepPosition;
            centerY = center.y ?? (sourceGapped.y + targetGapped.y) / 2;
        }
        else {
            // Primary direction is vertical, so stepPosition affects Y coordinate
            centerX = center.x ?? (sourceGapped.x + targetGapped.x) / 2;
            centerY = center.y ?? sourceGapped.y + (targetGapped.y - sourceGapped.y) * stepPosition;
        }
        /*
         *    --->
         *    |
         * >---
         */
        const verticalSplit = [
            { x: centerX, y: sourceGapped.y },
            { x: centerX, y: targetGapped.y },
        ];
        /*
         *    |
         *  ---
         *  |
         */
        const horizontalSplit = [
            { x: sourceGapped.x, y: centerY },
            { x: targetGapped.x, y: centerY },
        ];
        if (sourceDir[dirAccessor] === currDir) {
            points = dirAccessor === 'x' ? verticalSplit : horizontalSplit;
        }
        else {
            points = dirAccessor === 'x' ? horizontalSplit : verticalSplit;
        }
    }
    else {
        // sourceTarget means we take x from source and y from target, targetSource is the opposite
        const sourceTarget = [{ x: sourceGapped.x, y: targetGapped.y }];
        const targetSource = [{ x: targetGapped.x, y: sourceGapped.y }];
        // this handles edges with same handle positions
        if (dirAccessor === 'x') {
            points = sourceDir.x === currDir ? targetSource : sourceTarget;
        }
        else {
            points = sourceDir.y === currDir ? sourceTarget : targetSource;
        }
        if (sourcePosition === targetPosition) {
            const diff = Math.abs(source[dirAccessor] - target[dirAccessor]);
            // if an edge goes from right to right for example (sourcePosition === targetPosition) and the distance between source.x and target.x is less than the offset, the added point and the gapped source/target will overlap. This leads to a weird edge path. To avoid this we add a gapOffset to the source/target
            if (diff <= offset) {
                const gapOffset = Math.min(offset - 1, offset - diff);
                if (sourceDir[dirAccessor] === currDir) {
                    sourceGapOffset[dirAccessor] = (sourceGapped[dirAccessor] > source[dirAccessor] ? -1 : 1) * gapOffset;
                }
                else {
                    targetGapOffset[dirAccessor] = (targetGapped[dirAccessor] > target[dirAccessor] ? -1 : 1) * gapOffset;
                }
            }
        }
        // these are conditions for handling mixed handle positions like Right -> Bottom for example
        if (sourcePosition !== targetPosition) {
            const dirAccessorOpposite = dirAccessor === 'x' ? 'y' : 'x';
            const isSameDir = sourceDir[dirAccessor] === targetDir[dirAccessorOpposite];
            const sourceGtTargetOppo = sourceGapped[dirAccessorOpposite] > targetGapped[dirAccessorOpposite];
            const sourceLtTargetOppo = sourceGapped[dirAccessorOpposite] < targetGapped[dirAccessorOpposite];
            const flipSourceTarget = (sourceDir[dirAccessor] === 1 && ((!isSameDir && sourceGtTargetOppo) || (isSameDir && sourceLtTargetOppo))) ||
                (sourceDir[dirAccessor] !== 1 && ((!isSameDir && sourceLtTargetOppo) || (isSameDir && sourceGtTargetOppo)));
            if (flipSourceTarget) {
                points = dirAccessor === 'x' ? sourceTarget : targetSource;
            }
        }
        const sourceGapPoint = { x: sourceGapped.x + sourceGapOffset.x, y: sourceGapped.y + sourceGapOffset.y };
        const targetGapPoint = { x: targetGapped.x + targetGapOffset.x, y: targetGapped.y + targetGapOffset.y };
        const maxXDistance = Math.max(Math.abs(sourceGapPoint.x - points[0].x), Math.abs(targetGapPoint.x - points[0].x));
        const maxYDistance = Math.max(Math.abs(sourceGapPoint.y - points[0].y), Math.abs(targetGapPoint.y - points[0].y));
        // we want to place the label on the longest segment of the edge
        if (maxXDistance >= maxYDistance) {
            centerX = (sourceGapPoint.x + targetGapPoint.x) / 2;
            centerY = points[0].y;
        }
        else {
            centerX = points[0].x;
            centerY = (sourceGapPoint.y + targetGapPoint.y) / 2;
        }
    }
    const gappedSource = { x: sourceGapped.x + sourceGapOffset.x, y: sourceGapped.y + sourceGapOffset.y };
    const gappedTarget = { x: targetGapped.x + targetGapOffset.x, y: targetGapped.y + targetGapOffset.y };
    const pathPoints = [
        source,
        // we only want to add the gapped source/target if they are different from the first/last point to avoid duplicates which can cause issues with the bends
        ...(gappedSource.x !== points[0].x || gappedSource.y !== points[0].y ? [gappedSource] : []),
        ...points,
        ...(gappedTarget.x !== points[points.length - 1].x || gappedTarget.y !== points[points.length - 1].y
            ? [gappedTarget]
            : []),
        target,
    ];
    return [pathPoints, centerX, centerY, defaultOffsetX, defaultOffsetY];
}
function getBend(a, b, c, size) {
    const bendSize = Math.min(distance(a, b) / 2, distance(b, c) / 2, size);
    const { x, y } = b;
    // no bend
    if ((a.x === x && x === c.x) || (a.y === y && y === c.y)) {
        return `L${x} ${y}`;
    }
    // first segment is horizontal
    if (a.y === y) {
        const xDir = a.x < c.x ? -1 : 1;
        const yDir = a.y < c.y ? 1 : -1;
        return `L ${x + bendSize * xDir},${y}Q ${x},${y} ${x},${y + bendSize * yDir}`;
    }
    const xDir = a.x < c.x ? 1 : -1;
    const yDir = a.y < c.y ? -1 : 1;
    return `L ${x},${y + bendSize * yDir}Q ${x},${y} ${x + bendSize * xDir},${y}`;
}
/**
 * The `getSmoothStepPath` util returns everything you need to render a stepped path
 * between two nodes. The `borderRadius` property can be used to choose how rounded
 * the corners of those steps are.
 * @public
 * @returns A path string you can use in an SVG, the `labelX` and `labelY` position (center of path)
 * and `offsetX`, `offsetY` between source handle and label.
 *
 * - `path`: the path to use in an SVG `<path>` element.
 * - `labelX`: the `x` position you can use to render a label for this edge.
 * - `labelY`: the `y` position you can use to render a label for this edge.
 * - `offsetX`: the absolute difference between the source `x` position and the `x` position of the
 * middle of this path.
 * - `offsetY`: the absolute difference between the source `y` position and the `y` position of the
 * middle of this path.
 * @example
 * ```js
 *  const source = { x: 0, y: 20 };
 *  const target = { x: 150, y: 100 };
 *
 *  const [path, labelX, labelY, offsetX, offsetY] = getSmoothStepPath({
 *    sourceX: source.x,
 *    sourceY: source.y,
 *    sourcePosition: Position.Right,
 *    targetX: target.x,
 *    targetY: target.y,
 *    targetPosition: Position.Left,
 *  });
 * ```
 * @remarks This function returns a tuple (aka a fixed-size array) to make it easier to work with multiple edge paths at once.
 */
function getSmoothStepPath({ sourceX, sourceY, sourcePosition = Position.Bottom, targetX, targetY, targetPosition = Position.Top, borderRadius = 5, centerX, centerY, offset = 20, stepPosition = 0.5, }) {
    const [points, labelX, labelY, offsetX, offsetY] = getPoints({
        source: { x: sourceX, y: sourceY },
        sourcePosition,
        target: { x: targetX, y: targetY },
        targetPosition,
        center: { x: centerX, y: centerY },
        offset,
        stepPosition,
    });
    let path = `M${points[0].x} ${points[0].y}`;
    for (let i = 1; i < points.length - 1; i++) {
        path += getBend(points[i - 1], points[i], points[i + 1], borderRadius);
    }
    path += `L${points[points.length - 1].x} ${points[points.length - 1].y}`;
    return [path, labelX, labelY, offsetX, offsetY];
}

function isNodeInitialized(node) {
    return (node &&
        !!(node.internals.handleBounds || node.handles?.length) &&
        !!(node.measured.width || node.width || node.initialWidth));
}
function getEdgePosition(params) {
    const { sourceNode, targetNode } = params;
    if (!isNodeInitialized(sourceNode) || !isNodeInitialized(targetNode)) {
        return null;
    }
    const sourceHandleBounds = sourceNode.internals.handleBounds || toHandleBounds(sourceNode.handles);
    const targetHandleBounds = targetNode.internals.handleBounds || toHandleBounds(targetNode.handles);
    const sourceHandle = getHandle$1(sourceHandleBounds?.source ?? [], params.sourceHandle);
    const targetHandle = getHandle$1(
    // when connection type is loose we can define all handles as sources and connect source -> source
    params.connectionMode === ConnectionMode.Strict
        ? targetHandleBounds?.target ?? []
        : (targetHandleBounds?.target ?? []).concat(targetHandleBounds?.source ?? []), params.targetHandle);
    if (!sourceHandle || !targetHandle) {
        params.onError?.('008', errorMessages['error008'](!sourceHandle ? 'source' : 'target', {
            id: params.id,
            sourceHandle: params.sourceHandle,
            targetHandle: params.targetHandle,
        }));
        return null;
    }
    const sourcePosition = sourceHandle?.position || Position.Bottom;
    const targetPosition = targetHandle?.position || Position.Top;
    const source = getHandlePosition(sourceNode, sourceHandle, sourcePosition);
    const target = getHandlePosition(targetNode, targetHandle, targetPosition);
    return {
        sourceX: source.x,
        sourceY: source.y,
        targetX: target.x,
        targetY: target.y,
        sourcePosition,
        targetPosition,
    };
}
function toHandleBounds(handles) {
    if (!handles) {
        return null;
    }
    const source = [];
    const target = [];
    for (const handle of handles) {
        handle.width = handle.width ?? 1;
        handle.height = handle.height ?? 1;
        if (handle.type === 'source') {
            source.push(handle);
        }
        else if (handle.type === 'target') {
            target.push(handle);
        }
    }
    return {
        source,
        target,
    };
}
function getHandlePosition(node, handle, fallbackPosition = Position.Left, center = false) {
    const x = (handle?.x ?? 0) + node.internals.positionAbsolute.x;
    const y = (handle?.y ?? 0) + node.internals.positionAbsolute.y;
    const { width, height } = handle ?? getNodeDimensions(node);
    if (center) {
        return { x: x + width / 2, y: y + height / 2 };
    }
    const position = handle?.position ?? fallbackPosition;
    switch (position) {
        case Position.Top:
            return { x: x + width / 2, y };
        case Position.Right:
            return { x: x + width, y: y + height / 2 };
        case Position.Bottom:
            return { x: x + width / 2, y: y + height };
        case Position.Left:
            return { x, y: y + height / 2 };
    }
}
function getHandle$1(bounds, handleId) {
    if (!bounds) {
        return null;
    }
    // if no handleId is given, we use the first handle, otherwise we check for the id
    return (!handleId ? bounds[0] : bounds.find((d) => d.id === handleId)) || null;
}

function getMarkerId(marker, id) {
    if (!marker) {
        return '';
    }
    if (typeof marker === 'string') {
        return marker;
    }
    const idPrefix = id ? `${id}__` : '';
    return `${idPrefix}${Object.keys(marker)
        .sort()
        .map((key) => `${key}=${marker[key]}`)
        .join('&')}`;
}
function createMarkerIds(edges, { id, defaultColor, defaultMarkerStart, defaultMarkerEnd, }) {
    const ids = new Set();
    return edges
        .reduce((markers, edge) => {
        [edge.markerStart || defaultMarkerStart, edge.markerEnd || defaultMarkerEnd].forEach((marker) => {
            if (marker && typeof marker === 'object') {
                const markerId = getMarkerId(marker, id);
                if (!ids.has(markerId)) {
                    markers.push({ id: markerId, color: marker.color || defaultColor, ...marker });
                    ids.add(markerId);
                }
            }
        });
        return markers;
    }, [])
        .sort((a, b) => a.id.localeCompare(b.id));
}

const SELECTED_NODE_Z = 1000;
const ROOT_PARENT_Z_INCREMENT = 10;
const defaultOptions = {
    nodeOrigin: [0, 0],
    nodeExtent: infiniteExtent,
    elevateNodesOnSelect: true,
    zIndexMode: 'basic',
    defaults: {},
};
const adoptUserNodesDefaultOptions = {
    ...defaultOptions,
    checkEquality: true,
};
function mergeObjects(base, incoming) {
    const result = { ...base };
    for (const key in incoming) {
        if (incoming[key] !== undefined) {
            // typecast is safe here, because we check for undefined
            result[key] = incoming[key];
        }
    }
    return result;
}
function updateAbsolutePositions(nodeLookup, parentLookup, options) {
    const _options = mergeObjects(defaultOptions, options);
    for (const node of nodeLookup.values()) {
        if (node.parentId) {
            updateChildNode(node, nodeLookup, parentLookup, _options);
        }
        else {
            const positionWithOrigin = getNodePositionWithOrigin(node, _options.nodeOrigin);
            const extent = isCoordinateExtent(node.extent) ? node.extent : _options.nodeExtent;
            const clampedPosition = clampPosition(positionWithOrigin, extent, getNodeDimensions(node));
            node.internals.positionAbsolute = clampedPosition;
        }
    }
}
function parseHandles(userNode, internalNode) {
    if (!userNode.handles) {
        return !userNode.measured ? undefined : internalNode?.internals.handleBounds;
    }
    const source = [];
    const target = [];
    for (const handle of userNode.handles) {
        const handleBounds = {
            id: handle.id,
            width: handle.width ?? 1,
            height: handle.height ?? 1,
            nodeId: userNode.id,
            x: handle.x,
            y: handle.y,
            position: handle.position,
            type: handle.type,
        };
        if (handle.type === 'source') {
            source.push(handleBounds);
        }
        else if (handle.type === 'target') {
            target.push(handleBounds);
        }
    }
    return {
        source,
        target,
    };
}
function isManualZIndexMode(zIndexMode) {
    return zIndexMode === 'manual';
}
function adoptUserNodes(nodes, nodeLookup, parentLookup, options = {}) {
    const _options = mergeObjects(adoptUserNodesDefaultOptions, options);
    const rootParentIndex = { i: 0 };
    const tmpLookup = new Map(nodeLookup);
    const selectedNodeZ = _options?.elevateNodesOnSelect && !isManualZIndexMode(_options.zIndexMode) ? SELECTED_NODE_Z : 0;
    let nodesInitialized = nodes.length > 0;
    let hasSelectedNodes = false;
    nodeLookup.clear();
    parentLookup.clear();
    for (const userNode of nodes) {
        let internalNode = tmpLookup.get(userNode.id);
        if (_options.checkEquality && userNode === internalNode?.internals.userNode) {
            nodeLookup.set(userNode.id, internalNode);
        }
        else {
            const positionWithOrigin = getNodePositionWithOrigin(userNode, _options.nodeOrigin);
            const extent = isCoordinateExtent(userNode.extent) ? userNode.extent : _options.nodeExtent;
            const clampedPosition = clampPosition(positionWithOrigin, extent, getNodeDimensions(userNode));
            internalNode = {
                ..._options.defaults,
                ...userNode,
                measured: {
                    width: userNode.measured?.width,
                    height: userNode.measured?.height,
                },
                internals: {
                    positionAbsolute: clampedPosition,
                    // if user re-initializes the node or removes `measured` for whatever reason, we reset the handleBounds so that the node gets re-measured
                    handleBounds: parseHandles(userNode, internalNode),
                    z: calculateZ(userNode, selectedNodeZ, _options.zIndexMode),
                    userNode,
                },
            };
            nodeLookup.set(userNode.id, internalNode);
        }
        if ((internalNode.measured === undefined ||
            internalNode.measured.width === undefined ||
            internalNode.measured.height === undefined) &&
            !internalNode.hidden) {
            nodesInitialized = false;
        }
        if (userNode.parentId) {
            updateChildNode(internalNode, nodeLookup, parentLookup, options, rootParentIndex);
        }
        hasSelectedNodes ||= userNode.selected ?? false;
    }
    return { nodesInitialized, hasSelectedNodes };
}
function updateParentLookup(node, parentLookup) {
    if (!node.parentId) {
        return;
    }
    const childNodes = parentLookup.get(node.parentId);
    if (childNodes) {
        childNodes.set(node.id, node);
    }
    else {
        parentLookup.set(node.parentId, new Map([[node.id, node]]));
    }
}
/**
 * Updates positionAbsolute and zIndex of a child node and the parentLookup.
 */
function updateChildNode(node, nodeLookup, parentLookup, options, rootParentIndex) {
    const { elevateNodesOnSelect, nodeOrigin, nodeExtent, zIndexMode } = mergeObjects(defaultOptions, options);
    const parentId = node.parentId;
    const parentNode = nodeLookup.get(parentId);
    if (!parentNode) {
        console.warn(`Parent node ${parentId} not found. Please make sure that parent nodes are in front of their child nodes in the nodes array.`);
        return;
    }
    updateParentLookup(node, parentLookup);
    // We just want to set the rootParentIndex for the first child
    if (rootParentIndex &&
        !parentNode.parentId &&
        parentNode.internals.rootParentIndex === undefined &&
        zIndexMode === 'auto') {
        parentNode.internals.rootParentIndex = ++rootParentIndex.i;
        parentNode.internals.z = parentNode.internals.z + rootParentIndex.i * ROOT_PARENT_Z_INCREMENT;
    }
    // But we need to update rootParentIndex.i also when parent has not been updated
    if (rootParentIndex && parentNode.internals.rootParentIndex !== undefined) {
        rootParentIndex.i = parentNode.internals.rootParentIndex;
    }
    const selectedNodeZ = elevateNodesOnSelect && !isManualZIndexMode(zIndexMode) ? SELECTED_NODE_Z : 0;
    const { x, y, z } = calculateChildXYZ(node, parentNode, nodeOrigin, nodeExtent, selectedNodeZ, zIndexMode);
    const { positionAbsolute } = node.internals;
    const positionChanged = x !== positionAbsolute.x || y !== positionAbsolute.y;
    if (positionChanged || z !== node.internals.z) {
        // we create a new object to mark the node as updated
        nodeLookup.set(node.id, {
            ...node,
            internals: {
                ...node.internals,
                positionAbsolute: positionChanged ? { x, y } : positionAbsolute,
                z,
            },
        });
    }
}
function calculateZ(node, selectedNodeZ, zIndexMode) {
    const zIndex = isNumeric(node.zIndex) ? node.zIndex : 0;
    if (isManualZIndexMode(zIndexMode)) {
        return zIndex;
    }
    return zIndex + (node.selected ? selectedNodeZ : 0);
}
function calculateChildXYZ(childNode, parentNode, nodeOrigin, nodeExtent, selectedNodeZ, zIndexMode) {
    const { x: parentX, y: parentY } = parentNode.internals.positionAbsolute;
    const childDimensions = getNodeDimensions(childNode);
    const positionWithOrigin = getNodePositionWithOrigin(childNode, nodeOrigin);
    const clampedPosition = isCoordinateExtent(childNode.extent)
        ? clampPosition(positionWithOrigin, childNode.extent, childDimensions)
        : positionWithOrigin;
    let absolutePosition = clampPosition({ x: parentX + clampedPosition.x, y: parentY + clampedPosition.y }, nodeExtent, childDimensions);
    if (childNode.extent === 'parent') {
        absolutePosition = clampPositionToParent(absolutePosition, childDimensions, parentNode);
    }
    const childZ = calculateZ(childNode, selectedNodeZ, zIndexMode);
    const parentZ = parentNode.internals.z ?? 0;
    return {
        x: absolutePosition.x,
        y: absolutePosition.y,
        z: parentZ >= childZ ? parentZ + 1 : childZ,
    };
}
function handleExpandParent(children, nodeLookup, parentLookup, nodeOrigin = [0, 0]) {
    const changes = [];
    const parentExpansions = new Map();
    // determine the expanded rectangle the child nodes would take for each parent
    for (const child of children) {
        const parent = nodeLookup.get(child.parentId);
        if (!parent) {
            continue;
        }
        const parentRect = parentExpansions.get(child.parentId)?.expandedRect ?? nodeToRect(parent);
        const expandedRect = getBoundsOfRects(parentRect, child.rect);
        parentExpansions.set(child.parentId, { expandedRect, parent });
    }
    if (parentExpansions.size > 0) {
        parentExpansions.forEach(({ expandedRect, parent }, parentId) => {
            // determine the position & dimensions of the parent
            const positionAbsolute = parent.internals.positionAbsolute;
            const dimensions = getNodeDimensions(parent);
            const origin = parent.origin ?? nodeOrigin;
            // determine how much the parent expands in width and position
            const xChange = expandedRect.x < positionAbsolute.x ? Math.round(Math.abs(positionAbsolute.x - expandedRect.x)) : 0;
            const yChange = expandedRect.y < positionAbsolute.y ? Math.round(Math.abs(positionAbsolute.y - expandedRect.y)) : 0;
            const newWidth = Math.max(dimensions.width, Math.round(expandedRect.width));
            const newHeight = Math.max(dimensions.height, Math.round(expandedRect.height));
            const widthChange = (newWidth - dimensions.width) * origin[0];
            const heightChange = (newHeight - dimensions.height) * origin[1];
            // We need to correct the position of the parent node if the origin is not [0,0]
            if (xChange > 0 || yChange > 0 || widthChange || heightChange) {
                changes.push({
                    id: parentId,
                    type: 'position',
                    position: {
                        x: parent.position.x - xChange + widthChange,
                        y: parent.position.y - yChange + heightChange,
                    },
                });
                /*
                 * We move all child nodes in the oppsite direction
                 * so the x,y changes of the parent do not move the children
                 */
                parentLookup.get(parentId)?.forEach((childNode) => {
                    if (!children.some((child) => child.id === childNode.id)) {
                        changes.push({
                            id: childNode.id,
                            type: 'position',
                            position: {
                                x: childNode.position.x + xChange,
                                y: childNode.position.y + yChange,
                            },
                        });
                    }
                });
            }
            // We need to correct the dimensions of the parent node if the origin is not [0,0]
            if (dimensions.width < expandedRect.width || dimensions.height < expandedRect.height || xChange || yChange) {
                changes.push({
                    id: parentId,
                    type: 'dimensions',
                    setAttributes: true,
                    dimensions: {
                        width: newWidth + (xChange ? origin[0] * xChange - widthChange : 0),
                        height: newHeight + (yChange ? origin[1] * yChange - heightChange : 0),
                    },
                });
            }
        });
    }
    return changes;
}
function updateNodeInternals(updates, nodeLookup, parentLookup, domNode, nodeOrigin, nodeExtent, zIndexMode) {
    const viewportNode = domNode?.querySelector('.xyflow__viewport');
    let updatedInternals = false;
    if (!viewportNode) {
        return { changes: [], updatedInternals };
    }
    const changes = [];
    const style = window.getComputedStyle(viewportNode);
    const { m22: zoom } = new window.DOMMatrixReadOnly(style.transform);
    // in this array we collect nodes, that might trigger changes (like expanding parent)
    const parentExpandChildren = [];
    for (const update of updates.values()) {
        const node = nodeLookup.get(update.id);
        if (!node) {
            continue;
        }
        if (node.hidden) {
            nodeLookup.set(node.id, {
                ...node,
                internals: {
                    ...node.internals,
                    handleBounds: undefined,
                },
            });
            updatedInternals = true;
            continue;
        }
        const dimensions = getDimensions(update.nodeElement);
        const dimensionChanged = node.measured.width !== dimensions.width || node.measured.height !== dimensions.height;
        const doUpdate = !!(dimensions.width &&
            dimensions.height &&
            (dimensionChanged || !node.internals.handleBounds || update.force));
        if (doUpdate) {
            const nodeBounds = update.nodeElement.getBoundingClientRect();
            const extent = isCoordinateExtent(node.extent) ? node.extent : nodeExtent;
            let { positionAbsolute } = node.internals;
            if (node.parentId && node.extent === 'parent') {
                positionAbsolute = clampPositionToParent(positionAbsolute, dimensions, nodeLookup.get(node.parentId));
            }
            else if (extent) {
                positionAbsolute = clampPosition(positionAbsolute, extent, dimensions);
            }
            const newNode = {
                ...node,
                measured: dimensions,
                internals: {
                    ...node.internals,
                    positionAbsolute,
                    handleBounds: {
                        source: getHandleBounds('source', update.nodeElement, nodeBounds, zoom, node.id),
                        target: getHandleBounds('target', update.nodeElement, nodeBounds, zoom, node.id),
                    },
                },
            };
            nodeLookup.set(node.id, newNode);
            if (node.parentId) {
                updateChildNode(newNode, nodeLookup, parentLookup, { nodeOrigin, zIndexMode });
            }
            updatedInternals = true;
            if (dimensionChanged) {
                changes.push({
                    id: node.id,
                    type: 'dimensions',
                    dimensions,
                });
                if (node.expandParent && node.parentId) {
                    parentExpandChildren.push({
                        id: node.id,
                        parentId: node.parentId,
                        rect: nodeToRect(newNode, nodeOrigin),
                    });
                }
            }
        }
    }
    if (parentExpandChildren.length > 0) {
        const parentExpandChanges = handleExpandParent(parentExpandChildren, nodeLookup, parentLookup, nodeOrigin);
        changes.push(...parentExpandChanges);
    }
    return { changes, updatedInternals };
}
async function panBy({ delta, panZoom, transform, translateExtent, width, height, }) {
    if (!panZoom || (!delta.x && !delta.y)) {
        return Promise.resolve(false);
    }
    const nextViewport = await panZoom.setViewportConstrained({
        x: transform[0] + delta.x,
        y: transform[1] + delta.y,
        zoom: transform[2],
    }, [
        [0, 0],
        [width, height],
    ], translateExtent);
    const transformChanged = !!nextViewport &&
        (nextViewport.x !== transform[0] || nextViewport.y !== transform[1] || nextViewport.k !== transform[2]);
    return Promise.resolve(transformChanged);
}
/**
 * this function adds the connection to the connectionLookup
 * at the following keys: nodeId-type-handleId, nodeId-type and nodeId
 * @param type type of the connection
 * @param connection connection that should be added to the lookup
 * @param connectionKey at which key the connection should be added
 * @param connectionLookup reference to the connection lookup
 * @param nodeId nodeId of the connection
 * @param handleId handleId of the connection
 */
function addConnectionToLookup(type, connection, connectionKey, connectionLookup, nodeId, handleId) {
    /*
     * We add the connection to the connectionLookup at the following keys
     * 1. nodeId, 2. nodeId-type, 3. nodeId-type-handleId
     * If the key already exists, we add the connection to the existing map
     */
    let key = nodeId;
    const nodeMap = connectionLookup.get(key) || new Map();
    connectionLookup.set(key, nodeMap.set(connectionKey, connection));
    key = `${nodeId}-${type}`;
    const typeMap = connectionLookup.get(key) || new Map();
    connectionLookup.set(key, typeMap.set(connectionKey, connection));
    if (handleId) {
        key = `${nodeId}-${type}-${handleId}`;
        const handleMap = connectionLookup.get(key) || new Map();
        connectionLookup.set(key, handleMap.set(connectionKey, connection));
    }
}
function updateConnectionLookup(connectionLookup, edgeLookup, edges) {
    connectionLookup.clear();
    edgeLookup.clear();
    for (const edge of edges) {
        const { source: sourceNode, target: targetNode, sourceHandle = null, targetHandle = null } = edge;
        const connection = { edgeId: edge.id, source: sourceNode, target: targetNode, sourceHandle, targetHandle };
        const sourceKey = `${sourceNode}-${sourceHandle}--${targetNode}-${targetHandle}`;
        const targetKey = `${targetNode}-${targetHandle}--${sourceNode}-${sourceHandle}`;
        addConnectionToLookup('source', connection, targetKey, connectionLookup, sourceNode, sourceHandle);
        addConnectionToLookup('target', connection, sourceKey, connectionLookup, targetNode, targetHandle);
        edgeLookup.set(edge.id, edge);
    }
}

function isParentSelected(node, nodeLookup) {
    if (!node.parentId) {
        return false;
    }
    const parentNode = nodeLookup.get(node.parentId);
    if (!parentNode) {
        return false;
    }
    if (parentNode.selected) {
        return true;
    }
    return isParentSelected(parentNode, nodeLookup);
}
function hasSelector(target, selector, domNode) {
    let current = target;
    do {
        if (current?.matches?.(selector))
            return true;
        if (current === domNode)
            return false;
        current = current?.parentElement;
    } while (current);
    return false;
}
// looks for all selected nodes and created a NodeDragItem for each of them
function getDragItems(nodeLookup, nodesDraggable, mousePos, nodeId) {
    const dragItems = new Map();
    for (const [id, node] of nodeLookup) {
        if ((node.selected || node.id === nodeId) &&
            (!node.parentId || !isParentSelected(node, nodeLookup)) &&
            (node.draggable || (nodesDraggable && typeof node.draggable === 'undefined'))) {
            const internalNode = nodeLookup.get(id);
            if (internalNode) {
                dragItems.set(id, {
                    id,
                    position: internalNode.position || { x: 0, y: 0 },
                    distance: {
                        x: mousePos.x - internalNode.internals.positionAbsolute.x,
                        y: mousePos.y - internalNode.internals.positionAbsolute.y,
                    },
                    extent: internalNode.extent,
                    parentId: internalNode.parentId,
                    origin: internalNode.origin,
                    expandParent: internalNode.expandParent,
                    internals: {
                        positionAbsolute: internalNode.internals.positionAbsolute || { x: 0, y: 0 },
                    },
                    measured: {
                        width: internalNode.measured.width ?? 0,
                        height: internalNode.measured.height ?? 0,
                    },
                });
            }
        }
    }
    return dragItems;
}
/*
 * returns two params:
 * 1. the dragged node (or the first of the list, if we are dragging a node selection)
 * 2. array of selected nodes (for multi selections)
 */
function getEventHandlerParams({ nodeId, dragItems, nodeLookup, dragging = true, }) {
    const nodesFromDragItems = [];
    for (const [id, dragItem] of dragItems) {
        const node = nodeLookup.get(id)?.internals.userNode;
        if (node) {
            nodesFromDragItems.push({
                ...node,
                position: dragItem.position,
                dragging,
            });
        }
    }
    if (!nodeId) {
        return [nodesFromDragItems[0], nodesFromDragItems];
    }
    const node = nodeLookup.get(nodeId)?.internals.userNode;
    return [
        !node
            ? nodesFromDragItems[0]
            : {
                ...node,
                position: dragItems.get(nodeId)?.position || node.position,
                dragging,
            },
        nodesFromDragItems,
    ];
}
/**
 * If a selection is being dragged we want to apply the same snap offset to all nodes in the selection.
 * This function calculates the snap offset based on the first node in the selection.
 */
function calculateSnapOffset({ dragItems, snapGrid, x, y, }) {
    const refDragItem = dragItems.values().next().value;
    if (!refDragItem) {
        return null;
    }
    const refPos = {
        x: x - refDragItem.distance.x,
        y: y - refDragItem.distance.y,
    };
    const refPosSnapped = snapPosition(refPos, snapGrid);
    return {
        x: refPosSnapped.x - refPos.x,
        y: refPosSnapped.y - refPos.y,
    };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function XYDrag({ onNodeMouseDown, getStoreItems, onDragStart, onDrag, onDragStop, }) {
    let lastPos = { x: null, y: null };
    let autoPanId = 0;
    let dragItems = new Map();
    let autoPanStarted = false;
    let mousePosition = { x: 0, y: 0 };
    let containerBounds = null;
    let dragStarted = false;
    let d3Selection = null;
    let abortDrag = false; // prevents unintentional dragging on multitouch
    let nodePositionsChanged = false;
    // we store the last drag event to be able to use it in the update function
    let dragEvent = null;
    // public functions
    function update({ noDragClassName, handleSelector, domNode, isSelectable, nodeId, nodeClickDistance = 0, }) {
        d3Selection = select(domNode);
        function updateNodes({ x, y }) {
            const { nodeLookup, nodeExtent, snapGrid, snapToGrid, nodeOrigin, onNodeDrag, onSelectionDrag, onError, updateNodePositions, } = getStoreItems();
            lastPos = { x, y };
            let hasChange = false;
            const isMultiDrag = dragItems.size > 1;
            const nodesBox = isMultiDrag && nodeExtent ? rectToBox(getInternalNodesBounds(dragItems)) : null;
            const multiDragSnapOffset = isMultiDrag && snapToGrid
                ? calculateSnapOffset({
                    dragItems,
                    snapGrid,
                    x,
                    y,
                })
                : null;
            for (const [id, dragItem] of dragItems) {
                /*
                 * if the node is not in the nodeLookup anymore, it was probably deleted while dragging
                 */
                if (!nodeLookup.has(id)) {
                    continue;
                }
                let nextPosition = { x: x - dragItem.distance.x, y: y - dragItem.distance.y };
                if (snapToGrid) {
                    nextPosition = multiDragSnapOffset
                        ? {
                            x: Math.round(nextPosition.x + multiDragSnapOffset.x),
                            y: Math.round(nextPosition.y + multiDragSnapOffset.y),
                        }
                        : snapPosition(nextPosition, snapGrid);
                }
                let adjustedNodeExtent = null;
                if (isMultiDrag && nodeExtent && !dragItem.extent && nodesBox) {
                    const { positionAbsolute } = dragItem.internals;
                    const x1 = positionAbsolute.x - nodesBox.x + nodeExtent[0][0];
                    const x2 = positionAbsolute.x + dragItem.measured.width - nodesBox.x2 + nodeExtent[1][0];
                    const y1 = positionAbsolute.y - nodesBox.y + nodeExtent[0][1];
                    const y2 = positionAbsolute.y + dragItem.measured.height - nodesBox.y2 + nodeExtent[1][1];
                    adjustedNodeExtent = [
                        [x1, y1],
                        [x2, y2],
                    ];
                }
                const { position, positionAbsolute } = calculateNodePosition({
                    nodeId: id,
                    nextPosition,
                    nodeLookup,
                    nodeExtent: adjustedNodeExtent ? adjustedNodeExtent : nodeExtent,
                    nodeOrigin,
                    onError,
                });
                // we want to make sure that we only fire a change event when there is a change
                hasChange = hasChange || dragItem.position.x !== position.x || dragItem.position.y !== position.y;
                dragItem.position = position;
                dragItem.internals.positionAbsolute = positionAbsolute;
            }
            nodePositionsChanged = nodePositionsChanged || hasChange;
            if (!hasChange) {
                return;
            }
            updateNodePositions(dragItems, true);
            if (dragEvent && (onDrag || onNodeDrag || (!nodeId && onSelectionDrag))) {
                const [currentNode, currentNodes] = getEventHandlerParams({
                    nodeId,
                    dragItems,
                    nodeLookup,
                });
                onDrag?.(dragEvent, dragItems, currentNode, currentNodes);
                onNodeDrag?.(dragEvent, currentNode, currentNodes);
                if (!nodeId) {
                    onSelectionDrag?.(dragEvent, currentNodes);
                }
            }
        }
        async function autoPan() {
            if (!containerBounds) {
                return;
            }
            const { transform, panBy, autoPanSpeed, autoPanOnNodeDrag } = getStoreItems();
            if (!autoPanOnNodeDrag) {
                autoPanStarted = false;
                cancelAnimationFrame(autoPanId);
                return;
            }
            const [xMovement, yMovement] = calcAutoPan(mousePosition, containerBounds, autoPanSpeed);
            if (xMovement !== 0 || yMovement !== 0) {
                lastPos.x = (lastPos.x ?? 0) - xMovement / transform[2];
                lastPos.y = (lastPos.y ?? 0) - yMovement / transform[2];
                if (await panBy({ x: xMovement, y: yMovement })) {
                    updateNodes(lastPos);
                }
            }
            autoPanId = requestAnimationFrame(autoPan);
        }
        function startDrag(event) {
            const { nodeLookup, multiSelectionActive, nodesDraggable, transform, snapGrid, snapToGrid, selectNodesOnDrag, onNodeDragStart, onSelectionDragStart, unselectNodesAndEdges, } = getStoreItems();
            dragStarted = true;
            if ((!selectNodesOnDrag || !isSelectable) && !multiSelectionActive && nodeId) {
                if (!nodeLookup.get(nodeId)?.selected) {
                    // we need to reset selected nodes when selectNodesOnDrag=false
                    unselectNodesAndEdges();
                }
            }
            if (isSelectable && selectNodesOnDrag && nodeId) {
                onNodeMouseDown?.(nodeId);
            }
            const pointerPos = getPointerPosition(event.sourceEvent, { transform, snapGrid, snapToGrid, containerBounds });
            lastPos = pointerPos;
            dragItems = getDragItems(nodeLookup, nodesDraggable, pointerPos, nodeId);
            if (dragItems.size > 0 && (onDragStart || onNodeDragStart || (!nodeId && onSelectionDragStart))) {
                const [currentNode, currentNodes] = getEventHandlerParams({
                    nodeId,
                    dragItems,
                    nodeLookup,
                });
                onDragStart?.(event.sourceEvent, dragItems, currentNode, currentNodes);
                onNodeDragStart?.(event.sourceEvent, currentNode, currentNodes);
                if (!nodeId) {
                    onSelectionDragStart?.(event.sourceEvent, currentNodes);
                }
            }
        }
        const d3DragInstance = drag()
            .clickDistance(nodeClickDistance)
            .on('start', (event) => {
            const { domNode, nodeDragThreshold, transform, snapGrid, snapToGrid } = getStoreItems();
            containerBounds = domNode?.getBoundingClientRect() || null;
            abortDrag = false;
            nodePositionsChanged = false;
            dragEvent = event.sourceEvent;
            if (nodeDragThreshold === 0) {
                startDrag(event);
            }
            const pointerPos = getPointerPosition(event.sourceEvent, { transform, snapGrid, snapToGrid, containerBounds });
            lastPos = pointerPos;
            mousePosition = getEventPosition(event.sourceEvent, containerBounds);
        })
            .on('drag', (event) => {
            const { autoPanOnNodeDrag, transform, snapGrid, snapToGrid, nodeDragThreshold, nodeLookup } = getStoreItems();
            const pointerPos = getPointerPosition(event.sourceEvent, { transform, snapGrid, snapToGrid, containerBounds });
            dragEvent = event.sourceEvent;
            if ((event.sourceEvent.type === 'touchmove' && event.sourceEvent.touches.length > 1) ||
                // if user deletes a node while dragging, we need to abort the drag to prevent errors
                (nodeId && !nodeLookup.has(nodeId))) {
                abortDrag = true;
            }
            if (abortDrag) {
                return;
            }
            if (!autoPanStarted && autoPanOnNodeDrag && dragStarted) {
                autoPanStarted = true;
                autoPan();
            }
            if (!dragStarted) {
                // Calculate distance in client coordinates for consistent drag threshold behavior across zoom levels
                const currentMousePosition = getEventPosition(event.sourceEvent, containerBounds);
                const x = currentMousePosition.x - mousePosition.x;
                const y = currentMousePosition.y - mousePosition.y;
                const distance = Math.sqrt(x * x + y * y);
                if (distance > nodeDragThreshold) {
                    startDrag(event);
                }
            }
            // skip events without movement
            if ((lastPos.x !== pointerPos.xSnapped || lastPos.y !== pointerPos.ySnapped) && dragItems && dragStarted) {
                mousePosition = getEventPosition(event.sourceEvent, containerBounds);
                updateNodes(pointerPos);
            }
        })
            .on('end', (event) => {
            if (!dragStarted || abortDrag) {
                return;
            }
            autoPanStarted = false;
            dragStarted = false;
            cancelAnimationFrame(autoPanId);
            if (dragItems.size > 0) {
                const { nodeLookup, updateNodePositions, onNodeDragStop, onSelectionDragStop } = getStoreItems();
                if (nodePositionsChanged) {
                    updateNodePositions(dragItems, false);
                    nodePositionsChanged = false;
                }
                if (onDragStop || onNodeDragStop || (!nodeId && onSelectionDragStop)) {
                    const [currentNode, currentNodes] = getEventHandlerParams({
                        nodeId,
                        dragItems,
                        nodeLookup,
                        dragging: false,
                    });
                    onDragStop?.(event.sourceEvent, dragItems, currentNode, currentNodes);
                    onNodeDragStop?.(event.sourceEvent, currentNode, currentNodes);
                    if (!nodeId) {
                        onSelectionDragStop?.(event.sourceEvent, currentNodes);
                    }
                }
            }
        })
            .filter((event) => {
            const target = event.target;
            const isDraggable = !event.button &&
                (!noDragClassName || !hasSelector(target, `.${noDragClassName}`, domNode)) &&
                (!handleSelector || hasSelector(target, handleSelector, domNode));
            return isDraggable;
        });
        d3Selection.call(d3DragInstance);
    }
    function destroy() {
        d3Selection?.on('.drag', null);
    }
    return {
        update,
        destroy,
    };
}

function getNodesWithinDistance(position, nodeLookup, distance) {
    const nodes = [];
    const rect = {
        x: position.x - distance,
        y: position.y - distance,
        width: distance * 2,
        height: distance * 2,
    };
    for (const node of nodeLookup.values()) {
        if (getOverlappingArea(rect, nodeToRect(node)) > 0) {
            nodes.push(node);
        }
    }
    return nodes;
}
/*
 * this distance is used for the area around the user pointer
 * while doing a connection for finding the closest nodes
 */
const ADDITIONAL_DISTANCE = 250;
function getClosestHandle(position, connectionRadius, nodeLookup, fromHandle) {
    let closestHandles = [];
    let minDistance = Infinity;
    const closeNodes = getNodesWithinDistance(position, nodeLookup, connectionRadius + ADDITIONAL_DISTANCE);
    for (const node of closeNodes) {
        const allHandles = [...(node.internals.handleBounds?.source ?? []), ...(node.internals.handleBounds?.target ?? [])];
        for (const handle of allHandles) {
            // if the handle is the same as the fromHandle we skip it
            if (fromHandle.nodeId === handle.nodeId && fromHandle.type === handle.type && fromHandle.id === handle.id) {
                continue;
            }
            // determine absolute position of the handle
            const { x, y } = getHandlePosition(node, handle, handle.position, true);
            const distance = Math.sqrt(Math.pow(x - position.x, 2) + Math.pow(y - position.y, 2));
            if (distance > connectionRadius) {
                continue;
            }
            if (distance < minDistance) {
                closestHandles = [{ ...handle, x, y }];
                minDistance = distance;
            }
            else if (distance === minDistance) {
                // when multiple handles are on the same distance we collect all of them
                closestHandles.push({ ...handle, x, y });
            }
        }
    }
    if (!closestHandles.length) {
        return null;
    }
    // when multiple handles overlay each other we prefer the opposite handle
    if (closestHandles.length > 1) {
        const oppositeHandleType = fromHandle.type === 'source' ? 'target' : 'source';
        return closestHandles.find((handle) => handle.type === oppositeHandleType) ?? closestHandles[0];
    }
    return closestHandles[0];
}
function getHandle(nodeId, handleType, handleId, nodeLookup, connectionMode, withAbsolutePosition = false) {
    const node = nodeLookup.get(nodeId);
    if (!node) {
        return null;
    }
    const handles = connectionMode === 'strict'
        ? node.internals.handleBounds?.[handleType]
        : [...(node.internals.handleBounds?.source ?? []), ...(node.internals.handleBounds?.target ?? [])];
    const handle = (handleId ? handles?.find((h) => h.id === handleId) : handles?.[0]) ?? null;
    return handle && withAbsolutePosition
        ? { ...handle, ...getHandlePosition(node, handle, handle.position, true) }
        : handle;
}
function getHandleType(edgeUpdaterType, handleDomNode) {
    if (edgeUpdaterType) {
        return edgeUpdaterType;
    }
    else if (handleDomNode?.classList.contains('target')) {
        return 'target';
    }
    else if (handleDomNode?.classList.contains('source')) {
        return 'source';
    }
    return null;
}
function isConnectionValid(isInsideConnectionRadius, isHandleValid) {
    let isValid = null;
    if (isHandleValid) {
        isValid = true;
    }
    else if (isInsideConnectionRadius && !isHandleValid) {
        isValid = false;
    }
    return isValid;
}

const alwaysValid = () => true;
function onPointerDown(event, { connectionMode, connectionRadius, handleId, nodeId, edgeUpdaterType, isTarget, domNode, nodeLookup, lib, autoPanOnConnect, flowId, panBy, cancelConnection, onConnectStart, onConnect, onConnectEnd, isValidConnection = alwaysValid, onReconnectEnd, updateConnection, getTransform, getFromHandle, autoPanSpeed, dragThreshold = 1, handleDomNode, }) {
    // when xyflow is used inside a shadow root we can't use document
    const doc = getHostForElement(event.target);
    let autoPanId = 0;
    let closestHandle;
    const { x, y } = getEventPosition(event);
    const handleType = getHandleType(edgeUpdaterType, handleDomNode);
    const containerBounds = domNode?.getBoundingClientRect();
    let connectionStarted = false;
    if (!containerBounds || !handleType) {
        return;
    }
    const fromHandleInternal = getHandle(nodeId, handleType, handleId, nodeLookup, connectionMode);
    if (!fromHandleInternal) {
        return;
    }
    let position = getEventPosition(event, containerBounds);
    let autoPanStarted = false;
    let connection = null;
    let isValid = false;
    let resultHandleDomNode = null;
    // when the user is moving the mouse close to the edge of the canvas while connecting we move the canvas
    function autoPan() {
        if (!autoPanOnConnect || !containerBounds) {
            return;
        }
        const [x, y] = calcAutoPan(position, containerBounds, autoPanSpeed);
        panBy({ x, y });
        autoPanId = requestAnimationFrame(autoPan);
    }
    // Stays the same for all consecutive pointermove events
    const fromHandle = {
        ...fromHandleInternal,
        nodeId,
        type: handleType,
        position: fromHandleInternal.position,
    };
    const fromInternalNode = nodeLookup.get(nodeId);
    const from = getHandlePosition(fromInternalNode, fromHandle, Position.Left, true);
    let previousConnection = {
        inProgress: true,
        isValid: null,
        from,
        fromHandle,
        fromPosition: fromHandle.position,
        fromNode: fromInternalNode,
        to: position,
        toHandle: null,
        toPosition: oppositePosition[fromHandle.position],
        toNode: null,
        pointer: position,
    };
    function startConnection() {
        connectionStarted = true;
        updateConnection(previousConnection);
        onConnectStart?.(event, { nodeId, handleId, handleType });
    }
    if (dragThreshold === 0) {
        startConnection();
    }
    function onPointerMove(event) {
        if (!connectionStarted) {
            const { x: evtX, y: evtY } = getEventPosition(event);
            const dx = evtX - x;
            const dy = evtY - y;
            const nextConnectionStarted = dx * dx + dy * dy > dragThreshold * dragThreshold;
            if (!nextConnectionStarted) {
                return;
            }
            startConnection();
        }
        if (!getFromHandle() || !fromHandle) {
            onPointerUp(event);
            return;
        }
        const transform = getTransform();
        position = getEventPosition(event, containerBounds);
        closestHandle = getClosestHandle(pointToRendererPoint(position, transform, false, [1, 1]), connectionRadius, nodeLookup, fromHandle);
        if (!autoPanStarted) {
            autoPan();
            autoPanStarted = true;
        }
        const result = isValidHandle(event, {
            handle: closestHandle,
            connectionMode,
            fromNodeId: nodeId,
            fromHandleId: handleId,
            fromType: isTarget ? 'target' : 'source',
            isValidConnection,
            doc,
            lib,
            flowId,
            nodeLookup,
        });
        resultHandleDomNode = result.handleDomNode;
        connection = result.connection;
        isValid = isConnectionValid(!!closestHandle, result.isValid);
        const fromInternalNode = nodeLookup.get(nodeId);
        const from = fromInternalNode
            ? getHandlePosition(fromInternalNode, fromHandle, Position.Left, true)
            : previousConnection.from;
        const newConnection = {
            ...previousConnection,
            from,
            isValid,
            to: result.toHandle && isValid
                ? rendererPointToPoint({ x: result.toHandle.x, y: result.toHandle.y }, transform)
                : position,
            toHandle: result.toHandle,
            toPosition: isValid && result.toHandle ? result.toHandle.position : oppositePosition[fromHandle.position],
            toNode: result.toHandle ? nodeLookup.get(result.toHandle.nodeId) : null,
            pointer: position,
        };
        updateConnection(newConnection);
        previousConnection = newConnection;
    }
    function onPointerUp(event) {
        // Prevent multi-touch aborting connection
        if ('touches' in event && event.touches.length > 0) {
            return;
        }
        if (connectionStarted) {
            if ((closestHandle || resultHandleDomNode) && connection && isValid) {
                onConnect?.(connection);
            }
            /*
             * it's important to get a fresh reference from the store here
             * in order to get the latest state of onConnectEnd
             */
            // eslint-disable-next-line @typescript-eslint/no-unused-vars
            const { inProgress, ...connectionState } = previousConnection;
            const finalConnectionState = {
                ...connectionState,
                toPosition: previousConnection.toHandle ? previousConnection.toPosition : null,
            };
            onConnectEnd?.(event, finalConnectionState);
            if (edgeUpdaterType) {
                onReconnectEnd?.(event, finalConnectionState);
            }
        }
        cancelConnection();
        cancelAnimationFrame(autoPanId);
        autoPanStarted = false;
        isValid = false;
        connection = null;
        resultHandleDomNode = null;
        doc.removeEventListener('mousemove', onPointerMove);
        doc.removeEventListener('mouseup', onPointerUp);
        doc.removeEventListener('touchmove', onPointerMove);
        doc.removeEventListener('touchend', onPointerUp);
    }
    doc.addEventListener('mousemove', onPointerMove);
    doc.addEventListener('mouseup', onPointerUp);
    doc.addEventListener('touchmove', onPointerMove);
    doc.addEventListener('touchend', onPointerUp);
}
// checks if  and returns connection in form of an object { source: 123, target: 312 }
function isValidHandle(event, { handle, connectionMode, fromNodeId, fromHandleId, fromType, doc, lib, flowId, isValidConnection = alwaysValid, nodeLookup, }) {
    const isTarget = fromType === 'target';
    const handleDomNode = handle
        ? doc.querySelector(`.${lib}-flow__handle[data-id="${flowId}-${handle?.nodeId}-${handle?.id}-${handle?.type}"]`)
        : null;
    const { x, y } = getEventPosition(event);
    const handleBelow = doc.elementFromPoint(x, y);
    /*
     * we always want to prioritize the handle below the mouse cursor over the closest distance handle,
     * because it could be that the center of another handle is closer to the mouse pointer than the handle below the cursor
     */
    const handleToCheck = handleBelow?.classList.contains(`${lib}-flow__handle`) ? handleBelow : handleDomNode;
    const result = {
        handleDomNode: handleToCheck,
        isValid: false,
        connection: null,
        toHandle: null,
    };
    if (handleToCheck) {
        const handleType = getHandleType(undefined, handleToCheck);
        const handleNodeId = handleToCheck.getAttribute('data-nodeid');
        const handleId = handleToCheck.getAttribute('data-handleid');
        const connectable = handleToCheck.classList.contains('connectable');
        const connectableEnd = handleToCheck.classList.contains('connectableend');
        if (!handleNodeId || !handleType) {
            return result;
        }
        const connection = {
            source: isTarget ? handleNodeId : fromNodeId,
            sourceHandle: isTarget ? handleId : fromHandleId,
            target: isTarget ? fromNodeId : handleNodeId,
            targetHandle: isTarget ? fromHandleId : handleId,
        };
        result.connection = connection;
        const isConnectable = connectable && connectableEnd;
        // in strict mode we don't allow target to target or source to source connections
        const isValid = isConnectable &&
            (connectionMode === ConnectionMode.Strict
                ? (isTarget && handleType === 'source') || (!isTarget && handleType === 'target')
                : handleNodeId !== fromNodeId || handleId !== fromHandleId);
        result.isValid = isValid && isValidConnection(connection);
        result.toHandle = getHandle(handleNodeId, handleType, handleId, nodeLookup, connectionMode, true);
    }
    return result;
}
const XYHandle = {
    onPointerDown,
    isValid: isValidHandle,
};

function XYMinimap({ domNode, panZoom, getTransform, getViewScale }) {
    const selection = select(domNode);
    function update({ translateExtent, width, height, zoomStep = 1, pannable = true, zoomable = true, inversePan = false, }) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const zoomHandler = (event) => {
            if (event.sourceEvent.type !== 'wheel' || !panZoom) {
                return;
            }
            const transform = getTransform();
            const factor = event.sourceEvent.ctrlKey && isMacOs() ? 10 : 1;
            const pinchDelta = -event.sourceEvent.deltaY *
                (event.sourceEvent.deltaMode === 1 ? 0.05 : event.sourceEvent.deltaMode ? 1 : 0.002) *
                zoomStep;
            const nextZoom = transform[2] * Math.pow(2, pinchDelta * factor);
            panZoom.scaleTo(nextZoom);
        };
        let panStart = [0, 0];
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const panStartHandler = (event) => {
            if (event.sourceEvent.type === 'mousedown' || event.sourceEvent.type === 'touchstart') {
                panStart = [
                    event.sourceEvent.clientX ?? event.sourceEvent.touches[0].clientX,
                    event.sourceEvent.clientY ?? event.sourceEvent.touches[0].clientY,
                ];
            }
        };
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const panHandler = (event) => {
            const transform = getTransform();
            if ((event.sourceEvent.type !== 'mousemove' && event.sourceEvent.type !== 'touchmove') || !panZoom) {
                return;
            }
            const panCurrent = [
                event.sourceEvent.clientX ?? event.sourceEvent.touches[0].clientX,
                event.sourceEvent.clientY ?? event.sourceEvent.touches[0].clientY,
            ];
            const panDelta = [panCurrent[0] - panStart[0], panCurrent[1] - panStart[1]];
            panStart = panCurrent;
            const moveScale = getViewScale() * Math.max(transform[2], Math.log(transform[2])) * (inversePan ? -1 : 1);
            const position = {
                x: transform[0] - panDelta[0] * moveScale,
                y: transform[1] - panDelta[1] * moveScale,
            };
            const extent = [
                [0, 0],
                [width, height],
            ];
            panZoom.setViewportConstrained({
                x: position.x,
                y: position.y,
                zoom: transform[2],
            }, extent, translateExtent);
        };
        const zoomAndPanHandler = zoom()
            .on('start', panStartHandler)
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            .on('zoom', pannable ? panHandler : null)
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore
            .on('zoom.wheel', zoomable ? zoomHandler : null);
        selection.call(zoomAndPanHandler, {});
    }
    function destroy() {
        selection.on('zoom', null);
    }
    return {
        update,
        destroy,
        pointer,
    };
}

/* eslint-disable @typescript-eslint/no-explicit-any */
const transformToViewport = (transform) => ({
    x: transform.x,
    y: transform.y,
    zoom: transform.k,
});
const viewportToTransform = ({ x, y, zoom }) => identity$2.translate(x, y).scale(zoom);
const isWrappedWithClass = (event, className) => event.target.closest(`.${className}`);
const isRightClickPan = (panOnDrag, usedButton) => usedButton === 2 && Array.isArray(panOnDrag) && panOnDrag.includes(2);
// taken from d3-ease: https://github.com/d3/d3-ease/blob/main/src/cubic.js
const defaultEase = (t) => ((t *= 2) <= 1 ? t * t * t : (t -= 2) * t * t + 2) / 2;
const getD3Transition = (selection, duration = 0, ease = defaultEase, onEnd = () => { }) => {
    const hasDuration = typeof duration === 'number' && duration > 0;
    if (!hasDuration) {
        onEnd();
    }
    return hasDuration ? selection.transition().duration(duration).ease(ease).on('end', onEnd) : selection;
};
const wheelDelta = (event) => {
    const factor = event.ctrlKey && isMacOs() ? 10 : 1;
    return -event.deltaY * (event.deltaMode === 1 ? 0.05 : event.deltaMode ? 1 : 0.002) * factor;
};

function createPanOnScrollHandler({ zoomPanValues, noWheelClassName, d3Selection, d3Zoom, panOnScrollMode, panOnScrollSpeed, zoomOnPinch, onPanZoomStart, onPanZoom, onPanZoomEnd, }) {
    return (event) => {
        if (isWrappedWithClass(event, noWheelClassName)) {
            if (event.ctrlKey) {
                event.preventDefault(); // stop native page zoom for pinch zooming
            }
            return false;
        }
        event.preventDefault();
        event.stopImmediatePropagation();
        const currentZoom = d3Selection.property('__zoom').k || 1;
        // macos sets ctrlKey=true for pinch gesture on a trackpad
        if (event.ctrlKey && zoomOnPinch) {
            const point = pointer(event);
            const pinchDelta = wheelDelta(event);
            const zoom = currentZoom * Math.pow(2, pinchDelta);
            // @ts-ignore
            d3Zoom.scaleTo(d3Selection, zoom, point, event);
            return;
        }
        /*
         * increase scroll speed in firefox
         * firefox: deltaMode === 1; chrome: deltaMode === 0
         */
        const deltaNormalize = event.deltaMode === 1 ? 20 : 1;
        let deltaX = panOnScrollMode === PanOnScrollMode.Vertical ? 0 : event.deltaX * deltaNormalize;
        let deltaY = panOnScrollMode === PanOnScrollMode.Horizontal ? 0 : event.deltaY * deltaNormalize;
        // this enables vertical scrolling with shift + scroll on windows
        if (!isMacOs() && event.shiftKey && panOnScrollMode !== PanOnScrollMode.Vertical) {
            deltaX = event.deltaY * deltaNormalize;
            deltaY = 0;
        }
        d3Zoom.translateBy(d3Selection, -(deltaX / currentZoom) * panOnScrollSpeed, -(deltaY / currentZoom) * panOnScrollSpeed, 
        // @ts-ignore
        { internal: true });
        const nextViewport = transformToViewport(d3Selection.property('__zoom'));
        clearTimeout(zoomPanValues.panScrollTimeout);
        /*
         * for pan on scroll we need to handle the event calls on our own
         * we can't use the start, zoom and end events from d3-zoom
         * because start and move gets called on every scroll event and not once at the beginning
         */
        if (!zoomPanValues.isPanScrolling) {
            zoomPanValues.isPanScrolling = true;
            onPanZoomStart?.(event, nextViewport);
        }
        else {
            onPanZoom?.(event, nextViewport);
            zoomPanValues.panScrollTimeout = setTimeout(() => {
                onPanZoomEnd?.(event, nextViewport);
                zoomPanValues.isPanScrolling = false;
            }, 150);
        }
    };
}
function createZoomOnScrollHandler({ noWheelClassName, preventScrolling, d3ZoomHandler }) {
    return function (event, d) {
        const isWheel = event.type === 'wheel';
        // we still want to enable pinch zooming even if preventScrolling is set to false
        const preventZoom = !preventScrolling && isWheel && !event.ctrlKey;
        const hasNoWheelClass = isWrappedWithClass(event, noWheelClassName);
        // if user is pinch zooming above a nowheel element, we don't want the browser to zoom
        if (event.ctrlKey && isWheel && hasNoWheelClass) {
            event.preventDefault();
        }
        if (preventZoom || hasNoWheelClass) {
            return null;
        }
        event.preventDefault();
        d3ZoomHandler.call(this, event, d);
    };
}
function createPanZoomStartHandler({ zoomPanValues, onDraggingChange, onPanZoomStart }) {
    return (event) => {
        if (event.sourceEvent?.internal) {
            return;
        }
        const viewport = transformToViewport(event.transform);
        // we need to remember it here, because it's always 0 in the "zoom" event
        zoomPanValues.mouseButton = event.sourceEvent?.button || 0;
        zoomPanValues.isZoomingOrPanning = true;
        zoomPanValues.prevViewport = viewport;
        if (event.sourceEvent?.type === 'mousedown') {
            onDraggingChange(true);
        }
        if (onPanZoomStart) {
            onPanZoomStart?.(event.sourceEvent, viewport);
        }
    };
}
function createPanZoomHandler({ zoomPanValues, panOnDrag, onPaneContextMenu, onTransformChange, onPanZoom, }) {
    return (event) => {
        zoomPanValues.usedRightMouseButton = !!(onPaneContextMenu && isRightClickPan(panOnDrag, zoomPanValues.mouseButton ?? 0));
        if (!event.sourceEvent?.sync) {
            onTransformChange([event.transform.x, event.transform.y, event.transform.k]);
        }
        if (onPanZoom && !event.sourceEvent?.internal) {
            onPanZoom?.(event.sourceEvent, transformToViewport(event.transform));
        }
    };
}
function createPanZoomEndHandler({ zoomPanValues, panOnDrag, panOnScroll, onDraggingChange, onPanZoomEnd, onPaneContextMenu, }) {
    return (event) => {
        if (event.sourceEvent?.internal) {
            return;
        }
        zoomPanValues.isZoomingOrPanning = false;
        if (onPaneContextMenu &&
            isRightClickPan(panOnDrag, zoomPanValues.mouseButton ?? 0) &&
            !zoomPanValues.usedRightMouseButton &&
            event.sourceEvent) {
            onPaneContextMenu(event.sourceEvent);
        }
        zoomPanValues.usedRightMouseButton = false;
        onDraggingChange(false);
        if (onPanZoomEnd) {
            const viewport = transformToViewport(event.transform);
            zoomPanValues.prevViewport = viewport;
            clearTimeout(zoomPanValues.timerId);
            zoomPanValues.timerId = setTimeout(() => {
                onPanZoomEnd?.(event.sourceEvent, viewport);
            }, 
            // we need a setTimeout for panOnScroll to suppress multiple end events fired during scroll
            panOnScroll ? 150 : 0);
        }
    };
}

/* eslint-disable @typescript-eslint/no-explicit-any */
function createFilter({ zoomActivationKeyPressed, zoomOnScroll, zoomOnPinch, panOnDrag, panOnScroll, zoomOnDoubleClick, userSelectionActive, noWheelClassName, noPanClassName, lib, connectionInProgress, }) {
    return (event) => {
        const zoomScroll = zoomActivationKeyPressed || zoomOnScroll;
        const pinchZoom = zoomOnPinch && event.ctrlKey;
        const isWheelEvent = event.type === 'wheel';
        if (event.button === 1 &&
            event.type === 'mousedown' &&
            (isWrappedWithClass(event, `${lib}-flow__node`) || isWrappedWithClass(event, `${lib}-flow__edge`))) {
            return true;
        }
        // if all interactions are disabled, we prevent all zoom events
        if (!panOnDrag && !zoomScroll && !panOnScroll && !zoomOnDoubleClick && !zoomOnPinch) {
            return false;
        }
        // during a selection we prevent all other interactions
        if (userSelectionActive) {
            return false;
        }
        // we want to disable pinch-zooming while making a connection
        if (connectionInProgress && !isWheelEvent) {
            return false;
        }
        // if the target element is inside an element with the nowheel class, we prevent zooming
        if (isWrappedWithClass(event, noWheelClassName) && isWheelEvent) {
            return false;
        }
        // if the target element is inside an element with the nopan class, we prevent panning
        if (isWrappedWithClass(event, noPanClassName) &&
            (!isWheelEvent || (panOnScroll && isWheelEvent && !zoomActivationKeyPressed))) {
            return false;
        }
        if (!zoomOnPinch && event.ctrlKey && isWheelEvent) {
            return false;
        }
        if (!zoomOnPinch && event.type === 'touchstart' && event.touches?.length > 1) {
            event.preventDefault(); // if you manage to start with 2 touches, we prevent native zoom
            return false;
        }
        // when there is no scroll handling enabled, we prevent all wheel events
        if (!zoomScroll && !panOnScroll && !pinchZoom && isWheelEvent) {
            return false;
        }
        // if the pane is not movable, we prevent dragging it with mousestart or touchstart
        if (!panOnDrag && (event.type === 'mousedown' || event.type === 'touchstart')) {
            return false;
        }
        // if the pane is only movable using allowed clicks
        if (Array.isArray(panOnDrag) && !panOnDrag.includes(event.button) && event.type === 'mousedown') {
            return false;
        }
        // We only allow right clicks if pan on drag is set to right click
        const buttonAllowed = (Array.isArray(panOnDrag) && panOnDrag.includes(event.button)) || !event.button || event.button <= 1;
        // default filter for d3-zoom
        return (!event.ctrlKey || isWheelEvent) && buttonAllowed;
    };
}

function XYPanZoom({ domNode, minZoom, maxZoom, translateExtent, viewport, onPanZoom, onPanZoomStart, onPanZoomEnd, onDraggingChange, }) {
    const zoomPanValues = {
        isZoomingOrPanning: false,
        usedRightMouseButton: false,
        prevViewport: { },
        mouseButton: 0,
        timerId: undefined,
        panScrollTimeout: undefined,
        isPanScrolling: false,
    };
    const bbox = domNode.getBoundingClientRect();
    const d3ZoomInstance = zoom().scaleExtent([minZoom, maxZoom]).translateExtent(translateExtent);
    const d3Selection = select(domNode).call(d3ZoomInstance);
    setViewportConstrained({
        x: viewport.x,
        y: viewport.y,
        zoom: clamp(viewport.zoom, minZoom, maxZoom),
    }, [
        [0, 0],
        [bbox.width, bbox.height],
    ], translateExtent);
    const d3ZoomHandler = d3Selection.on('wheel.zoom');
    const d3DblClickZoomHandler = d3Selection.on('dblclick.zoom');
    d3ZoomInstance.wheelDelta(wheelDelta);
    function setTransform(transform, options) {
        if (d3Selection) {
            return new Promise((resolve) => {
                d3ZoomInstance?.interpolate(options?.interpolate === 'linear' ? interpolate$1 : interpolateZoom).transform(getD3Transition(d3Selection, options?.duration, options?.ease, () => resolve(true)), transform);
            });
        }
        return Promise.resolve(false);
    }
    // public functions
    function update({ noWheelClassName, noPanClassName, onPaneContextMenu, userSelectionActive, panOnScroll, panOnDrag, panOnScrollMode, panOnScrollSpeed, preventScrolling, zoomOnPinch, zoomOnScroll, zoomOnDoubleClick, zoomActivationKeyPressed, lib, onTransformChange, connectionInProgress, paneClickDistance, selectionOnDrag, }) {
        if (userSelectionActive && !zoomPanValues.isZoomingOrPanning) {
            destroy();
        }
        const isPanOnScroll = panOnScroll && !zoomActivationKeyPressed && !userSelectionActive;
        d3ZoomInstance.clickDistance(selectionOnDrag ? Infinity : !isNumeric(paneClickDistance) || paneClickDistance < 0 ? 0 : paneClickDistance);
        const wheelHandler = isPanOnScroll
            ? createPanOnScrollHandler({
                zoomPanValues,
                noWheelClassName,
                d3Selection,
                d3Zoom: d3ZoomInstance,
                panOnScrollMode,
                panOnScrollSpeed,
                zoomOnPinch,
                onPanZoomStart,
                onPanZoom,
                onPanZoomEnd,
            })
            : createZoomOnScrollHandler({
                noWheelClassName,
                preventScrolling,
                d3ZoomHandler,
            });
        d3Selection.on('wheel.zoom', wheelHandler, { passive: false });
        if (!userSelectionActive) {
            // pan zoom start
            const startHandler = createPanZoomStartHandler({
                zoomPanValues,
                onDraggingChange,
                onPanZoomStart,
            });
            d3ZoomInstance.on('start', startHandler);
            // pan zoom
            const panZoomHandler = createPanZoomHandler({
                zoomPanValues,
                panOnDrag,
                onPaneContextMenu: !!onPaneContextMenu,
                onPanZoom,
                onTransformChange,
            });
            d3ZoomInstance.on('zoom', panZoomHandler);
            // pan zoom end
            const panZoomEndHandler = createPanZoomEndHandler({
                zoomPanValues,
                panOnDrag,
                panOnScroll,
                onPaneContextMenu,
                onPanZoomEnd,
                onDraggingChange,
            });
            d3ZoomInstance.on('end', panZoomEndHandler);
        }
        const filter = createFilter({
            zoomActivationKeyPressed,
            panOnDrag,
            zoomOnScroll,
            panOnScroll,
            zoomOnDoubleClick,
            zoomOnPinch,
            userSelectionActive,
            noPanClassName,
            noWheelClassName,
            lib,
            connectionInProgress,
        });
        d3ZoomInstance.filter(filter);
        /*
         * We cannot add zoomOnDoubleClick to the filter above because
         * double tapping on touch screens circumvents the filter and
         * dblclick.zoom is fired on the selection directly
         */
        if (zoomOnDoubleClick) {
            d3Selection.on('dblclick.zoom', d3DblClickZoomHandler);
        }
        else {
            d3Selection.on('dblclick.zoom', null);
        }
    }
    function destroy() {
        d3ZoomInstance.on('zoom', null);
    }
    async function setViewportConstrained(viewport, extent, translateExtent) {
        const nextTransform = viewportToTransform(viewport);
        const contrainedTransform = d3ZoomInstance?.constrain()(nextTransform, extent, translateExtent);
        if (contrainedTransform) {
            await setTransform(contrainedTransform);
        }
        return new Promise((resolve) => resolve(contrainedTransform));
    }
    async function setViewport(viewport, options) {
        const nextTransform = viewportToTransform(viewport);
        await setTransform(nextTransform, options);
        return new Promise((resolve) => resolve(nextTransform));
    }
    function syncViewport(viewport) {
        if (d3Selection) {
            const nextTransform = viewportToTransform(viewport);
            const currentTransform = d3Selection.property('__zoom');
            if (currentTransform.k !== viewport.zoom ||
                currentTransform.x !== viewport.x ||
                currentTransform.y !== viewport.y) {
                // eslint-disable-next-line @typescript-eslint/ban-ts-comment
                // @ts-ignore
                d3ZoomInstance?.transform(d3Selection, nextTransform, null, { sync: true });
            }
        }
    }
    function getViewport() {
        const transform$1 = d3Selection ? transform(d3Selection.node()) : { x: 0, y: 0, k: 1 };
        return { x: transform$1.x, y: transform$1.y, zoom: transform$1.k };
    }
    function scaleTo(zoom, options) {
        if (d3Selection) {
            return new Promise((resolve) => {
                d3ZoomInstance?.interpolate(options?.interpolate === 'linear' ? interpolate$1 : interpolateZoom).scaleTo(getD3Transition(d3Selection, options?.duration, options?.ease, () => resolve(true)), zoom);
            });
        }
        return Promise.resolve(false);
    }
    function scaleBy(factor, options) {
        if (d3Selection) {
            return new Promise((resolve) => {
                d3ZoomInstance?.interpolate(options?.interpolate === 'linear' ? interpolate$1 : interpolateZoom).scaleBy(getD3Transition(d3Selection, options?.duration, options?.ease, () => resolve(true)), factor);
            });
        }
        return Promise.resolve(false);
    }
    function setScaleExtent(scaleExtent) {
        d3ZoomInstance?.scaleExtent(scaleExtent);
    }
    function setTranslateExtent(translateExtent) {
        d3ZoomInstance?.translateExtent(translateExtent);
    }
    function setClickDistance(distance) {
        const validDistance = !isNumeric(distance) || distance < 0 ? 0 : distance;
        d3ZoomInstance?.clickDistance(validDistance);
    }
    return {
        update,
        destroy,
        setViewport,
        setViewportConstrained,
        getViewport,
        scaleTo,
        scaleBy,
        setScaleExtent,
        setTranslateExtent,
        syncViewport,
        setClickDistance,
    };
}

/**
 * Used to determine the variant of the resize control
 *
 * @public
 */
var ResizeControlVariant;
(function (ResizeControlVariant) {
    ResizeControlVariant["Line"] = "line";
    ResizeControlVariant["Handle"] = "handle";
})(ResizeControlVariant || (ResizeControlVariant = {}));

/**
 * Get all connecting edges for a given set of nodes
 * @param width - new width of the node
 * @param prevWidth - previous width of the node
 * @param height - new height of the node
 * @param prevHeight - previous height of the node
 * @param affectsX - whether to invert the resize direction for the x axis
 * @param affectsY - whether to invert the resize direction for the y axis
 * @returns array of two numbers representing the direction of the resize for each axis, 0 = no change, 1 = increase, -1 = decrease
 */
function getResizeDirection({ width, prevWidth, height, prevHeight, affectsX, affectsY, }) {
    const deltaWidth = width - prevWidth;
    const deltaHeight = height - prevHeight;
    const direction = [deltaWidth > 0 ? 1 : deltaWidth < 0 ? -1 : 0, deltaHeight > 0 ? 1 : deltaHeight < 0 ? -1 : 0];
    if (deltaWidth && affectsX) {
        direction[0] = direction[0] * -1;
    }
    if (deltaHeight && affectsY) {
        direction[1] = direction[1] * -1;
    }
    return direction;
}
/**
 * Parses the control position that is being dragged to dimensions that are being resized
 * @param controlPosition - position of the control that is being dragged
 * @returns isHorizontal, isVertical, affectsX, affectsY,
 */
function getControlDirection(controlPosition) {
    const isHorizontal = controlPosition.includes('right') || controlPosition.includes('left');
    const isVertical = controlPosition.includes('bottom') || controlPosition.includes('top');
    const affectsX = controlPosition.includes('left');
    const affectsY = controlPosition.includes('top');
    return {
        isHorizontal,
        isVertical,
        affectsX,
        affectsY,
    };
}
function getLowerExtentClamp(lowerExtent, lowerBound) {
    return Math.max(0, lowerBound - lowerExtent);
}
function getUpperExtentClamp(upperExtent, upperBound) {
    return Math.max(0, upperExtent - upperBound);
}
function getSizeClamp(size, minSize, maxSize) {
    return Math.max(0, minSize - size, size - maxSize);
}
function xor(a, b) {
    return a ? !b : b;
}
/**
 * Calculates new width & height and x & y of node after resize based on pointer position
 * @description - Buckle up, this is a chunky one... If you want to determine the new dimensions of a node after a resize,
 * you have to account for all possible restrictions: min/max width/height of the node, the maximum extent the node is allowed
 * to move in (in this case: resize into) determined by the parent node, the minimal extent determined by child nodes
 * with expandParent or extent: 'parent' set and oh yeah, these things also have to work with keepAspectRatio!
 * The way this is done is by determining how much each of these restricting actually restricts the resize and then applying the
 * strongest restriction. Because the resize affects x, y and width, height and width, height of a opposing side with keepAspectRatio,
 * the resize amount is always kept in distX & distY amount (the distance in mouse movement)
 * Instead of clamping each value, we first calculate the biggest 'clamp' (for the lack of a better name) and then apply it to all values.
 * To complicate things nodeOrigin has to be taken into account as well. This is done by offsetting the nodes as if their origin is [0, 0],
 * then calculating the restrictions as usual
 * @param startValues - starting values of resize
 * @param controlDirection - dimensions affected by the resize
 * @param pointerPosition - the current pointer position corrected for snapping
 * @param boundaries - minimum and maximum dimensions of the node
 * @param keepAspectRatio - prevent changes of asprect ratio
 * @returns x, y, width and height of the node after resize
 */
function getDimensionsAfterResize(startValues, controlDirection, pointerPosition, boundaries, keepAspectRatio, nodeOrigin, extent, childExtent) {
    let { affectsX, affectsY } = controlDirection;
    const { isHorizontal, isVertical } = controlDirection;
    const isDiagonal = isHorizontal && isVertical;
    const { xSnapped, ySnapped } = pointerPosition;
    const { minWidth, maxWidth, minHeight, maxHeight } = boundaries;
    const { x: startX, y: startY, width: startWidth, height: startHeight, aspectRatio } = startValues;
    let distX = Math.floor(isHorizontal ? xSnapped - startValues.pointerX : 0);
    let distY = Math.floor(isVertical ? ySnapped - startValues.pointerY : 0);
    const newWidth = startWidth + (affectsX ? -distX : distX);
    const newHeight = startHeight + (affectsY ? -distY : distY);
    const originOffsetX = -nodeOrigin[0] * startWidth;
    const originOffsetY = -nodeOrigin[1] * startHeight;
    // Check if maxWidth, minWWidth, maxHeight, minHeight are restricting the resize
    let clampX = getSizeClamp(newWidth, minWidth, maxWidth);
    let clampY = getSizeClamp(newHeight, minHeight, maxHeight);
    // Check if extent is restricting the resize
    if (extent) {
        let xExtentClamp = 0;
        let yExtentClamp = 0;
        if (affectsX && distX < 0) {
            xExtentClamp = getLowerExtentClamp(startX + distX + originOffsetX, extent[0][0]);
        }
        else if (!affectsX && distX > 0) {
            xExtentClamp = getUpperExtentClamp(startX + newWidth + originOffsetX, extent[1][0]);
        }
        if (affectsY && distY < 0) {
            yExtentClamp = getLowerExtentClamp(startY + distY + originOffsetY, extent[0][1]);
        }
        else if (!affectsY && distY > 0) {
            yExtentClamp = getUpperExtentClamp(startY + newHeight + originOffsetY, extent[1][1]);
        }
        clampX = Math.max(clampX, xExtentClamp);
        clampY = Math.max(clampY, yExtentClamp);
    }
    // Check if the child extent is restricting the resize
    if (childExtent) {
        let xExtentClamp = 0;
        let yExtentClamp = 0;
        if (affectsX && distX > 0) {
            xExtentClamp = getUpperExtentClamp(startX + distX, childExtent[0][0]);
        }
        else if (!affectsX && distX < 0) {
            xExtentClamp = getLowerExtentClamp(startX + newWidth, childExtent[1][0]);
        }
        if (affectsY && distY > 0) {
            yExtentClamp = getUpperExtentClamp(startY + distY, childExtent[0][1]);
        }
        else if (!affectsY && distY < 0) {
            yExtentClamp = getLowerExtentClamp(startY + newHeight, childExtent[1][1]);
        }
        clampX = Math.max(clampX, xExtentClamp);
        clampY = Math.max(clampY, yExtentClamp);
    }
    // Check if the aspect ratio resizing of the other side is restricting the resize
    if (keepAspectRatio) {
        if (isHorizontal) {
            // Check if the max dimensions might be restricting the resize
            const aspectHeightClamp = getSizeClamp(newWidth / aspectRatio, minHeight, maxHeight) * aspectRatio;
            clampX = Math.max(clampX, aspectHeightClamp);
            // Check if the extent is restricting the resize
            if (extent) {
                let aspectExtentClamp = 0;
                if ((!affectsX && !affectsY) || (affectsX && !affectsY && isDiagonal)) {
                    aspectExtentClamp =
                        getUpperExtentClamp(startY + originOffsetY + newWidth / aspectRatio, extent[1][1]) * aspectRatio;
                }
                else {
                    aspectExtentClamp =
                        getLowerExtentClamp(startY + originOffsetY + (affectsX ? distX : -distX) / aspectRatio, extent[0][1]) *
                            aspectRatio;
                }
                clampX = Math.max(clampX, aspectExtentClamp);
            }
            // Check if the child extent is restricting the resize
            if (childExtent) {
                let aspectExtentClamp = 0;
                if ((!affectsX && !affectsY) || (affectsX && !affectsY && isDiagonal)) {
                    aspectExtentClamp = getLowerExtentClamp(startY + newWidth / aspectRatio, childExtent[1][1]) * aspectRatio;
                }
                else {
                    aspectExtentClamp =
                        getUpperExtentClamp(startY + (affectsX ? distX : -distX) / aspectRatio, childExtent[0][1]) * aspectRatio;
                }
                clampX = Math.max(clampX, aspectExtentClamp);
            }
        }
        // Do the same thing for vertical resizing
        if (isVertical) {
            const aspectWidthClamp = getSizeClamp(newHeight * aspectRatio, minWidth, maxWidth) / aspectRatio;
            clampY = Math.max(clampY, aspectWidthClamp);
            if (extent) {
                let aspectExtentClamp = 0;
                if ((!affectsX && !affectsY) || (affectsY && !affectsX && isDiagonal)) {
                    aspectExtentClamp =
                        getUpperExtentClamp(startX + newHeight * aspectRatio + originOffsetX, extent[1][0]) / aspectRatio;
                }
                else {
                    aspectExtentClamp =
                        getLowerExtentClamp(startX + (affectsY ? distY : -distY) * aspectRatio + originOffsetX, extent[0][0]) /
                            aspectRatio;
                }
                clampY = Math.max(clampY, aspectExtentClamp);
            }
            if (childExtent) {
                let aspectExtentClamp = 0;
                if ((!affectsX && !affectsY) || (affectsY && !affectsX && isDiagonal)) {
                    aspectExtentClamp = getLowerExtentClamp(startX + newHeight * aspectRatio, childExtent[1][0]) / aspectRatio;
                }
                else {
                    aspectExtentClamp =
                        getUpperExtentClamp(startX + (affectsY ? distY : -distY) * aspectRatio, childExtent[0][0]) / aspectRatio;
                }
                clampY = Math.max(clampY, aspectExtentClamp);
            }
        }
    }
    distY = distY + (distY < 0 ? clampY : -clampY);
    distX = distX + (distX < 0 ? clampX : -clampX);
    if (keepAspectRatio) {
        if (isDiagonal) {
            if (newWidth > newHeight * aspectRatio) {
                distY = (xor(affectsX, affectsY) ? -distX : distX) / aspectRatio;
            }
            else {
                distX = (xor(affectsX, affectsY) ? -distY : distY) * aspectRatio;
            }
        }
        else {
            if (isHorizontal) {
                distY = distX / aspectRatio;
                affectsY = affectsX;
            }
            else {
                distX = distY * aspectRatio;
                affectsX = affectsY;
            }
        }
    }
    const x = affectsX ? startX + distX : startX;
    const y = affectsY ? startY + distY : startY;
    return {
        width: startWidth + (affectsX ? -distX : distX),
        height: startHeight + (affectsY ? -distY : distY),
        x: nodeOrigin[0] * distX * (!affectsX ? 1 : -1) + x,
        y: nodeOrigin[1] * distY * (!affectsY ? 1 : -1) + y,
    };
}

const initPrevValues$1 = { width: 0, height: 0, x: 0, y: 0 };
const initStartValues = {
    ...initPrevValues$1,
    pointerX: 0,
    pointerY: 0,
    aspectRatio: 1,
};
function nodeToParentExtent(node) {
    return [
        [0, 0],
        [node.measured.width, node.measured.height],
    ];
}
function nodeToChildExtent(child, parent, nodeOrigin) {
    const x = parent.position.x + child.position.x;
    const y = parent.position.y + child.position.y;
    const width = child.measured.width ?? 0;
    const height = child.measured.height ?? 0;
    const originOffsetX = nodeOrigin[0] * width;
    const originOffsetY = nodeOrigin[1] * height;
    return [
        [x - originOffsetX, y - originOffsetY],
        [x + width - originOffsetX, y + height - originOffsetY],
    ];
}
function XYResizer({ domNode, nodeId, getStoreItems, onChange, onEnd }) {
    const selection = select(domNode);
    let params = {
        controlDirection: getControlDirection('bottom-right'),
        boundaries: {
            minWidth: 0,
            minHeight: 0,
            maxWidth: Number.MAX_VALUE,
            maxHeight: Number.MAX_VALUE,
        },
        resizeDirection: undefined,
        keepAspectRatio: false,
    };
    function update({ controlPosition, boundaries, keepAspectRatio, resizeDirection, onResizeStart, onResize, onResizeEnd, shouldResize, }) {
        let prevValues = { ...initPrevValues$1 };
        let startValues = { ...initStartValues };
        params = {
            boundaries,
            resizeDirection,
            keepAspectRatio,
            controlDirection: getControlDirection(controlPosition),
        };
        let node = undefined;
        let containerBounds = null;
        let childNodes = [];
        let parentNode = undefined; // Needed to fix expandParent
        let parentExtent = undefined;
        let childExtent = undefined;
        // we only want to trigger onResizeEnd if onResize was actually called
        let resizeDetected = false;
        const dragHandler = drag()
            .on('start', (event) => {
            const { nodeLookup, transform, snapGrid, snapToGrid, nodeOrigin, paneDomNode } = getStoreItems();
            node = nodeLookup.get(nodeId);
            if (!node) {
                return;
            }
            containerBounds = paneDomNode?.getBoundingClientRect() ?? null;
            const { xSnapped, ySnapped } = getPointerPosition(event.sourceEvent, {
                transform,
                snapGrid,
                snapToGrid,
                containerBounds,
            });
            prevValues = {
                width: node.measured.width ?? 0,
                height: node.measured.height ?? 0,
                x: node.position.x ?? 0,
                y: node.position.y ?? 0,
            };
            startValues = {
                ...prevValues,
                pointerX: xSnapped,
                pointerY: ySnapped,
                aspectRatio: prevValues.width / prevValues.height,
            };
            parentNode = undefined;
            if (node.parentId && (node.extent === 'parent' || node.expandParent)) {
                parentNode = nodeLookup.get(node.parentId);
                parentExtent = parentNode && node.extent === 'parent' ? nodeToParentExtent(parentNode) : undefined;
            }
            /*
             * Collect all child nodes to correct their relative positions when top/left changes
             * Determine largest minimal extent the parent node is allowed to resize to
             */
            childNodes = [];
            childExtent = undefined;
            for (const [childId, child] of nodeLookup) {
                if (child.parentId === nodeId) {
                    childNodes.push({
                        id: childId,
                        position: { ...child.position },
                        extent: child.extent,
                    });
                    if (child.extent === 'parent' || child.expandParent) {
                        const extent = nodeToChildExtent(child, node, child.origin ?? nodeOrigin);
                        if (childExtent) {
                            childExtent = [
                                [Math.min(extent[0][0], childExtent[0][0]), Math.min(extent[0][1], childExtent[0][1])],
                                [Math.max(extent[1][0], childExtent[1][0]), Math.max(extent[1][1], childExtent[1][1])],
                            ];
                        }
                        else {
                            childExtent = extent;
                        }
                    }
                }
            }
            onResizeStart?.(event, { ...prevValues });
        })
            .on('drag', (event) => {
            const { transform, snapGrid, snapToGrid, nodeOrigin: storeNodeOrigin } = getStoreItems();
            const pointerPosition = getPointerPosition(event.sourceEvent, {
                transform,
                snapGrid,
                snapToGrid,
                containerBounds,
            });
            const childChanges = [];
            if (!node) {
                return;
            }
            const { x: prevX, y: prevY, width: prevWidth, height: prevHeight } = prevValues;
            const change = {};
            const nodeOrigin = node.origin ?? storeNodeOrigin;
            const { width, height, x, y } = getDimensionsAfterResize(startValues, params.controlDirection, pointerPosition, params.boundaries, params.keepAspectRatio, nodeOrigin, parentExtent, childExtent);
            const isWidthChange = width !== prevWidth;
            const isHeightChange = height !== prevHeight;
            const isXPosChange = x !== prevX && isWidthChange;
            const isYPosChange = y !== prevY && isHeightChange;
            if (!isXPosChange && !isYPosChange && !isWidthChange && !isHeightChange) {
                return;
            }
            if (isXPosChange || isYPosChange || nodeOrigin[0] === 1 || nodeOrigin[1] === 1) {
                change.x = isXPosChange ? x : prevValues.x;
                change.y = isYPosChange ? y : prevValues.y;
                prevValues.x = change.x;
                prevValues.y = change.y;
                /*
                 * when top/left changes, correct the relative positions of child nodes
                 * so that they stay in the same position
                 */
                if (childNodes.length > 0) {
                    const xChange = x - prevX;
                    const yChange = y - prevY;
                    for (const childNode of childNodes) {
                        childNode.position = {
                            x: childNode.position.x - xChange + nodeOrigin[0] * (width - prevWidth),
                            y: childNode.position.y - yChange + nodeOrigin[1] * (height - prevHeight),
                        };
                        childChanges.push(childNode);
                    }
                }
            }
            if (isWidthChange || isHeightChange) {
                change.width =
                    isWidthChange && (!params.resizeDirection || params.resizeDirection === 'horizontal')
                        ? width
                        : prevValues.width;
                change.height =
                    isHeightChange && (!params.resizeDirection || params.resizeDirection === 'vertical')
                        ? height
                        : prevValues.height;
                prevValues.width = change.width;
                prevValues.height = change.height;
            }
            // Fix expandParent when resizing from top/left
            if (parentNode && node.expandParent) {
                const xLimit = nodeOrigin[0] * (change.width ?? 0);
                if (change.x && change.x < xLimit) {
                    prevValues.x = xLimit;
                    startValues.x = startValues.x - (change.x - xLimit);
                }
                const yLimit = nodeOrigin[1] * (change.height ?? 0);
                if (change.y && change.y < yLimit) {
                    prevValues.y = yLimit;
                    startValues.y = startValues.y - (change.y - yLimit);
                }
            }
            const direction = getResizeDirection({
                width: prevValues.width,
                prevWidth,
                height: prevValues.height,
                prevHeight,
                affectsX: params.controlDirection.affectsX,
                affectsY: params.controlDirection.affectsY,
            });
            const nextValues = { ...prevValues, direction };
            const callResize = shouldResize?.(event, nextValues);
            if (callResize === false) {
                return;
            }
            resizeDetected = true;
            onResize?.(event, nextValues);
            onChange(change, childChanges);
        })
            .on('end', (event) => {
            if (!resizeDetected) {
                return;
            }
            onResizeEnd?.(event, { ...prevValues });
            onEnd?.({ ...prevValues });
            resizeDetected = false;
        });
        selection.call(dragHandler);
    }
    function destroy() {
        selection.on('.drag', null);
    }
    return {
        update,
        destroy,
    };
}

function getDefaultExportFromCjs (x) {
	return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, 'default') ? x['default'] : x;
}

var withSelector = {exports: {}};

var withSelector_production = {};

var shim = {exports: {}};

var useSyncExternalStoreShim_production = {};

/**
 * @license React
 * use-sync-external-store-shim.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

var hasRequiredUseSyncExternalStoreShim_production;

function requireUseSyncExternalStoreShim_production () {
	if (hasRequiredUseSyncExternalStoreShim_production) return useSyncExternalStoreShim_production;
	hasRequiredUseSyncExternalStoreShim_production = 1;
	var React = React__default;
	function is(x, y) {
	  return (x === y && (0 !== x || 1 / x === 1 / y)) || (x !== x && y !== y);
	}
	var objectIs = "function" === typeof Object.is ? Object.is : is,
	  useState = React.useState,
	  useEffect = React.useEffect,
	  useLayoutEffect = React.useLayoutEffect,
	  useDebugValue = React.useDebugValue;
	function useSyncExternalStore$2(subscribe, getSnapshot) {
	  var value = getSnapshot(),
	    _useState = useState({ inst: { value: value, getSnapshot: getSnapshot } }),
	    inst = _useState[0].inst,
	    forceUpdate = _useState[1];
	  useLayoutEffect(
	    function () {
	      inst.value = value;
	      inst.getSnapshot = getSnapshot;
	      checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	    },
	    [subscribe, value, getSnapshot]
	  );
	  useEffect(
	    function () {
	      checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	      return subscribe(function () {
	        checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	      });
	    },
	    [subscribe]
	  );
	  useDebugValue(value);
	  return value;
	}
	function checkIfSnapshotChanged(inst) {
	  var latestGetSnapshot = inst.getSnapshot;
	  inst = inst.value;
	  try {
	    var nextValue = latestGetSnapshot();
	    return !objectIs(inst, nextValue);
	  } catch (error) {
	    return true;
	  }
	}
	function useSyncExternalStore$1(subscribe, getSnapshot) {
	  return getSnapshot();
	}
	var shim =
	  "undefined" === typeof window ||
	  "undefined" === typeof window.document ||
	  "undefined" === typeof window.document.createElement
	    ? useSyncExternalStore$1
	    : useSyncExternalStore$2;
	useSyncExternalStoreShim_production.useSyncExternalStore =
	  void 0 !== React.useSyncExternalStore ? React.useSyncExternalStore : shim;
	return useSyncExternalStoreShim_production;
}

var useSyncExternalStoreShim_development = {};

/**
 * @license React
 * use-sync-external-store-shim.development.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

var hasRequiredUseSyncExternalStoreShim_development;

function requireUseSyncExternalStoreShim_development () {
	if (hasRequiredUseSyncExternalStoreShim_development) return useSyncExternalStoreShim_development;
	hasRequiredUseSyncExternalStoreShim_development = 1;
	"production" !== process.env.NODE_ENV &&
	  (function () {
	    function is(x, y) {
	      return (x === y && (0 !== x || 1 / x === 1 / y)) || (x !== x && y !== y);
	    }
	    function useSyncExternalStore$2(subscribe, getSnapshot) {
	      didWarnOld18Alpha ||
	        void 0 === React.startTransition ||
	        ((didWarnOld18Alpha = true),
	        console.error(
	          "You are using an outdated, pre-release alpha of React 18 that does not support useSyncExternalStore. The use-sync-external-store shim will not work correctly. Upgrade to a newer pre-release."
	        ));
	      var value = getSnapshot();
	      if (!didWarnUncachedGetSnapshot) {
	        var cachedValue = getSnapshot();
	        objectIs(value, cachedValue) ||
	          (console.error(
	            "The result of getSnapshot should be cached to avoid an infinite loop"
	          ),
	          (didWarnUncachedGetSnapshot = true));
	      }
	      cachedValue = useState({
	        inst: { value: value, getSnapshot: getSnapshot }
	      });
	      var inst = cachedValue[0].inst,
	        forceUpdate = cachedValue[1];
	      useLayoutEffect(
	        function () {
	          inst.value = value;
	          inst.getSnapshot = getSnapshot;
	          checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	        },
	        [subscribe, value, getSnapshot]
	      );
	      useEffect(
	        function () {
	          checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	          return subscribe(function () {
	            checkIfSnapshotChanged(inst) && forceUpdate({ inst: inst });
	          });
	        },
	        [subscribe]
	      );
	      useDebugValue(value);
	      return value;
	    }
	    function checkIfSnapshotChanged(inst) {
	      var latestGetSnapshot = inst.getSnapshot;
	      inst = inst.value;
	      try {
	        var nextValue = latestGetSnapshot();
	        return !objectIs(inst, nextValue);
	      } catch (error) {
	        return true;
	      }
	    }
	    function useSyncExternalStore$1(subscribe, getSnapshot) {
	      return getSnapshot();
	    }
	    "undefined" !== typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ &&
	      "function" ===
	        typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStart &&
	      __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStart(Error());
	    var React = React__default,
	      objectIs = "function" === typeof Object.is ? Object.is : is,
	      useState = React.useState,
	      useEffect = React.useEffect,
	      useLayoutEffect = React.useLayoutEffect,
	      useDebugValue = React.useDebugValue,
	      didWarnOld18Alpha = false,
	      didWarnUncachedGetSnapshot = false,
	      shim =
	        "undefined" === typeof window ||
	        "undefined" === typeof window.document ||
	        "undefined" === typeof window.document.createElement
	          ? useSyncExternalStore$1
	          : useSyncExternalStore$2;
	    useSyncExternalStoreShim_development.useSyncExternalStore =
	      void 0 !== React.useSyncExternalStore ? React.useSyncExternalStore : shim;
	    "undefined" !== typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ &&
	      "function" ===
	        typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStop &&
	      __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStop(Error());
	  })();
	return useSyncExternalStoreShim_development;
}

var hasRequiredShim;

function requireShim () {
	if (hasRequiredShim) return shim.exports;
	hasRequiredShim = 1;

	if (process.env.NODE_ENV === 'production') {
	  shim.exports = requireUseSyncExternalStoreShim_production();
	} else {
	  shim.exports = requireUseSyncExternalStoreShim_development();
	}
	return shim.exports;
}

/**
 * @license React
 * use-sync-external-store-shim/with-selector.production.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

var hasRequiredWithSelector_production;

function requireWithSelector_production () {
	if (hasRequiredWithSelector_production) return withSelector_production;
	hasRequiredWithSelector_production = 1;
	var React = React__default,
	  shim = requireShim();
	function is(x, y) {
	  return (x === y && (0 !== x || 1 / x === 1 / y)) || (x !== x && y !== y);
	}
	var objectIs = "function" === typeof Object.is ? Object.is : is,
	  useSyncExternalStore = shim.useSyncExternalStore,
	  useRef = React.useRef,
	  useEffect = React.useEffect,
	  useMemo = React.useMemo,
	  useDebugValue = React.useDebugValue;
	withSelector_production.useSyncExternalStoreWithSelector = function (
	  subscribe,
	  getSnapshot,
	  getServerSnapshot,
	  selector,
	  isEqual
	) {
	  var instRef = useRef(null);
	  if (null === instRef.current) {
	    var inst = { hasValue: false, value: null };
	    instRef.current = inst;
	  } else inst = instRef.current;
	  instRef = useMemo(
	    function () {
	      function memoizedSelector(nextSnapshot) {
	        if (!hasMemo) {
	          hasMemo = true;
	          memoizedSnapshot = nextSnapshot;
	          nextSnapshot = selector(nextSnapshot);
	          if (void 0 !== isEqual && inst.hasValue) {
	            var currentSelection = inst.value;
	            if (isEqual(currentSelection, nextSnapshot))
	              return (memoizedSelection = currentSelection);
	          }
	          return (memoizedSelection = nextSnapshot);
	        }
	        currentSelection = memoizedSelection;
	        if (objectIs(memoizedSnapshot, nextSnapshot)) return currentSelection;
	        var nextSelection = selector(nextSnapshot);
	        if (void 0 !== isEqual && isEqual(currentSelection, nextSelection))
	          return (memoizedSnapshot = nextSnapshot), currentSelection;
	        memoizedSnapshot = nextSnapshot;
	        return (memoizedSelection = nextSelection);
	      }
	      var hasMemo = false,
	        memoizedSnapshot,
	        memoizedSelection,
	        maybeGetServerSnapshot =
	          void 0 === getServerSnapshot ? null : getServerSnapshot;
	      return [
	        function () {
	          return memoizedSelector(getSnapshot());
	        },
	        null === maybeGetServerSnapshot
	          ? void 0
	          : function () {
	              return memoizedSelector(maybeGetServerSnapshot());
	            }
	      ];
	    },
	    [getSnapshot, getServerSnapshot, selector, isEqual]
	  );
	  var value = useSyncExternalStore(subscribe, instRef[0], instRef[1]);
	  useEffect(
	    function () {
	      inst.hasValue = true;
	      inst.value = value;
	    },
	    [value]
	  );
	  useDebugValue(value);
	  return value;
	};
	return withSelector_production;
}

var withSelector_development = {};

/**
 * @license React
 * use-sync-external-store-shim/with-selector.development.js
 *
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

var hasRequiredWithSelector_development;

function requireWithSelector_development () {
	if (hasRequiredWithSelector_development) return withSelector_development;
	hasRequiredWithSelector_development = 1;
	"production" !== process.env.NODE_ENV &&
	  (function () {
	    function is(x, y) {
	      return (x === y && (0 !== x || 1 / x === 1 / y)) || (x !== x && y !== y);
	    }
	    "undefined" !== typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ &&
	      "function" ===
	        typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStart &&
	      __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStart(Error());
	    var React = React__default,
	      shim = requireShim(),
	      objectIs = "function" === typeof Object.is ? Object.is : is,
	      useSyncExternalStore = shim.useSyncExternalStore,
	      useRef = React.useRef,
	      useEffect = React.useEffect,
	      useMemo = React.useMemo,
	      useDebugValue = React.useDebugValue;
	    withSelector_development.useSyncExternalStoreWithSelector = function (
	      subscribe,
	      getSnapshot,
	      getServerSnapshot,
	      selector,
	      isEqual
	    ) {
	      var instRef = useRef(null);
	      if (null === instRef.current) {
	        var inst = { hasValue: false, value: null };
	        instRef.current = inst;
	      } else inst = instRef.current;
	      instRef = useMemo(
	        function () {
	          function memoizedSelector(nextSnapshot) {
	            if (!hasMemo) {
	              hasMemo = true;
	              memoizedSnapshot = nextSnapshot;
	              nextSnapshot = selector(nextSnapshot);
	              if (void 0 !== isEqual && inst.hasValue) {
	                var currentSelection = inst.value;
	                if (isEqual(currentSelection, nextSnapshot))
	                  return (memoizedSelection = currentSelection);
	              }
	              return (memoizedSelection = nextSnapshot);
	            }
	            currentSelection = memoizedSelection;
	            if (objectIs(memoizedSnapshot, nextSnapshot))
	              return currentSelection;
	            var nextSelection = selector(nextSnapshot);
	            if (void 0 !== isEqual && isEqual(currentSelection, nextSelection))
	              return (memoizedSnapshot = nextSnapshot), currentSelection;
	            memoizedSnapshot = nextSnapshot;
	            return (memoizedSelection = nextSelection);
	          }
	          var hasMemo = false,
	            memoizedSnapshot,
	            memoizedSelection,
	            maybeGetServerSnapshot =
	              void 0 === getServerSnapshot ? null : getServerSnapshot;
	          return [
	            function () {
	              return memoizedSelector(getSnapshot());
	            },
	            null === maybeGetServerSnapshot
	              ? void 0
	              : function () {
	                  return memoizedSelector(maybeGetServerSnapshot());
	                }
	          ];
	        },
	        [getSnapshot, getServerSnapshot, selector, isEqual]
	      );
	      var value = useSyncExternalStore(subscribe, instRef[0], instRef[1]);
	      useEffect(
	        function () {
	          inst.hasValue = true;
	          inst.value = value;
	        },
	        [value]
	      );
	      useDebugValue(value);
	      return value;
	    };
	    "undefined" !== typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ &&
	      "function" ===
	        typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStop &&
	      __REACT_DEVTOOLS_GLOBAL_HOOK__.registerInternalModuleStop(Error());
	  })();
	return withSelector_development;
}

if (process.env.NODE_ENV === 'production') {
  withSelector.exports = requireWithSelector_production();
} else {
  withSelector.exports = requireWithSelector_development();
}

var withSelectorExports = withSelector.exports;
const useSyncExternalStoreExports = /*@__PURE__*/getDefaultExportFromCjs(withSelectorExports);

const __vite_import_meta_env__ = {};
const createStoreImpl$1 = (createState) => {
  let state;
  const listeners = /* @__PURE__ */ new Set();
  const setState = (partial, replace) => {
    const nextState = typeof partial === "function" ? partial(state) : partial;
    if (!Object.is(nextState, state)) {
      const previousState = state;
      state = (replace != null ? replace : typeof nextState !== "object" || nextState === null) ? nextState : Object.assign({}, state, nextState);
      listeners.forEach((listener) => listener(state, previousState));
    }
  };
  const getState = () => state;
  const getInitialState = () => initialState;
  const subscribe = (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };
  const destroy = () => {
    if ((__vite_import_meta_env__ ? "production" : void 0) !== "production") {
      console.warn(
        "[DEPRECATED] The `destroy` method will be unsupported in a future version. Instead use unsubscribe function returned by subscribe. Everything will be garbage-collected if store is garbage-collected."
      );
    }
    listeners.clear();
  };
  const api = { setState, getState, getInitialState, subscribe, destroy };
  const initialState = state = createState(setState, getState, api);
  return api;
};
const createStore$2 = (createState) => createState ? createStoreImpl$1(createState) : createStoreImpl$1;

const { useDebugValue } = React__default;
const { useSyncExternalStoreWithSelector } = useSyncExternalStoreExports;
const identity$1 = (arg) => arg;
function useStoreWithEqualityFn(api, selector = identity$1, equalityFn) {
  const slice = useSyncExternalStoreWithSelector(
    api.subscribe,
    api.getState,
    api.getServerState || api.getInitialState,
    selector,
    equalityFn
  );
  useDebugValue(slice);
  return slice;
}
const createWithEqualityFnImpl = (createState, defaultEqualityFn) => {
  const api = createStore$2(createState);
  const useBoundStoreWithEqualityFn = (selector, equalityFn = defaultEqualityFn) => useStoreWithEqualityFn(api, selector, equalityFn);
  Object.assign(useBoundStoreWithEqualityFn, api);
  return useBoundStoreWithEqualityFn;
};
const createWithEqualityFn = (createState, defaultEqualityFn) => createState ? createWithEqualityFnImpl(createState, defaultEqualityFn) : createWithEqualityFnImpl;

function shallow$1(objA, objB) {
  if (Object.is(objA, objB)) {
    return true;
  }
  if (typeof objA !== "object" || objA === null || typeof objB !== "object" || objB === null) {
    return false;
  }
  if (objA instanceof Map && objB instanceof Map) {
    if (objA.size !== objB.size) return false;
    for (const [key, value] of objA) {
      if (!Object.is(value, objB.get(key))) {
        return false;
      }
    }
    return true;
  }
  if (objA instanceof Set && objB instanceof Set) {
    if (objA.size !== objB.size) return false;
    for (const value of objA) {
      if (!objB.has(value)) {
        return false;
      }
    }
    return true;
  }
  const keysA = Object.keys(objA);
  if (keysA.length !== Object.keys(objB).length) {
    return false;
  }
  for (const keyA of keysA) {
    if (!Object.prototype.hasOwnProperty.call(objB, keyA) || !Object.is(objA[keyA], objB[keyA])) {
      return false;
    }
  }
  return true;
}

const StoreContext = createContext(null);
const Provider$1 = StoreContext.Provider;

const zustandErrorMessage = errorMessages['error001']();
/**
 * This hook can be used to subscribe to internal state changes of the React Flow
 * component. The `useStore` hook is re-exported from the [Zustand](https://github.com/pmndrs/zustand)
 * state management library, so you should check out their docs for more details.
 *
 * @public
 * @param selector - A selector function that returns a slice of the flow's internal state.
 * Extracting or transforming just the state you need is a good practice to avoid unnecessary
 * re-renders.
 * @param equalityFn - A function to compare the previous and next value. This is incredibly useful
 * for preventing unnecessary re-renders. Good sensible defaults are using `Object.is` or importing
 * `zustand/shallow`, but you can be as granular as you like.
 * @returns The selected state slice.
 *
 * @example
 * ```ts
 * const nodes = useStore((state) => state.nodes);
 * ```
 *
 * @remarks This hook should only be used if there is no other way to access the internal
 * state. For many of the common use cases, there are dedicated hooks available
 * such as {@link useReactFlow}, {@link useViewport}, etc.
 */
function useStore$1(selector, equalityFn) {
    const store = useContext(StoreContext);
    if (store === null) {
        throw new Error(zustandErrorMessage);
    }
    return useStoreWithEqualityFn(store, selector, equalityFn);
}
/**
 * In some cases, you might need to access the store directly. This hook returns the store object which can be used on demand to access the state or dispatch actions.
 *
 * @returns The store object.
 * @example
 * ```ts
 * const store = useStoreApi();
 * ```
 *
 * @remarks This hook should only be used if there is no other way to access the internal
 * state. For many of the common use cases, there are dedicated hooks available
 * such as {@link useReactFlow}, {@link useViewport}, etc.
 */
function useStoreApi() {
    const store = useContext(StoreContext);
    if (store === null) {
        throw new Error(zustandErrorMessage);
    }
    return useMemo(() => ({
        getState: store.getState,
        setState: store.setState,
        subscribe: store.subscribe,
    }), [store]);
}

const style = { display: 'none' };
const ariaLiveStyle = {
    position: 'absolute',
    width: 1,
    height: 1,
    margin: -1,
    border: 0,
    padding: 0,
    overflow: 'hidden',
    clip: 'rect(0px, 0px, 0px, 0px)',
    clipPath: 'inset(100%)',
};
const ARIA_NODE_DESC_KEY = 'react-flow__node-desc';
const ARIA_EDGE_DESC_KEY = 'react-flow__edge-desc';
const ARIA_LIVE_MESSAGE = 'react-flow__aria-live';
const ariaLiveSelector = (s) => s.ariaLiveMessage;
const ariaLabelConfigSelector = (s) => s.ariaLabelConfig;
function AriaLiveMessage({ rfId }) {
    const ariaLiveMessage = useStore$1(ariaLiveSelector);
    return (jsx("div", { id: `${ARIA_LIVE_MESSAGE}-${rfId}`, "aria-live": "assertive", "aria-atomic": "true", style: ariaLiveStyle, children: ariaLiveMessage }));
}
function A11yDescriptions({ rfId, disableKeyboardA11y }) {
    const ariaLabelConfig = useStore$1(ariaLabelConfigSelector);
    return (jsxs(Fragment, { children: [jsx("div", { id: `${ARIA_NODE_DESC_KEY}-${rfId}`, style: style, children: disableKeyboardA11y
                    ? ariaLabelConfig['node.a11yDescription.default']
                    : ariaLabelConfig['node.a11yDescription.keyboardDisabled'] }), jsx("div", { id: `${ARIA_EDGE_DESC_KEY}-${rfId}`, style: style, children: ariaLabelConfig['edge.a11yDescription.default'] }), !disableKeyboardA11y && jsx(AriaLiveMessage, { rfId: rfId })] }));
}

/**
 * The `<Panel />` component helps you position content above the viewport.
 * It is used internally by the [`<MiniMap />`](/api-reference/components/minimap)
 * and [`<Controls />`](/api-reference/components/controls) components.
 *
 * @public
 *
 * @example
 * ```jsx
 *import { ReactFlow, Background, Panel } from '@xyflow/react';
 *
 *export default function Flow() {
 *  return (
 *    <ReactFlow nodes={[]} fitView>
 *      <Panel position="top-left">top-left</Panel>
 *      <Panel position="top-center">top-center</Panel>
 *      <Panel position="top-right">top-right</Panel>
 *      <Panel position="bottom-left">bottom-left</Panel>
 *      <Panel position="bottom-center">bottom-center</Panel>
 *      <Panel position="bottom-right">bottom-right</Panel>
 *    </ReactFlow>
 *  );
 *}
 *```
 */
const Panel = forwardRef(({ position = 'top-left', children, className, style, ...rest }, ref) => {
    const positionClasses = `${position}`.split('-');
    return (jsx("div", { className: cc(['react-flow__panel', className, ...positionClasses]), style: style, ref: ref, ...rest, children: children }));
});
Panel.displayName = 'Panel';

function Attribution({ proOptions, position = 'bottom-right' }) {
    if (proOptions?.hideAttribution) {
        return null;
    }
    return (jsx(Panel, { position: position, className: "react-flow__attribution", "data-message": "Please only hide this attribution when you are subscribed to React Flow Pro: https://pro.reactflow.dev", children: jsx("a", { href: "https://reactflow.dev", target: "_blank", rel: "noopener noreferrer", "aria-label": "React Flow attribution", children: "React Flow" }) }));
}

const selector$m = (s) => {
    const selectedNodes = [];
    const selectedEdges = [];
    for (const [, node] of s.nodeLookup) {
        if (node.selected) {
            selectedNodes.push(node.internals.userNode);
        }
    }
    for (const [, edge] of s.edgeLookup) {
        if (edge.selected) {
            selectedEdges.push(edge);
        }
    }
    return { selectedNodes, selectedEdges };
};
const selectId = (obj) => obj.id;
function areEqual(a, b) {
    return (shallow$1(a.selectedNodes.map(selectId), b.selectedNodes.map(selectId)) &&
        shallow$1(a.selectedEdges.map(selectId), b.selectedEdges.map(selectId)));
}
function SelectionListenerInner({ onSelectionChange, }) {
    const store = useStoreApi();
    const { selectedNodes, selectedEdges } = useStore$1(selector$m, areEqual);
    useEffect(() => {
        const params = { nodes: selectedNodes, edges: selectedEdges };
        onSelectionChange?.(params);
        store.getState().onSelectionChangeHandlers.forEach((fn) => fn(params));
    }, [selectedNodes, selectedEdges, onSelectionChange]);
    return null;
}
const changeSelector = (s) => !!s.onSelectionChangeHandlers;
function SelectionListener({ onSelectionChange, }) {
    const storeHasSelectionChangeHandlers = useStore$1(changeSelector);
    if (onSelectionChange || storeHasSelectionChangeHandlers) {
        return jsx(SelectionListenerInner, { onSelectionChange: onSelectionChange });
    }
    return null;
}

// we need this hook to prevent a warning when using react-flow in SSR
const useIsomorphicLayoutEffect = typeof window !== 'undefined' ? useLayoutEffect : useEffect;

const defaultNodeOrigin = [0, 0];
const defaultViewport = { x: 0, y: 0, zoom: 1 };

/*
 * This component helps us to update the store with the values coming from the user.
 * We distinguish between values we can update directly with `useDirectStoreUpdater` (like `snapGrid`)
 * and values that have a dedicated setter function in the store (like `setNodes`).
 */
// These fields exist in the global store, and we need to keep them up to date
const reactFlowFieldsToTrack = [
    'nodes',
    'edges',
    'defaultNodes',
    'defaultEdges',
    'onConnect',
    'onConnectStart',
    'onConnectEnd',
    'onClickConnectStart',
    'onClickConnectEnd',
    'nodesDraggable',
    'autoPanOnNodeFocus',
    'nodesConnectable',
    'nodesFocusable',
    'edgesFocusable',
    'edgesReconnectable',
    'elevateNodesOnSelect',
    'elevateEdgesOnSelect',
    'minZoom',
    'maxZoom',
    'nodeExtent',
    'onNodesChange',
    'onEdgesChange',
    'elementsSelectable',
    'connectionMode',
    'snapGrid',
    'snapToGrid',
    'translateExtent',
    'connectOnClick',
    'defaultEdgeOptions',
    'fitView',
    'fitViewOptions',
    'onNodesDelete',
    'onEdgesDelete',
    'onDelete',
    'onNodeDrag',
    'onNodeDragStart',
    'onNodeDragStop',
    'onSelectionDrag',
    'onSelectionDragStart',
    'onSelectionDragStop',
    'onMoveStart',
    'onMove',
    'onMoveEnd',
    'noPanClassName',
    'nodeOrigin',
    'autoPanOnConnect',
    'autoPanOnNodeDrag',
    'onError',
    'connectionRadius',
    'isValidConnection',
    'selectNodesOnDrag',
    'nodeDragThreshold',
    'connectionDragThreshold',
    'onBeforeDelete',
    'debug',
    'autoPanSpeed',
    'ariaLabelConfig',
    'zIndexMode',
];
// rfId doesn't exist in ReactFlowProps, but it's one of the fields we want to update
const fieldsToTrack = [...reactFlowFieldsToTrack, 'rfId'];
const selector$l = (s) => ({
    setNodes: s.setNodes,
    setEdges: s.setEdges,
    setMinZoom: s.setMinZoom,
    setMaxZoom: s.setMaxZoom,
    setTranslateExtent: s.setTranslateExtent,
    setNodeExtent: s.setNodeExtent,
    reset: s.reset,
    setDefaultNodesAndEdges: s.setDefaultNodesAndEdges,
});
const initPrevValues = {
    /*
     * these are values that are also passed directly to other components
     * than the StoreUpdater. We can reduce the number of setStore calls
     * by setting the same values here as prev fields.
     */
    translateExtent: infiniteExtent,
    nodeOrigin: defaultNodeOrigin,
    minZoom: 0.5,
    maxZoom: 2,
    elementsSelectable: true,
    noPanClassName: 'nopan',
    rfId: '1',
};
function StoreUpdater(props) {
    const { setNodes, setEdges, setMinZoom, setMaxZoom, setTranslateExtent, setNodeExtent, reset, setDefaultNodesAndEdges, } = useStore$1(selector$l, shallow$1);
    const store = useStoreApi();
    // We use layout effects here so that the store is always populated before
    // any child useEffect or useLayoutEffect fires. With regular useEffect, the
    // cleanup calls reset() which empties the store, and child effects can run
    // before the new mount effect repopulates it — causing children to read
    // empty nodeLookup/nodes/edges during a <ReactFlow> remount.
    useIsomorphicLayoutEffect(() => {
        setDefaultNodesAndEdges(props.defaultNodes, props.defaultEdges);
        return () => {
            // when we reset the store we also need to reset the previous fields
            previousFields.current = initPrevValues;
            reset();
        };
    }, []);
    const previousFields = useRef(initPrevValues);
    useIsomorphicLayoutEffect(() => {
        for (const fieldName of fieldsToTrack) {
            const fieldValue = props[fieldName];
            const previousFieldValue = previousFields.current[fieldName];
            if (fieldValue === previousFieldValue)
                continue;
            if (typeof props[fieldName] === 'undefined')
                continue;
            // Custom handling with dedicated setters for some fields
            if (fieldName === 'nodes')
                setNodes(fieldValue);
            else if (fieldName === 'edges')
                setEdges(fieldValue);
            else if (fieldName === 'minZoom')
                setMinZoom(fieldValue);
            else if (fieldName === 'maxZoom')
                setMaxZoom(fieldValue);
            else if (fieldName === 'translateExtent')
                setTranslateExtent(fieldValue);
            else if (fieldName === 'nodeExtent')
                setNodeExtent(fieldValue);
            else if (fieldName === 'ariaLabelConfig')
                store.setState({ ariaLabelConfig: mergeAriaLabelConfig(fieldValue) });
            // Renamed fields
            else if (fieldName === 'fitView')
                store.setState({ fitViewQueued: fieldValue });
            else if (fieldName === 'fitViewOptions')
                store.setState({ fitViewOptions: fieldValue });
            // General case
            else
                store.setState({ [fieldName]: fieldValue });
        }
        previousFields.current = props;
    }, 
    // Only re-run the effect if one of the fields we track changes
    fieldsToTrack.map((fieldName) => props[fieldName]));
    return null;
}

function getMediaQuery() {
    if (typeof window === 'undefined' || !window.matchMedia) {
        return null;
    }
    return window.matchMedia('(prefers-color-scheme: dark)');
}
/**
 * Hook for receiving the current color mode class 'dark' or 'light'.
 *
 * @internal
 * @param colorMode - The color mode to use ('dark', 'light' or 'system')
 */
function useColorModeClass(colorMode) {
    const [colorModeClass, setColorModeClass] = useState(colorMode === 'system' ? null : colorMode);
    useEffect(() => {
        if (colorMode !== 'system') {
            setColorModeClass(colorMode);
            return;
        }
        const mediaQuery = getMediaQuery();
        const updateColorModeClass = () => setColorModeClass(mediaQuery?.matches ? 'dark' : 'light');
        updateColorModeClass();
        mediaQuery?.addEventListener('change', updateColorModeClass);
        return () => {
            mediaQuery?.removeEventListener('change', updateColorModeClass);
        };
    }, [colorMode]);
    return colorModeClass !== null ? colorModeClass : getMediaQuery()?.matches ? 'dark' : 'light';
}

const defaultDoc = typeof document !== 'undefined' ? document : null;
/**
 * This hook lets you listen for specific key codes and tells you whether they are
 * currently pressed or not.
 *
 * @public
 * @param options - Options
 *
 * @example
 * ```tsx
 *import { useKeyPress } from '@xyflow/react';
 *
 *export default function () {
 *  const spacePressed = useKeyPress('Space');
 *  const cmdAndSPressed = useKeyPress(['Meta+s', 'Strg+s']);
 *
 *  return (
 *    <div>
 *     {spacePressed && <p>Space pressed!</p>}
 *     {cmdAndSPressed && <p>Cmd + S pressed!</p>}
 *    </div>
 *  );
 *}
 *```
 */
function useKeyPress(
/**
 * The key code (string or array of strings) specifies which key(s) should trigger
 * an action.
 *
 * A **string** can represent:
 * - A **single key**, e.g. `'a'`
 * - A **key combination**, using `'+'` to separate keys, e.g. `'a+d'`
 *
 * An  **array of strings** represents **multiple possible key inputs**. For example, `['a', 'd+s']`
 * means the user can press either the single key `'a'` or the combination of `'d'` and `'s'`.
 * @default null
 */
keyCode = null, options = { target: defaultDoc, actInsideInputWithModifier: true }) {
    const [keyPressed, setKeyPressed] = useState(false);
    // we need to remember if a modifier key is pressed in order to track it
    const modifierPressed = useRef(false);
    // we need to remember the pressed keys in order to support combinations
    const pressedKeys = useRef(new Set([]));
    /*
     * keyCodes = array with single keys [['a']] or key combinations [['a', 's']]
     * keysToWatch = array with all keys flattened ['a', 'd', 'ShiftLeft']
     * used to check if we store event.code or event.key. When the code is in the list of keysToWatch
     * we use the code otherwise the key. Explainer: When you press the left "command" key, the code is "MetaLeft"
     * and the key is "Meta". We want users to be able to pass keys and codes so we assume that the key is meant when
     * we can't find it in the list of keysToWatch.
     */
    const [keyCodes, keysToWatch] = useMemo(() => {
        if (keyCode !== null) {
            const keyCodeArr = Array.isArray(keyCode) ? keyCode : [keyCode];
            const keys = keyCodeArr
                .filter((kc) => typeof kc === 'string')
                /*
                 * we first replace all '+' with '\n'  which we will use to split the keys on
                 * then we replace '\n\n' with '\n+', this way we can also support the combination 'key++'
                 * in the end we simply split on '\n' to get the key array
                 */
                .map((kc) => kc.replace('+', '\n').replace('\n\n', '\n+').split('\n'));
            const keysFlat = keys.reduce((res, item) => res.concat(...item), []);
            return [keys, keysFlat];
        }
        return [[], []];
    }, [keyCode]);
    useEffect(() => {
        const target = options?.target ?? defaultDoc;
        const actInsideInputWithModifier = options?.actInsideInputWithModifier ?? true;
        if (keyCode !== null) {
            const downHandler = (event) => {
                modifierPressed.current = event.ctrlKey || event.metaKey || event.shiftKey || event.altKey;
                const preventAction = (!modifierPressed.current || (modifierPressed.current && !actInsideInputWithModifier)) &&
                    isInputDOMNode(event);
                if (preventAction) {
                    return false;
                }
                const keyOrCode = useKeyOrCode(event.code, keysToWatch);
                pressedKeys.current.add(event[keyOrCode]);
                if (isMatchingKey(keyCodes, pressedKeys.current, false)) {
                    const target = (event.composedPath?.()?.[0] || event.target);
                    const isInteractiveElement = target?.nodeName === 'BUTTON' || target?.nodeName === 'A';
                    if (options.preventDefault !== false && (modifierPressed.current || !isInteractiveElement)) {
                        event.preventDefault();
                    }
                    setKeyPressed(true);
                }
            };
            const upHandler = (event) => {
                const keyOrCode = useKeyOrCode(event.code, keysToWatch);
                if (isMatchingKey(keyCodes, pressedKeys.current, true)) {
                    setKeyPressed(false);
                    pressedKeys.current.clear();
                }
                else {
                    pressedKeys.current.delete(event[keyOrCode]);
                }
                // fix for Mac: when cmd key is pressed, keyup is not triggered for any other key, see: https://stackoverflow.com/questions/27380018/when-cmd-key-is-kept-pressed-keyup-is-not-triggered-for-any-other-key
                if (event.key === 'Meta') {
                    pressedKeys.current.clear();
                }
                modifierPressed.current = false;
            };
            const resetHandler = () => {
                pressedKeys.current.clear();
                setKeyPressed(false);
            };
            target?.addEventListener('keydown', downHandler);
            target?.addEventListener('keyup', upHandler);
            window.addEventListener('blur', resetHandler);
            window.addEventListener('contextmenu', resetHandler);
            return () => {
                target?.removeEventListener('keydown', downHandler);
                target?.removeEventListener('keyup', upHandler);
                window.removeEventListener('blur', resetHandler);
                window.removeEventListener('contextmenu', resetHandler);
            };
        }
    }, [keyCode, setKeyPressed]);
    return keyPressed;
}
// utils
function isMatchingKey(keyCodes, pressedKeys, isUp) {
    return (keyCodes
        /*
         * we only want to compare same sizes of keyCode definitions
         * and pressed keys. When the user specified 'Meta' as a key somewhere
         * this would also be truthy without this filter when user presses 'Meta' + 'r'
         */
        .filter((keys) => isUp || keys.length === pressedKeys.size)
        /*
         * since we want to support multiple possibilities only one of the
         * combinations need to be part of the pressed keys
         */
        .some((keys) => keys.every((k) => pressedKeys.has(k))));
}
function useKeyOrCode(eventCode, keysToWatch) {
    return keysToWatch.includes(eventCode) ? 'code' : 'key';
}

/**
 * Hook for getting viewport helper functions.
 *
 * @internal
 * @returns viewport helper functions
 */
const useViewportHelper = () => {
    const store = useStoreApi();
    return useMemo(() => {
        return {
            zoomIn: (options) => {
                const { panZoom } = store.getState();
                return panZoom ? panZoom.scaleBy(1.2, options) : Promise.resolve(false);
            },
            zoomOut: (options) => {
                const { panZoom } = store.getState();
                return panZoom ? panZoom.scaleBy(1 / 1.2, options) : Promise.resolve(false);
            },
            zoomTo: (zoomLevel, options) => {
                const { panZoom } = store.getState();
                return panZoom ? panZoom.scaleTo(zoomLevel, options) : Promise.resolve(false);
            },
            getZoom: () => store.getState().transform[2],
            setViewport: async (viewport, options) => {
                const { transform: [tX, tY, tZoom], panZoom, } = store.getState();
                if (!panZoom) {
                    return Promise.resolve(false);
                }
                await panZoom.setViewport({
                    x: viewport.x ?? tX,
                    y: viewport.y ?? tY,
                    zoom: viewport.zoom ?? tZoom,
                }, options);
                return Promise.resolve(true);
            },
            getViewport: () => {
                const [x, y, zoom] = store.getState().transform;
                return { x, y, zoom };
            },
            setCenter: async (x, y, options) => {
                return store.getState().setCenter(x, y, options);
            },
            fitBounds: async (bounds, options) => {
                const { width, height, minZoom, maxZoom, panZoom } = store.getState();
                const viewport = getViewportForBounds(bounds, width, height, minZoom, maxZoom, options?.padding ?? 0.1);
                if (!panZoom) {
                    return Promise.resolve(false);
                }
                await panZoom.setViewport(viewport, {
                    duration: options?.duration,
                    ease: options?.ease,
                    interpolate: options?.interpolate,
                });
                return Promise.resolve(true);
            },
            screenToFlowPosition: (clientPosition, options = {}) => {
                const { transform, snapGrid, snapToGrid, domNode } = store.getState();
                if (!domNode) {
                    return clientPosition;
                }
                const { x: domX, y: domY } = domNode.getBoundingClientRect();
                const correctedPosition = {
                    x: clientPosition.x - domX,
                    y: clientPosition.y - domY,
                };
                const _snapGrid = options.snapGrid ?? snapGrid;
                const _snapToGrid = options.snapToGrid ?? snapToGrid;
                return pointToRendererPoint(correctedPosition, transform, _snapToGrid, _snapGrid);
            },
            flowToScreenPosition: (flowPosition) => {
                const { transform, domNode } = store.getState();
                if (!domNode) {
                    return flowPosition;
                }
                const { x: domX, y: domY } = domNode.getBoundingClientRect();
                const rendererPosition = rendererPointToPoint(flowPosition, transform);
                return {
                    x: rendererPosition.x + domX,
                    y: rendererPosition.y + domY,
                };
            },
        };
    }, []);
};

/*
 * This function applies changes to nodes or edges that are triggered by React Flow internally.
 * When you drag a node for example, React Flow will send a position change update.
 * This function then applies the changes and returns the updated elements.
 */
function applyChanges(changes, elements) {
    const updatedElements = [];
    /*
     * By storing a map of changes for each element, we can a quick lookup as we
     * iterate over the elements array!
     */
    const changesMap = new Map();
    const addItemChanges = [];
    for (const change of changes) {
        if (change.type === 'add') {
            addItemChanges.push(change);
            continue;
        }
        else if (change.type === 'remove' || change.type === 'replace') {
            /*
             * For a 'remove' change we can safely ignore any other changes queued for
             * the same element, it's going to be removed anyway!
             */
            changesMap.set(change.id, [change]);
        }
        else {
            const elementChanges = changesMap.get(change.id);
            if (elementChanges) {
                /*
                 * If we have some changes queued already, we can do a mutable update of
                 * that array and save ourselves some copying.
                 */
                elementChanges.push(change);
            }
            else {
                changesMap.set(change.id, [change]);
            }
        }
    }
    for (const element of elements) {
        const changes = changesMap.get(element.id);
        /*
         * When there are no changes for an element we can just push it unmodified,
         * no need to copy it.
         */
        if (!changes) {
            updatedElements.push(element);
            continue;
        }
        // If we have a 'remove' change queued, it'll be the only change in the array
        if (changes[0].type === 'remove') {
            continue;
        }
        if (changes[0].type === 'replace') {
            updatedElements.push({ ...changes[0].item });
            continue;
        }
        /**
         * For other types of changes, we want to start with a shallow copy of the
         * object so React knows this element has changed. Sequential changes will
         * each _mutate_ this object, so there's only ever one copy.
         */
        const updatedElement = { ...element };
        for (const change of changes) {
            applyChange(change, updatedElement);
        }
        updatedElements.push(updatedElement);
    }
    /*
     * we need to wait for all changes to be applied before adding new items
     * to be able to add them at the correct index
     */
    if (addItemChanges.length) {
        addItemChanges.forEach((change) => {
            if (change.index !== undefined) {
                updatedElements.splice(change.index, 0, { ...change.item });
            }
            else {
                updatedElements.push({ ...change.item });
            }
        });
    }
    return updatedElements;
}
// Applies a single change to an element. This is a *mutable* update.
function applyChange(change, element) {
    switch (change.type) {
        case 'select': {
            element.selected = change.selected;
            break;
        }
        case 'position': {
            if (typeof change.position !== 'undefined') {
                element.position = change.position;
            }
            if (typeof change.dragging !== 'undefined') {
                element.dragging = change.dragging;
            }
            break;
        }
        case 'dimensions': {
            if (typeof change.dimensions !== 'undefined') {
                element.measured = {
                    ...change.dimensions,
                };
                if (change.setAttributes) {
                    if (change.setAttributes === true || change.setAttributes === 'width') {
                        element.width = change.dimensions.width;
                    }
                    if (change.setAttributes === true || change.setAttributes === 'height') {
                        element.height = change.dimensions.height;
                    }
                }
            }
            if (typeof change.resizing === 'boolean') {
                element.resizing = change.resizing;
            }
            break;
        }
    }
}
/**
 * Drop in function that applies node changes to an array of nodes.
 * @public
 * @param changes - Array of changes to apply.
 * @param nodes - Array of nodes to apply the changes to.
 * @returns Array of updated nodes.
 * @example
 *```tsx
 *import { useState, useCallback } from 'react';
 *import { ReactFlow, applyNodeChanges, type Node, type Edge, type OnNodesChange } from '@xyflow/react';
 *
 *export default function Flow() {
 *  const [nodes, setNodes] = useState<Node[]>([]);
 *  const [edges, setEdges] = useState<Edge[]>([]);
 *  const onNodesChange: OnNodesChange = useCallback(
 *    (changes) => {
 *      setNodes((oldNodes) => applyNodeChanges(changes, oldNodes));
 *    },
 *    [setNodes],
 *  );
 *
 *  return (
 *    <ReactFlow nodes={nodes} edges={edges} onNodesChange={onNodesChange} />
 *  );
 *}
 *```
 * @remarks Various events on the <ReactFlow /> component can produce an {@link NodeChange}
 * that describes how to update the edges of your flow in some way.
 * If you don't need any custom behaviour, this util can be used to take an array
 * of these changes and apply them to your edges.
 */
function applyNodeChanges(changes, nodes) {
    return applyChanges(changes, nodes);
}
/**
 * Drop in function that applies edge changes to an array of edges.
 * @public
 * @param changes - Array of changes to apply.
 * @param edges - Array of edge to apply the changes to.
 * @returns Array of updated edges.
 * @example
 * ```tsx
 *import { useState, useCallback } from 'react';
 *import { ReactFlow, applyEdgeChanges } from '@xyflow/react';
 *
 *export default function Flow() {
 *  const [nodes, setNodes] = useState([]);
 *  const [edges, setEdges] = useState([]);
 *  const onEdgesChange = useCallback(
 *    (changes) => {
 *      setEdges((oldEdges) => applyEdgeChanges(changes, oldEdges));
 *    },
 *    [setEdges],
 *  );
 *
 *  return (
 *    <ReactFlow nodes={nodes} edges={edges} onEdgesChange={onEdgesChange} />
 *  );
 *}
 *```
 * @remarks Various events on the <ReactFlow /> component can produce an {@link EdgeChange}
 * that describes how to update the edges of your flow in some way.
 * If you don't need any custom behaviour, this util can be used to take an array
 * of these changes and apply them to your edges.
 */
function applyEdgeChanges(changes, edges) {
    return applyChanges(changes, edges);
}
function createSelectionChange(id, selected) {
    return {
        id,
        type: 'select',
        selected,
    };
}
function getSelectionChanges(items, selectedIds = new Set(), mutateItem = false) {
    const changes = [];
    for (const [id, item] of items) {
        const willBeSelected = selectedIds.has(id);
        // we don't want to set all items to selected=false on the first selection
        if (!(item.selected === undefined && !willBeSelected) && item.selected !== willBeSelected) {
            if (mutateItem) {
                /*
                 * this hack is needed for nodes. When the user dragged a node, it's selected.
                 * When another node gets dragged, we need to deselect the previous one,
                 * in order to have only one selected node at a time - the onNodesChange callback comes too late here :/
                 */
                item.selected = willBeSelected;
            }
            changes.push(createSelectionChange(item.id, willBeSelected));
        }
    }
    return changes;
}
function getElementsDiffChanges({ items = [], lookup, }) {
    const changes = [];
    const itemsLookup = new Map(items.map((item) => [item.id, item]));
    for (const [index, item] of items.entries()) {
        const lookupItem = lookup.get(item.id);
        const storeItem = lookupItem?.internals?.userNode ?? lookupItem;
        if (storeItem !== undefined && storeItem !== item) {
            changes.push({ id: item.id, item: item, type: 'replace' });
        }
        if (storeItem === undefined) {
            changes.push({ item: item, type: 'add', index });
        }
    }
    for (const [id] of lookup) {
        const nextNode = itemsLookup.get(id);
        if (nextNode === undefined) {
            changes.push({ id, type: 'remove' });
        }
    }
    return changes;
}
function elementToRemoveChange(item) {
    return {
        id: item.id,
        type: 'remove',
    };
}

/**
 * Test whether an object is usable as an [`Node`](/api-reference/types/node).
 * In TypeScript this is a type guard that will narrow the type of whatever you pass in to
 * [`Node`](/api-reference/types/node) if it returns `true`.
 *
 * @public
 * @remarks In TypeScript this is a type guard that will narrow the type of whatever you pass in to Node if it returns true
 * @param element - The element to test.
 * @returns Tests whether the provided value can be used as a `Node`. If you're using TypeScript,
 * this function acts as a type guard and will narrow the type of the value to `Node` if it returns
 * `true`.
 *
 * @example
 * ```js
 *import { isNode } from '@xyflow/react';
 *
 *if (isNode(node)) {
 * // ...
 *}
 *```
 */
const isNode = (element) => isNodeBase(element);
/**
 * Test whether an object is usable as an [`Edge`](/api-reference/types/edge).
 * In TypeScript this is a type guard that will narrow the type of whatever you pass in to
 * [`Edge`](/api-reference/types/edge) if it returns `true`.
 *
 * @public
 * @remarks In TypeScript this is a type guard that will narrow the type of whatever you pass in to Edge if it returns true
 * @param element - The element to test
 * @returns Tests whether the provided value can be used as an `Edge`. If you're using TypeScript,
 * this function acts as a type guard and will narrow the type of the value to `Edge` if it returns
 * `true`.
 *
 * @example
 * ```js
 *import { isEdge } from '@xyflow/react';
 *
 *if (isEdge(edge)) {
 * // ...
 *}
 *```
 */
const isEdge = (element) => isEdgeBase(element);
// eslint-disable-next-line @typescript-eslint/no-empty-object-type
function fixedForwardRef(render) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return forwardRef(render);
}

/**
 * This hook returns a queue that can be used to batch updates.
 *
 * @param runQueue - a function that gets called when the queue is flushed
 * @internal
 *
 * @returns a Queue object
 */
function useQueue(runQueue) {
    /*
     * Because we're using a ref above, we need some way to let React know when to
     * actually process the queue. We increment this number any time we mutate the
     * queue, creating a new state to trigger the layout effect below.
     * Using a boolean dirty flag here instead would lead to issues related to
     * automatic batching. (https://github.com/xyflow/xyflow/issues/4779)
     */
    const [serial, setSerial] = useState(BigInt(0));
    /*
     * A reference of all the batched updates to process before the next render. We
     * want a reference here so multiple synchronous calls to `setNodes` etc can be
     * batched together.
     */
    const [queue] = useState(() => createQueue(() => setSerial(n => n + BigInt(1))));
    /*
     * Layout effects are guaranteed to run before the next render which means we
     * shouldn't run into any issues with stale state or weird issues that come from
     * rendering things one frame later than expected (we used to use `setTimeout`).
     */
    useIsomorphicLayoutEffect(() => {
        const queueItems = queue.get();
        if (queueItems.length) {
            runQueue(queueItems);
            queue.reset();
        }
    }, [serial]);
    return queue;
}
function createQueue(cb) {
    let queue = [];
    return {
        get: () => queue,
        reset: () => {
            queue = [];
        },
        push: (item) => {
            queue.push(item);
            cb();
        },
    };
}

const BatchContext = createContext(null);
/**
 * This is a context provider that holds and processes the node and edge update queues
 * that are needed to handle setNodes, addNodes, setEdges and addEdges.
 *
 * @internal
 */
function BatchProvider({ children, }) {
    const store = useStoreApi();
    const nodeQueueHandler = useCallback((queueItems) => {
        const { nodes = [], setNodes, hasDefaultNodes, onNodesChange, nodeLookup, fitViewQueued, onNodesChangeMiddlewareMap, } = store.getState();
        /*
         * This is essentially an `Array.reduce` in imperative clothing. Processing
         * this queue is a relatively hot path so we'd like to avoid the overhead of
         * array methods where we can.
         */
        let next = nodes;
        for (const payload of queueItems) {
            next = typeof payload === 'function' ? payload(next) : payload;
        }
        let changes = getElementsDiffChanges({
            items: next,
            lookup: nodeLookup,
        });
        for (const middleware of onNodesChangeMiddlewareMap.values()) {
            changes = middleware(changes);
        }
        if (hasDefaultNodes) {
            setNodes(next);
        }
        // We only want to fire onNodesChange if there are changes to the nodes
        if (changes.length > 0) {
            onNodesChange?.(changes);
        }
        else if (fitViewQueued) {
            // If there are no changes to the nodes, we still need to call setNodes
            // to trigger a re-render and fitView.
            window.requestAnimationFrame(() => {
                const { fitViewQueued, nodes, setNodes } = store.getState();
                if (fitViewQueued) {
                    setNodes(nodes);
                }
            });
        }
    }, []);
    const nodeQueue = useQueue(nodeQueueHandler);
    const edgeQueueHandler = useCallback((queueItems) => {
        const { edges = [], setEdges, hasDefaultEdges, onEdgesChange, edgeLookup } = store.getState();
        let next = edges;
        for (const payload of queueItems) {
            next = typeof payload === 'function' ? payload(next) : payload;
        }
        if (hasDefaultEdges) {
            setEdges(next);
        }
        else if (onEdgesChange) {
            onEdgesChange(getElementsDiffChanges({
                items: next,
                lookup: edgeLookup,
            }));
        }
    }, []);
    const edgeQueue = useQueue(edgeQueueHandler);
    const value = useMemo(() => ({ nodeQueue, edgeQueue }), []);
    return jsx(BatchContext.Provider, { value: value, children: children });
}
function useBatchContext() {
    const batchContext = useContext(BatchContext);
    if (!batchContext) {
        throw new Error('useBatchContext must be used within a BatchProvider');
    }
    return batchContext;
}

const selector$k = (s) => !!s.panZoom;
/**
 * This hook returns a ReactFlowInstance that can be used to update nodes and edges, manipulate the viewport, or query the current state of the flow.
 *
 * @public
 * @example
 * ```jsx
 *import { useCallback, useState } from 'react';
 *import { useReactFlow } from '@xyflow/react';
 *
 *export function NodeCounter() {
 *  const reactFlow = useReactFlow();
 *  const [count, setCount] = useState(0);
 *  const countNodes = useCallback(() => {
 *    setCount(reactFlow.getNodes().length);
 *    // you need to pass it as a dependency if you are using it with useEffect or useCallback
 *    // because at the first render, it's not initialized yet and some functions might not work.
 *  }, [reactFlow]);
 *
 *  return (
 *    <div>
 *      <button onClick={countNodes}>Update count</button>
 *      <p>There are {count} nodes in the flow.</p>
 *    </div>
 *  );
 *}
 *```
 */
function useReactFlow() {
    const viewportHelper = useViewportHelper();
    const store = useStoreApi();
    const batchContext = useBatchContext();
    const viewportInitialized = useStore$1(selector$k);
    const generalHelper = useMemo(() => {
        const getInternalNode = (id) => store.getState().nodeLookup.get(id);
        const setNodes = (payload) => {
            batchContext.nodeQueue.push(payload);
        };
        const setEdges = (payload) => {
            batchContext.edgeQueue.push(payload);
        };
        const getNodeRect = (node) => {
            const { nodeLookup, nodeOrigin } = store.getState();
            const nodeToUse = isNode(node) ? node : nodeLookup.get(node.id);
            const position = nodeToUse.parentId
                ? evaluateAbsolutePosition(nodeToUse.position, nodeToUse.measured, nodeToUse.parentId, nodeLookup, nodeOrigin)
                : nodeToUse.position;
            const nodeWithPosition = {
                ...nodeToUse,
                position,
                width: nodeToUse.measured?.width ?? nodeToUse.width,
                height: nodeToUse.measured?.height ?? nodeToUse.height,
            };
            return nodeToRect(nodeWithPosition);
        };
        const updateNode = (id, nodeUpdate, options = { replace: false }) => {
            setNodes((prevNodes) => prevNodes.map((node) => {
                if (node.id === id) {
                    const nextNode = typeof nodeUpdate === 'function' ? nodeUpdate(node) : nodeUpdate;
                    return options.replace && isNode(nextNode) ? nextNode : { ...node, ...nextNode };
                }
                return node;
            }));
        };
        const updateEdge = (id, edgeUpdate, options = { replace: false }) => {
            setEdges((prevEdges) => prevEdges.map((edge) => {
                if (edge.id === id) {
                    const nextEdge = typeof edgeUpdate === 'function' ? edgeUpdate(edge) : edgeUpdate;
                    return options.replace && isEdge(nextEdge) ? nextEdge : { ...edge, ...nextEdge };
                }
                return edge;
            }));
        };
        return {
            getNodes: () => store.getState().nodes.map((n) => ({ ...n })),
            getNode: (id) => getInternalNode(id)?.internals.userNode,
            getInternalNode,
            getEdges: () => {
                const { edges = [] } = store.getState();
                return edges.map((e) => ({ ...e }));
            },
            getEdge: (id) => store.getState().edgeLookup.get(id),
            setNodes,
            setEdges,
            addNodes: (payload) => {
                const newNodes = Array.isArray(payload) ? payload : [payload];
                batchContext.nodeQueue.push((nodes) => [...nodes, ...newNodes]);
            },
            addEdges: (payload) => {
                const newEdges = Array.isArray(payload) ? payload : [payload];
                batchContext.edgeQueue.push((edges) => [...edges, ...newEdges]);
            },
            toObject: () => {
                const { nodes = [], edges = [], transform } = store.getState();
                const [x, y, zoom] = transform;
                return {
                    nodes: nodes.map((n) => ({ ...n })),
                    edges: edges.map((e) => ({ ...e })),
                    viewport: {
                        x,
                        y,
                        zoom,
                    },
                };
            },
            deleteElements: async ({ nodes: nodesToRemove = [], edges: edgesToRemove = [] }) => {
                const { nodes, edges, onNodesDelete, onEdgesDelete, triggerNodeChanges, triggerEdgeChanges, onDelete, onBeforeDelete, } = store.getState();
                const { nodes: matchingNodes, edges: matchingEdges } = await getElementsToRemove({
                    nodesToRemove,
                    edgesToRemove,
                    nodes,
                    edges,
                    onBeforeDelete,
                });
                const hasMatchingEdges = matchingEdges.length > 0;
                const hasMatchingNodes = matchingNodes.length > 0;
                if (hasMatchingEdges) {
                    const edgeChanges = matchingEdges.map(elementToRemoveChange);
                    onEdgesDelete?.(matchingEdges);
                    triggerEdgeChanges(edgeChanges);
                }
                if (hasMatchingNodes) {
                    const nodeChanges = matchingNodes.map(elementToRemoveChange);
                    onNodesDelete?.(matchingNodes);
                    triggerNodeChanges(nodeChanges);
                }
                if (hasMatchingNodes || hasMatchingEdges) {
                    onDelete?.({ nodes: matchingNodes, edges: matchingEdges });
                }
                return { deletedNodes: matchingNodes, deletedEdges: matchingEdges };
            },
            /**
             * Partial is defined as "the 2 nodes/areas are intersecting partially".
             * If a is contained in b or b is contained in a, they are both
             * considered fully intersecting.
             */
            getIntersectingNodes: (nodeOrRect, partially = true, nodes) => {
                const isRect = isRectObject(nodeOrRect);
                const nodeRect = isRect ? nodeOrRect : getNodeRect(nodeOrRect);
                const hasNodesOption = nodes !== undefined;
                if (!nodeRect) {
                    return [];
                }
                return (nodes || store.getState().nodes).filter((n) => {
                    const internalNode = store.getState().nodeLookup.get(n.id);
                    if (internalNode && !isRect && (n.id === nodeOrRect.id || !internalNode.internals.positionAbsolute)) {
                        return false;
                    }
                    const currNodeRect = nodeToRect(hasNodesOption ? n : internalNode);
                    const overlappingArea = getOverlappingArea(currNodeRect, nodeRect);
                    const partiallyVisible = partially && overlappingArea > 0;
                    return (partiallyVisible ||
                        overlappingArea >= currNodeRect.width * currNodeRect.height ||
                        overlappingArea >= nodeRect.width * nodeRect.height);
                });
            },
            isNodeIntersecting: (nodeOrRect, area, partially = true) => {
                const isRect = isRectObject(nodeOrRect);
                const nodeRect = isRect ? nodeOrRect : getNodeRect(nodeOrRect);
                if (!nodeRect) {
                    return false;
                }
                const overlappingArea = getOverlappingArea(nodeRect, area);
                const partiallyVisible = partially && overlappingArea > 0;
                return (partiallyVisible ||
                    overlappingArea >= area.width * area.height ||
                    overlappingArea >= nodeRect.width * nodeRect.height);
            },
            updateNode,
            updateNodeData: (id, dataUpdate, options = { replace: false }) => {
                updateNode(id, (node) => {
                    const nextData = typeof dataUpdate === 'function' ? dataUpdate(node) : dataUpdate;
                    return options.replace ? { ...node, data: nextData } : { ...node, data: { ...node.data, ...nextData } };
                }, options);
            },
            updateEdge,
            updateEdgeData: (id, dataUpdate, options = { replace: false }) => {
                updateEdge(id, (edge) => {
                    const nextData = typeof dataUpdate === 'function' ? dataUpdate(edge) : dataUpdate;
                    return options.replace ? { ...edge, data: nextData } : { ...edge, data: { ...edge.data, ...nextData } };
                }, options);
            },
            getNodesBounds: (nodes) => {
                const { nodeLookup, nodeOrigin } = store.getState();
                return getNodesBounds(nodes, { nodeLookup, nodeOrigin });
            },
            getHandleConnections: ({ type, id, nodeId }) => Array.from(store
                .getState()
                .connectionLookup.get(`${nodeId}-${type}${id ? `-${id}` : ''}`)
                ?.values() ?? []),
            getNodeConnections: ({ type, handleId, nodeId }) => Array.from(store
                .getState()
                .connectionLookup.get(`${nodeId}${type ? (handleId ? `-${type}-${handleId}` : `-${type}`) : ''}`)
                ?.values() ?? []),
            fitView: async (options) => {
                // We either create a new Promise or reuse the existing one
                // Even if fitView is called multiple times in a row, we only end up with a single Promise
                const fitViewResolver = store.getState().fitViewResolver ?? withResolvers();
                // We schedule a fitView by setting fitViewQueued and triggering a setNodes
                store.setState({ fitViewQueued: true, fitViewOptions: options, fitViewResolver });
                batchContext.nodeQueue.push((nodes) => [...nodes]);
                return fitViewResolver.promise;
            },
        };
    }, []);
    return useMemo(() => {
        return {
            ...generalHelper,
            ...viewportHelper,
            viewportInitialized,
        };
    }, [viewportInitialized]);
}

const selected = (item) => item.selected;
const win$1 = typeof window !== 'undefined' ? window : undefined;
/**
 * Hook for handling global key events.
 *
 * @internal
 */
function useGlobalKeyHandler({ deleteKeyCode, multiSelectionKeyCode, }) {
    const store = useStoreApi();
    const { deleteElements } = useReactFlow();
    const deleteKeyPressed = useKeyPress(deleteKeyCode, { actInsideInputWithModifier: false });
    const multiSelectionKeyPressed = useKeyPress(multiSelectionKeyCode, { target: win$1 });
    useEffect(() => {
        if (deleteKeyPressed) {
            const { edges, nodes } = store.getState();
            deleteElements({ nodes: nodes.filter(selected), edges: edges.filter(selected) });
            store.setState({ nodesSelectionActive: false });
        }
    }, [deleteKeyPressed]);
    useEffect(() => {
        store.setState({ multiSelectionActive: multiSelectionKeyPressed });
    }, [multiSelectionKeyPressed]);
}

/**
 * Hook for handling resize events.
 *
 * @internal
 */
function useResizeHandler(domNode) {
    const store = useStoreApi();
    useEffect(() => {
        const updateDimensions = () => {
            if (!domNode.current || !(domNode.current.checkVisibility?.() ?? true)) {
                return false;
            }
            const size = getDimensions(domNode.current);
            if (size.height === 0 || size.width === 0) {
                store.getState().onError?.('004', errorMessages['error004']());
            }
            store.setState({ width: size.width || 500, height: size.height || 500 });
        };
        if (domNode.current) {
            updateDimensions();
            window.addEventListener('resize', updateDimensions);
            const resizeObserver = new ResizeObserver(() => updateDimensions());
            resizeObserver.observe(domNode.current);
            return () => {
                window.removeEventListener('resize', updateDimensions);
                if (resizeObserver && domNode.current) {
                    resizeObserver.unobserve(domNode.current);
                }
            };
        }
    }, []);
}

const containerStyle = {
    position: 'absolute',
    width: '100%',
    height: '100%',
    top: 0,
    left: 0,
};

const selector$j = (s) => ({
    userSelectionActive: s.userSelectionActive,
    lib: s.lib,
    connectionInProgress: s.connection.inProgress,
});
function ZoomPane({ onPaneContextMenu, zoomOnScroll = true, zoomOnPinch = true, panOnScroll = false, panOnScrollSpeed = 0.5, panOnScrollMode = PanOnScrollMode.Free, zoomOnDoubleClick = true, panOnDrag = true, defaultViewport, translateExtent, minZoom, maxZoom, zoomActivationKeyCode, preventScrolling = true, children, noWheelClassName, noPanClassName, onViewportChange, isControlledViewport, paneClickDistance, selectionOnDrag, }) {
    const store = useStoreApi();
    const zoomPane = useRef(null);
    const { userSelectionActive, lib, connectionInProgress } = useStore$1(selector$j, shallow$1);
    const zoomActivationKeyPressed = useKeyPress(zoomActivationKeyCode);
    const panZoom = useRef();
    useResizeHandler(zoomPane);
    const onTransformChange = useCallback((transform) => {
        onViewportChange?.({ x: transform[0], y: transform[1], zoom: transform[2] });
        if (!isControlledViewport) {
            store.setState({ transform });
        }
    }, [onViewportChange, isControlledViewport]);
    useEffect(() => {
        if (zoomPane.current) {
            panZoom.current = XYPanZoom({
                domNode: zoomPane.current,
                minZoom,
                maxZoom,
                translateExtent,
                viewport: defaultViewport,
                onDraggingChange: (paneDragging) => store.setState((prevState) => prevState.paneDragging === paneDragging ? prevState : { paneDragging }),
                onPanZoomStart: (event, vp) => {
                    const { onViewportChangeStart, onMoveStart } = store.getState();
                    onMoveStart?.(event, vp);
                    onViewportChangeStart?.(vp);
                },
                onPanZoom: (event, vp) => {
                    const { onViewportChange, onMove } = store.getState();
                    onMove?.(event, vp);
                    onViewportChange?.(vp);
                },
                onPanZoomEnd: (event, vp) => {
                    const { onViewportChangeEnd, onMoveEnd } = store.getState();
                    onMoveEnd?.(event, vp);
                    onViewportChangeEnd?.(vp);
                },
            });
            const { x, y, zoom } = panZoom.current.getViewport();
            store.setState({
                panZoom: panZoom.current,
                transform: [x, y, zoom],
                domNode: zoomPane.current.closest('.react-flow'),
            });
            return () => {
                panZoom.current?.destroy();
            };
        }
    }, []);
    useEffect(() => {
        panZoom.current?.update({
            onPaneContextMenu,
            zoomOnScroll,
            zoomOnPinch,
            panOnScroll,
            panOnScrollSpeed,
            panOnScrollMode,
            zoomOnDoubleClick,
            panOnDrag,
            zoomActivationKeyPressed,
            preventScrolling,
            noPanClassName,
            userSelectionActive,
            noWheelClassName,
            lib,
            onTransformChange,
            connectionInProgress,
            selectionOnDrag,
            paneClickDistance,
        });
    }, [
        onPaneContextMenu,
        zoomOnScroll,
        zoomOnPinch,
        panOnScroll,
        panOnScrollSpeed,
        panOnScrollMode,
        zoomOnDoubleClick,
        panOnDrag,
        zoomActivationKeyPressed,
        preventScrolling,
        noPanClassName,
        userSelectionActive,
        noWheelClassName,
        lib,
        onTransformChange,
        connectionInProgress,
        selectionOnDrag,
        paneClickDistance,
    ]);
    return (jsx("div", { className: "react-flow__renderer", ref: zoomPane, style: containerStyle, children: children }));
}

const selector$i = (s) => ({
    userSelectionActive: s.userSelectionActive,
    userSelectionRect: s.userSelectionRect,
});
function UserSelection() {
    const { userSelectionActive, userSelectionRect } = useStore$1(selector$i, shallow$1);
    const isActive = userSelectionActive && userSelectionRect;
    if (!isActive) {
        return null;
    }
    return (jsx("div", { className: "react-flow__selection react-flow__container", style: {
            width: userSelectionRect.width,
            height: userSelectionRect.height,
            transform: `translate(${userSelectionRect.x}px, ${userSelectionRect.y}px)`,
        } }));
}

const wrapHandler = (handler, containerRef) => {
    return (event) => {
        if (event.target !== containerRef.current) {
            return;
        }
        handler?.(event);
    };
};
const selector$h = (s) => ({
    userSelectionActive: s.userSelectionActive,
    elementsSelectable: s.elementsSelectable,
    connectionInProgress: s.connection.inProgress,
    dragging: s.paneDragging,
});
function Pane({ isSelecting, selectionKeyPressed, selectionMode = SelectionMode.Full, panOnDrag, paneClickDistance, selectionOnDrag, onSelectionStart, onSelectionEnd, onPaneClick, onPaneContextMenu, onPaneScroll, onPaneMouseEnter, onPaneMouseMove, onPaneMouseLeave, children, }) {
    const store = useStoreApi();
    const { userSelectionActive, elementsSelectable, dragging, connectionInProgress } = useStore$1(selector$h, shallow$1);
    const isSelectionEnabled = elementsSelectable && (isSelecting || userSelectionActive);
    const container = useRef(null);
    const containerBounds = useRef();
    const selectedNodeIds = useRef(new Set());
    const selectedEdgeIds = useRef(new Set());
    // Used to prevent click events when the user lets go of the selectionKey during a selection
    const selectionInProgress = useRef(false);
    const onClick = (event) => {
        // We prevent click events when the user let go of the selectionKey during a selection
        // We also prevent click events when a connection is in progress
        if (selectionInProgress.current || connectionInProgress) {
            selectionInProgress.current = false;
            return;
        }
        onPaneClick?.(event);
        store.getState().resetSelectedElements();
        store.setState({ nodesSelectionActive: false });
    };
    const onContextMenu = (event) => {
        if (Array.isArray(panOnDrag) && panOnDrag?.includes(2)) {
            event.preventDefault();
            return;
        }
        onPaneContextMenu?.(event);
    };
    const onWheel = onPaneScroll ? (event) => onPaneScroll(event) : undefined;
    const onClickCapture = (event) => {
        if (selectionInProgress.current) {
            event.stopPropagation();
            selectionInProgress.current = false;
        }
    };
    // We are using capture here in order to prevent other pointer events
    // to be able to create a selection above a node or an edge
    const onPointerDownCapture = (event) => {
        const { domNode } = store.getState();
        containerBounds.current = domNode?.getBoundingClientRect();
        if (!containerBounds.current)
            return;
        const eventTargetIsContainer = event.target === container.current;
        // if a child element has the 'nokey' class, we don't want to swallow the event and don't start a selection
        const isNoKeyEvent = !eventTargetIsContainer && !!event.target.closest('.nokey');
        const isSelectionActive = (selectionOnDrag && eventTargetIsContainer) || selectionKeyPressed;
        if (isNoKeyEvent || !isSelecting || !isSelectionActive || event.button !== 0 || !event.isPrimary) {
            return;
        }
        event.target?.setPointerCapture?.(event.pointerId);
        selectionInProgress.current = false;
        const { x, y } = getEventPosition(event.nativeEvent, containerBounds.current);
        store.setState({
            userSelectionRect: {
                width: 0,
                height: 0,
                startX: x,
                startY: y,
                x,
                y,
            },
        });
        if (!eventTargetIsContainer) {
            event.stopPropagation();
            event.preventDefault();
        }
    };
    const onPointerMove = (event) => {
        const { userSelectionRect, transform, nodeLookup, edgeLookup, connectionLookup, triggerNodeChanges, triggerEdgeChanges, defaultEdgeOptions, resetSelectedElements, } = store.getState();
        if (!containerBounds.current || !userSelectionRect) {
            return;
        }
        const { x: mouseX, y: mouseY } = getEventPosition(event.nativeEvent, containerBounds.current);
        const { startX, startY } = userSelectionRect;
        if (!selectionInProgress.current) {
            const requiredDistance = selectionKeyPressed ? 0 : paneClickDistance;
            const distance = Math.hypot(mouseX - startX, mouseY - startY);
            if (distance <= requiredDistance) {
                return;
            }
            resetSelectedElements();
            onSelectionStart?.(event);
        }
        selectionInProgress.current = true;
        const nextUserSelectRect = {
            startX,
            startY,
            x: mouseX < startX ? mouseX : startX,
            y: mouseY < startY ? mouseY : startY,
            width: Math.abs(mouseX - startX),
            height: Math.abs(mouseY - startY),
        };
        const prevSelectedNodeIds = selectedNodeIds.current;
        const prevSelectedEdgeIds = selectedEdgeIds.current;
        selectedNodeIds.current = new Set(getNodesInside(nodeLookup, nextUserSelectRect, transform, selectionMode === SelectionMode.Partial, true).map((node) => node.id));
        selectedEdgeIds.current = new Set();
        const edgesSelectable = defaultEdgeOptions?.selectable ?? true;
        // We look for all edges connected to the selected nodes
        for (const nodeId of selectedNodeIds.current) {
            const connections = connectionLookup.get(nodeId);
            if (!connections)
                continue;
            for (const { edgeId } of connections.values()) {
                const edge = edgeLookup.get(edgeId);
                if (edge && (edge.selectable ?? edgesSelectable)) {
                    selectedEdgeIds.current.add(edgeId);
                }
            }
        }
        if (!areSetsEqual(prevSelectedNodeIds, selectedNodeIds.current)) {
            const changes = getSelectionChanges(nodeLookup, selectedNodeIds.current, true);
            triggerNodeChanges(changes);
        }
        if (!areSetsEqual(prevSelectedEdgeIds, selectedEdgeIds.current)) {
            const changes = getSelectionChanges(edgeLookup, selectedEdgeIds.current);
            triggerEdgeChanges(changes);
        }
        store.setState({
            userSelectionRect: nextUserSelectRect,
            userSelectionActive: true,
            nodesSelectionActive: false,
        });
    };
    const onPointerUp = (event) => {
        if (event.button !== 0) {
            return;
        }
        event.target?.releasePointerCapture?.(event.pointerId);
        /*
         * We only want to trigger click functions when in selection mode if
         * the user did not move the mouse.
         */
        if (!userSelectionActive && event.target === container.current && store.getState().userSelectionRect) {
            onClick?.(event);
        }
        store.setState({
            userSelectionActive: false,
            userSelectionRect: null,
        });
        if (selectionInProgress.current) {
            onSelectionEnd?.(event);
            store.setState({
                nodesSelectionActive: selectedNodeIds.current.size > 0,
            });
        }
    };
    const draggable = panOnDrag === true || (Array.isArray(panOnDrag) && panOnDrag.includes(0));
    return (jsxs("div", { className: cc(['react-flow__pane', { draggable, dragging, selection: isSelecting }]), onClick: isSelectionEnabled ? undefined : wrapHandler(onClick, container), onContextMenu: wrapHandler(onContextMenu, container), onWheel: wrapHandler(onWheel, container), onPointerEnter: isSelectionEnabled ? undefined : onPaneMouseEnter, onPointerMove: isSelectionEnabled ? onPointerMove : onPaneMouseMove, onPointerUp: isSelectionEnabled ? onPointerUp : undefined, onPointerDownCapture: isSelectionEnabled ? onPointerDownCapture : undefined, onClickCapture: isSelectionEnabled ? onClickCapture : undefined, onPointerLeave: onPaneMouseLeave, ref: container, style: containerStyle, children: [children, jsx(UserSelection, {})] }));
}

/*
 * this handler is called by
 * 1. the click handler when node is not draggable or selectNodesOnDrag = false
 * or
 * 2. the on drag start handler when node is draggable and selectNodesOnDrag = true
 */
function handleNodeClick({ id, store, unselect = false, nodeRef, }) {
    const { addSelectedNodes, unselectNodesAndEdges, multiSelectionActive, nodeLookup, onError } = store.getState();
    const node = nodeLookup.get(id);
    if (!node) {
        onError?.('012', errorMessages['error012'](id));
        return;
    }
    store.setState({ nodesSelectionActive: false });
    if (!node.selected) {
        addSelectedNodes([id]);
    }
    else if (unselect || (node.selected && multiSelectionActive)) {
        unselectNodesAndEdges({ nodes: [node], edges: [] });
        requestAnimationFrame(() => nodeRef?.current?.blur());
    }
}

/**
 * Hook for calling XYDrag helper from @xyflow/system.
 *
 * @internal
 */
function useDrag({ nodeRef, disabled = false, noDragClassName, handleSelector, nodeId, isSelectable, nodeClickDistance, }) {
    const store = useStoreApi();
    const [dragging, setDragging] = useState(false);
    const xyDrag = useRef();
    useEffect(() => {
        xyDrag.current = XYDrag({
            getStoreItems: () => store.getState(),
            onNodeMouseDown: (id) => {
                handleNodeClick({
                    id,
                    store,
                    nodeRef,
                });
            },
            onDragStart: () => {
                setDragging(true);
            },
            onDragStop: () => {
                setDragging(false);
            },
        });
    }, []);
    useEffect(() => {
        if (disabled || !nodeRef.current || !xyDrag.current) {
            return;
        }
        xyDrag.current.update({
            noDragClassName,
            handleSelector,
            domNode: nodeRef.current,
            isSelectable,
            nodeId,
            nodeClickDistance,
        });
        return () => {
            xyDrag.current?.destroy();
        };
    }, [noDragClassName, handleSelector, disabled, isSelectable, nodeRef, nodeId, nodeClickDistance]);
    return dragging;
}

const selectedAndDraggable = (nodesDraggable) => (n) => n.selected && (n.draggable || (nodesDraggable && typeof n.draggable === 'undefined'));
/**
 * Hook for updating node positions by passing a direction and factor
 *
 * @internal
 * @returns function for updating node positions
 */
function useMoveSelectedNodes() {
    const store = useStoreApi();
    const moveSelectedNodes = useCallback((params) => {
        const { nodeExtent, snapToGrid, snapGrid, nodesDraggable, onError, updateNodePositions, nodeLookup, nodeOrigin } = store.getState();
        const nodeUpdates = new Map();
        const isSelected = selectedAndDraggable(nodesDraggable);
        /*
         * by default a node moves 5px on each key press
         * if snap grid is enabled, we use that for the velocity
         */
        const xVelo = snapToGrid ? snapGrid[0] : 5;
        const yVelo = snapToGrid ? snapGrid[1] : 5;
        const xDiff = params.direction.x * xVelo * params.factor;
        const yDiff = params.direction.y * yVelo * params.factor;
        for (const [, node] of nodeLookup) {
            if (!isSelected(node)) {
                continue;
            }
            let nextPosition = {
                x: node.internals.positionAbsolute.x + xDiff,
                y: node.internals.positionAbsolute.y + yDiff,
            };
            if (snapToGrid) {
                nextPosition = snapPosition(nextPosition, snapGrid);
            }
            const { position, positionAbsolute } = calculateNodePosition({
                nodeId: node.id,
                nextPosition,
                nodeLookup,
                nodeExtent,
                nodeOrigin,
                onError,
            });
            node.position = position;
            node.internals.positionAbsolute = positionAbsolute;
            nodeUpdates.set(node.id, node);
        }
        updateNodePositions(nodeUpdates);
    }, []);
    return moveSelectedNodes;
}

const NodeIdContext = createContext(null);
const Provider = NodeIdContext.Provider;
NodeIdContext.Consumer;
/**
 * You can use this hook to get the id of the node it is used inside. It is useful
 * if you need the node's id deeper in the render tree but don't want to manually
 * drill down the id as a prop.
 *
 * @public
 * @returns The id for a node in the flow.
 *
 * @example
 *```jsx
 *import { useNodeId } from '@xyflow/react';
 *
 *export default function CustomNode() {
 *  return (
 *    <div>
 *      <span>This node has an id of </span>
 *      <NodeIdDisplay />
 *    </div>
 *  );
 *}
 *
 *function NodeIdDisplay() {
 *  const nodeId = useNodeId();
 *
 *  return <span>{nodeId}</span>;
 *}
 *```
 */
const useNodeId = () => {
    const nodeId = useContext(NodeIdContext);
    return nodeId;
};

const selector$g = (s) => ({
    connectOnClick: s.connectOnClick,
    noPanClassName: s.noPanClassName,
    rfId: s.rfId,
});
const connectingSelector = (nodeId, handleId, type) => (state) => {
    const { connectionClickStartHandle: clickHandle, connectionMode, connection } = state;
    const { fromHandle, toHandle, isValid } = connection;
    const connectingTo = toHandle?.nodeId === nodeId && toHandle?.id === handleId && toHandle?.type === type;
    return {
        connectingFrom: fromHandle?.nodeId === nodeId && fromHandle?.id === handleId && fromHandle?.type === type,
        connectingTo,
        clickConnecting: clickHandle?.nodeId === nodeId && clickHandle?.id === handleId && clickHandle?.type === type,
        isPossibleEndHandle: connectionMode === ConnectionMode.Strict
            ? fromHandle?.type !== type
            : nodeId !== fromHandle?.nodeId || handleId !== fromHandle?.id,
        connectionInProcess: !!fromHandle,
        clickConnectionInProcess: !!clickHandle,
        valid: connectingTo && isValid,
    };
};
function HandleComponent({ type = 'source', position = Position.Top, isValidConnection, isConnectable = true, isConnectableStart = true, isConnectableEnd = true, id, onConnect, children, className, onMouseDown, onTouchStart, ...rest }, ref) {
    const handleId = id || null;
    const isTarget = type === 'target';
    const store = useStoreApi();
    const nodeId = useNodeId();
    const { connectOnClick, noPanClassName, rfId } = useStore$1(selector$g, shallow$1);
    const { connectingFrom, connectingTo, clickConnecting, isPossibleEndHandle, connectionInProcess, clickConnectionInProcess, valid, } = useStore$1(connectingSelector(nodeId, handleId, type), shallow$1);
    if (!nodeId) {
        store.getState().onError?.('010', errorMessages['error010']());
    }
    const onConnectExtended = (params) => {
        const { defaultEdgeOptions, onConnect: onConnectAction, hasDefaultEdges } = store.getState();
        const edgeParams = {
            ...defaultEdgeOptions,
            ...params,
        };
        if (hasDefaultEdges) {
            const { edges, setEdges } = store.getState();
            setEdges(addEdge$1(edgeParams, edges));
        }
        onConnectAction?.(edgeParams);
        onConnect?.(edgeParams);
    };
    const onPointerDown = (event) => {
        if (!nodeId) {
            return;
        }
        const isMouseTriggered = isMouseEvent(event.nativeEvent);
        if (isConnectableStart &&
            ((isMouseTriggered && event.button === 0) || !isMouseTriggered)) {
            const currentStore = store.getState();
            XYHandle.onPointerDown(event.nativeEvent, {
                handleDomNode: event.currentTarget,
                autoPanOnConnect: currentStore.autoPanOnConnect,
                connectionMode: currentStore.connectionMode,
                connectionRadius: currentStore.connectionRadius,
                domNode: currentStore.domNode,
                nodeLookup: currentStore.nodeLookup,
                lib: currentStore.lib,
                isTarget,
                handleId,
                nodeId,
                flowId: currentStore.rfId,
                panBy: currentStore.panBy,
                cancelConnection: currentStore.cancelConnection,
                onConnectStart: currentStore.onConnectStart,
                onConnectEnd: (...args) => store.getState().onConnectEnd?.(...args),
                updateConnection: currentStore.updateConnection,
                onConnect: onConnectExtended,
                isValidConnection: isValidConnection || ((...args) => store.getState().isValidConnection?.(...args) ?? true),
                getTransform: () => store.getState().transform,
                getFromHandle: () => store.getState().connection.fromHandle,
                autoPanSpeed: currentStore.autoPanSpeed,
                dragThreshold: currentStore.connectionDragThreshold,
            });
        }
        if (isMouseTriggered) {
            onMouseDown?.(event);
        }
        else {
            onTouchStart?.(event);
        }
    };
    const onClick = (event) => {
        const { onClickConnectStart, onClickConnectEnd, connectionClickStartHandle, connectionMode, isValidConnection: isValidConnectionStore, lib, rfId: flowId, nodeLookup, connection: connectionState, } = store.getState();
        if (!nodeId || (!connectionClickStartHandle && !isConnectableStart)) {
            return;
        }
        if (!connectionClickStartHandle) {
            onClickConnectStart?.(event.nativeEvent, { nodeId, handleId, handleType: type });
            store.setState({ connectionClickStartHandle: { nodeId, type, id: handleId } });
            return;
        }
        const doc = getHostForElement(event.target);
        const isValidConnectionHandler = isValidConnection || isValidConnectionStore;
        const { connection, isValid } = XYHandle.isValid(event.nativeEvent, {
            handle: {
                nodeId,
                id: handleId,
                type,
            },
            connectionMode,
            fromNodeId: connectionClickStartHandle.nodeId,
            fromHandleId: connectionClickStartHandle.id || null,
            fromType: connectionClickStartHandle.type,
            isValidConnection: isValidConnectionHandler,
            flowId,
            doc,
            lib,
            nodeLookup,
        });
        if (isValid && connection) {
            onConnectExtended(connection);
        }
        const connectionClone = structuredClone(connectionState);
        delete connectionClone.inProgress;
        connectionClone.toPosition = connectionClone.toHandle ? connectionClone.toHandle.position : null;
        onClickConnectEnd?.(event, connectionClone);
        store.setState({ connectionClickStartHandle: null });
    };
    return (jsx("div", { "data-handleid": handleId, "data-nodeid": nodeId, "data-handlepos": position, "data-id": `${rfId}-${nodeId}-${handleId}-${type}`, className: cc([
            'react-flow__handle',
            `react-flow__handle-${position}`,
            'nodrag',
            noPanClassName,
            className,
            {
                source: !isTarget,
                target: isTarget,
                connectable: isConnectable,
                connectablestart: isConnectableStart,
                connectableend: isConnectableEnd,
                clickconnecting: clickConnecting,
                connectingfrom: connectingFrom,
                connectingto: connectingTo,
                valid,
                /*
                 * shows where you can start a connection from
                 * and where you can end it while connecting
                 */
                connectionindicator: isConnectable &&
                    (!connectionInProcess || isPossibleEndHandle) &&
                    (connectionInProcess || clickConnectionInProcess ? isConnectableEnd : isConnectableStart),
            },
        ]), onMouseDown: onPointerDown, onTouchStart: onPointerDown, onClick: connectOnClick ? onClick : undefined, ref: ref, ...rest, children: children }));
}
/**
 * The `<Handle />` component is used in your [custom nodes](/learn/customization/custom-nodes)
 * to define connection points.
 *
 *@public
 *
 *@example
 *
 *```jsx
 *import { Handle, Position } from '@xyflow/react';
 *
 *export function CustomNode({ data }) {
 *  return (
 *    <>
 *      <div style={{ padding: '10px 20px' }}>
 *        {data.label}
 *      </div>
 *
 *      <Handle type="target" position={Position.Left} />
 *      <Handle type="source" position={Position.Right} />
 *    </>
 *  );
 *};
 *```
 */
const Handle = memo(fixedForwardRef(HandleComponent));

function InputNode({ data, isConnectable, sourcePosition = Position.Bottom }) {
    return (jsxs(Fragment, { children: [data?.label, jsx(Handle, { type: "source", position: sourcePosition, isConnectable: isConnectable })] }));
}

function DefaultNode({ data, isConnectable, targetPosition = Position.Top, sourcePosition = Position.Bottom, }) {
    return (jsxs(Fragment, { children: [jsx(Handle, { type: "target", position: targetPosition, isConnectable: isConnectable }), data?.label, jsx(Handle, { type: "source", position: sourcePosition, isConnectable: isConnectable })] }));
}

function GroupNode() {
    return null;
}

function OutputNode({ data, isConnectable, targetPosition = Position.Top }) {
    return (jsxs(Fragment, { children: [jsx(Handle, { type: "target", position: targetPosition, isConnectable: isConnectable }), data?.label] }));
}

const arrowKeyDiffs = {
    ArrowUp: { x: 0, y: -1 },
    ArrowDown: { x: 0, y: 1 },
    ArrowLeft: { x: -1, y: 0 },
    ArrowRight: { x: 1, y: 0 },
};
const builtinNodeTypes = {
    input: InputNode,
    default: DefaultNode,
    output: OutputNode,
    group: GroupNode,
};
function getNodeInlineStyleDimensions(node) {
    if (node.internals.handleBounds === undefined) {
        return {
            width: node.width ?? node.initialWidth ?? node.style?.width,
            height: node.height ?? node.initialHeight ?? node.style?.height,
        };
    }
    return {
        width: node.width ?? node.style?.width,
        height: node.height ?? node.style?.height,
    };
}

const selector$f = (s) => {
    const { width, height, x, y } = getInternalNodesBounds(s.nodeLookup, {
        filter: (node) => !!node.selected,
    });
    return {
        width: isNumeric(width) ? width : null,
        height: isNumeric(height) ? height : null,
        userSelectionActive: s.userSelectionActive,
        transformString: `translate(${s.transform[0]}px,${s.transform[1]}px) scale(${s.transform[2]}) translate(${x}px,${y}px)`,
    };
};
function NodesSelection({ onSelectionContextMenu, noPanClassName, disableKeyboardA11y, }) {
    const store = useStoreApi();
    const { width, height, transformString, userSelectionActive } = useStore$1(selector$f, shallow$1);
    const moveSelectedNodes = useMoveSelectedNodes();
    const nodeRef = useRef(null);
    useEffect(() => {
        if (!disableKeyboardA11y) {
            nodeRef.current?.focus({
                preventScroll: true,
            });
        }
    }, [disableKeyboardA11y]);
    const shouldRender = !userSelectionActive && width !== null && height !== null;
    useDrag({
        nodeRef,
        disabled: !shouldRender,
    });
    if (!shouldRender) {
        return null;
    }
    const onContextMenu = onSelectionContextMenu
        ? (event) => {
            const selectedNodes = store.getState().nodes.filter((n) => n.selected);
            onSelectionContextMenu(event, selectedNodes);
        }
        : undefined;
    const onKeyDown = (event) => {
        if (Object.prototype.hasOwnProperty.call(arrowKeyDiffs, event.key)) {
            event.preventDefault();
            moveSelectedNodes({
                direction: arrowKeyDiffs[event.key],
                factor: event.shiftKey ? 4 : 1,
            });
        }
    };
    return (jsx("div", { className: cc(['react-flow__nodesselection', 'react-flow__container', noPanClassName]), style: {
            transform: transformString,
        }, children: jsx("div", { ref: nodeRef, className: "react-flow__nodesselection-rect", onContextMenu: onContextMenu, tabIndex: disableKeyboardA11y ? undefined : -1, onKeyDown: disableKeyboardA11y ? undefined : onKeyDown, style: {
                width,
                height,
            } }) }));
}

const win = typeof window !== 'undefined' ? window : undefined;
const selector$e = (s) => {
    return { nodesSelectionActive: s.nodesSelectionActive, userSelectionActive: s.userSelectionActive };
};
function FlowRendererComponent({ children, onPaneClick, onPaneMouseEnter, onPaneMouseMove, onPaneMouseLeave, onPaneContextMenu, onPaneScroll, paneClickDistance, deleteKeyCode, selectionKeyCode, selectionOnDrag, selectionMode, onSelectionStart, onSelectionEnd, multiSelectionKeyCode, panActivationKeyCode, zoomActivationKeyCode, elementsSelectable, zoomOnScroll, zoomOnPinch, panOnScroll: _panOnScroll, panOnScrollSpeed, panOnScrollMode, zoomOnDoubleClick, panOnDrag: _panOnDrag, defaultViewport, translateExtent, minZoom, maxZoom, preventScrolling, onSelectionContextMenu, noWheelClassName, noPanClassName, disableKeyboardA11y, onViewportChange, isControlledViewport, }) {
    const { nodesSelectionActive, userSelectionActive } = useStore$1(selector$e, shallow$1);
    const selectionKeyPressed = useKeyPress(selectionKeyCode, { target: win });
    const panActivationKeyPressed = useKeyPress(panActivationKeyCode, { target: win });
    const panOnDrag = panActivationKeyPressed || _panOnDrag;
    const panOnScroll = panActivationKeyPressed || _panOnScroll;
    const _selectionOnDrag = selectionOnDrag && panOnDrag !== true;
    const isSelecting = selectionKeyPressed || userSelectionActive || _selectionOnDrag;
    useGlobalKeyHandler({ deleteKeyCode, multiSelectionKeyCode });
    return (jsx(ZoomPane, { onPaneContextMenu: onPaneContextMenu, elementsSelectable: elementsSelectable, zoomOnScroll: zoomOnScroll, zoomOnPinch: zoomOnPinch, panOnScroll: panOnScroll, panOnScrollSpeed: panOnScrollSpeed, panOnScrollMode: panOnScrollMode, zoomOnDoubleClick: zoomOnDoubleClick, panOnDrag: !selectionKeyPressed && panOnDrag, defaultViewport: defaultViewport, translateExtent: translateExtent, minZoom: minZoom, maxZoom: maxZoom, zoomActivationKeyCode: zoomActivationKeyCode, preventScrolling: preventScrolling, noWheelClassName: noWheelClassName, noPanClassName: noPanClassName, onViewportChange: onViewportChange, isControlledViewport: isControlledViewport, paneClickDistance: paneClickDistance, selectionOnDrag: _selectionOnDrag, children: jsxs(Pane, { onSelectionStart: onSelectionStart, onSelectionEnd: onSelectionEnd, onPaneClick: onPaneClick, onPaneMouseEnter: onPaneMouseEnter, onPaneMouseMove: onPaneMouseMove, onPaneMouseLeave: onPaneMouseLeave, onPaneContextMenu: onPaneContextMenu, onPaneScroll: onPaneScroll, panOnDrag: panOnDrag, isSelecting: !!isSelecting, selectionMode: selectionMode, selectionKeyPressed: selectionKeyPressed, paneClickDistance: paneClickDistance, selectionOnDrag: _selectionOnDrag, children: [children, nodesSelectionActive && (jsx(NodesSelection, { onSelectionContextMenu: onSelectionContextMenu, noPanClassName: noPanClassName, disableKeyboardA11y: disableKeyboardA11y }))] }) }));
}
FlowRendererComponent.displayName = 'FlowRenderer';
const FlowRenderer = memo(FlowRendererComponent);

const selector$d = (onlyRenderVisible) => (s) => {
    return onlyRenderVisible
        ? getNodesInside(s.nodeLookup, { x: 0, y: 0, width: s.width, height: s.height }, s.transform, true).map((node) => node.id)
        : Array.from(s.nodeLookup.keys());
};
/**
 * Hook for getting the visible node ids from the store.
 *
 * @internal
 * @param onlyRenderVisible
 * @returns array with visible node ids
 */
function useVisibleNodeIds(onlyRenderVisible) {
    const nodeIds = useStore$1(useCallback(selector$d(onlyRenderVisible), [onlyRenderVisible]), shallow$1);
    return nodeIds;
}

const selector$c = (s) => s.updateNodeInternals;
function useResizeObserver() {
    const updateNodeInternals = useStore$1(selector$c);
    const [resizeObserver] = useState(() => {
        if (typeof ResizeObserver === 'undefined') {
            return null;
        }
        return new ResizeObserver((entries) => {
            const updates = new Map();
            entries.forEach((entry) => {
                const id = entry.target.getAttribute('data-id');
                updates.set(id, {
                    id,
                    nodeElement: entry.target,
                    force: true,
                });
            });
            updateNodeInternals(updates);
        });
    });
    useEffect(() => {
        return () => {
            resizeObserver?.disconnect();
        };
    }, [resizeObserver]);
    return resizeObserver;
}

/**
 * Hook to handle the resize observation + internal updates for the passed node.
 *
 * @internal
 * @returns nodeRef - reference to the node element
 */
function useNodeObserver({ node, nodeType, hasDimensions, resizeObserver, }) {
    const store = useStoreApi();
    const nodeRef = useRef(null);
    const observedNode = useRef(null);
    const prevSourcePosition = useRef(node.sourcePosition);
    const prevTargetPosition = useRef(node.targetPosition);
    const prevType = useRef(nodeType);
    const isInitialized = hasDimensions && !!node.internals.handleBounds;
    useEffect(() => {
        if (nodeRef.current && !node.hidden && (!isInitialized || observedNode.current !== nodeRef.current)) {
            if (observedNode.current) {
                resizeObserver?.unobserve(observedNode.current);
            }
            resizeObserver?.observe(nodeRef.current);
            observedNode.current = nodeRef.current;
        }
    }, [isInitialized, node.hidden]);
    useEffect(() => {
        return () => {
            if (observedNode.current) {
                resizeObserver?.unobserve(observedNode.current);
                observedNode.current = null;
            }
        };
    }, []);
    useEffect(() => {
        if (nodeRef.current) {
            /*
             * when the user programmatically changes the source or handle position, we need to update the internals
             * to make sure the edges are updated correctly
             */
            const typeChanged = prevType.current !== nodeType;
            const sourcePosChanged = prevSourcePosition.current !== node.sourcePosition;
            const targetPosChanged = prevTargetPosition.current !== node.targetPosition;
            if (typeChanged || sourcePosChanged || targetPosChanged) {
                prevType.current = nodeType;
                prevSourcePosition.current = node.sourcePosition;
                prevTargetPosition.current = node.targetPosition;
                store
                    .getState()
                    .updateNodeInternals(new Map([[node.id, { id: node.id, nodeElement: nodeRef.current, force: true }]]));
            }
        }
    }, [node.id, nodeType, node.sourcePosition, node.targetPosition]);
    return nodeRef;
}

function NodeWrapper({ id, onClick, onMouseEnter, onMouseMove, onMouseLeave, onContextMenu, onDoubleClick, nodesDraggable, elementsSelectable, nodesConnectable, nodesFocusable, resizeObserver, noDragClassName, noPanClassName, disableKeyboardA11y, rfId, nodeTypes, nodeClickDistance, onError, }) {
    const { node, internals, isParent } = useStore$1((s) => {
        const node = s.nodeLookup.get(id);
        const isParent = s.parentLookup.has(id);
        return {
            node,
            internals: node.internals,
            isParent,
        };
    }, shallow$1);
    let nodeType = node.type || 'default';
    let NodeComponent = nodeTypes?.[nodeType] || builtinNodeTypes[nodeType];
    if (NodeComponent === undefined) {
        onError?.('003', errorMessages['error003'](nodeType));
        nodeType = 'default';
        NodeComponent = nodeTypes?.['default'] || builtinNodeTypes.default;
    }
    const isDraggable = !!(node.draggable || (nodesDraggable && typeof node.draggable === 'undefined'));
    const isSelectable = !!(node.selectable || (elementsSelectable && typeof node.selectable === 'undefined'));
    const isConnectable = !!(node.connectable || (nodesConnectable && typeof node.connectable === 'undefined'));
    const isFocusable = !!(node.focusable || (nodesFocusable && typeof node.focusable === 'undefined'));
    const store = useStoreApi();
    const hasDimensions = nodeHasDimensions(node);
    const nodeRef = useNodeObserver({ node, nodeType, hasDimensions, resizeObserver });
    const dragging = useDrag({
        nodeRef,
        disabled: node.hidden || !isDraggable,
        noDragClassName,
        handleSelector: node.dragHandle,
        nodeId: id,
        isSelectable,
        nodeClickDistance,
    });
    const moveSelectedNodes = useMoveSelectedNodes();
    if (node.hidden) {
        return null;
    }
    const nodeDimensions = getNodeDimensions(node);
    const inlineDimensions = getNodeInlineStyleDimensions(node);
    const hasPointerEvents = isSelectable || isDraggable || onClick || onMouseEnter || onMouseMove || onMouseLeave;
    const onMouseEnterHandler = onMouseEnter
        ? (event) => onMouseEnter(event, { ...internals.userNode })
        : undefined;
    const onMouseMoveHandler = onMouseMove
        ? (event) => onMouseMove(event, { ...internals.userNode })
        : undefined;
    const onMouseLeaveHandler = onMouseLeave
        ? (event) => onMouseLeave(event, { ...internals.userNode })
        : undefined;
    const onContextMenuHandler = onContextMenu
        ? (event) => onContextMenu(event, { ...internals.userNode })
        : undefined;
    const onDoubleClickHandler = onDoubleClick
        ? (event) => onDoubleClick(event, { ...internals.userNode })
        : undefined;
    const onSelectNodeHandler = (event) => {
        const { selectNodesOnDrag, nodeDragThreshold } = store.getState();
        if (isSelectable && (!selectNodesOnDrag || !isDraggable || nodeDragThreshold > 0)) {
            /*
             * this handler gets called by XYDrag on drag start when selectNodesOnDrag=true
             * here we only need to call it when selectNodesOnDrag=false
             */
            handleNodeClick({
                id,
                store,
                nodeRef,
            });
        }
        if (onClick) {
            onClick(event, { ...internals.userNode });
        }
    };
    const onKeyDown = (event) => {
        if (isInputDOMNode(event.nativeEvent) || disableKeyboardA11y) {
            return;
        }
        if (elementSelectionKeys.includes(event.key) && isSelectable) {
            const unselect = event.key === 'Escape';
            handleNodeClick({
                id,
                store,
                unselect,
                nodeRef,
            });
        }
        else if (isDraggable && node.selected && Object.prototype.hasOwnProperty.call(arrowKeyDiffs, event.key)) {
            // prevent default scrolling behavior on arrow key press when node is moved
            event.preventDefault();
            const { ariaLabelConfig } = store.getState();
            store.setState({
                ariaLiveMessage: ariaLabelConfig['node.a11yDescription.ariaLiveMessage']({
                    direction: event.key.replace('Arrow', '').toLowerCase(),
                    x: ~~internals.positionAbsolute.x,
                    y: ~~internals.positionAbsolute.y,
                }),
            });
            moveSelectedNodes({
                direction: arrowKeyDiffs[event.key],
                factor: event.shiftKey ? 4 : 1,
            });
        }
    };
    const onFocus = () => {
        if (disableKeyboardA11y || !nodeRef.current?.matches(':focus-visible')) {
            return;
        }
        const { transform, width, height, autoPanOnNodeFocus, setCenter } = store.getState();
        if (!autoPanOnNodeFocus) {
            return;
        }
        const withinViewport = getNodesInside(new Map([[id, node]]), { x: 0, y: 0, width, height }, transform, true).length > 0;
        if (!withinViewport) {
            setCenter(node.position.x + nodeDimensions.width / 2, node.position.y + nodeDimensions.height / 2, {
                zoom: transform[2],
            });
        }
    };
    return (jsx("div", { className: cc([
            'react-flow__node',
            `react-flow__node-${nodeType}`,
            {
                // this is overwritable by passing `nopan` as a class name
                [noPanClassName]: isDraggable,
            },
            node.className,
            {
                selected: node.selected,
                selectable: isSelectable,
                parent: isParent,
                draggable: isDraggable,
                dragging,
            },
        ]), ref: nodeRef, style: {
            zIndex: internals.z,
            transform: `translate(${internals.positionAbsolute.x}px,${internals.positionAbsolute.y}px)`,
            pointerEvents: hasPointerEvents ? 'all' : 'none',
            visibility: hasDimensions ? 'visible' : 'hidden',
            ...node.style,
            ...inlineDimensions,
        }, "data-id": id, "data-testid": `rf__node-${id}`, onMouseEnter: onMouseEnterHandler, onMouseMove: onMouseMoveHandler, onMouseLeave: onMouseLeaveHandler, onContextMenu: onContextMenuHandler, onClick: onSelectNodeHandler, onDoubleClick: onDoubleClickHandler, onKeyDown: isFocusable ? onKeyDown : undefined, tabIndex: isFocusable ? 0 : undefined, onFocus: isFocusable ? onFocus : undefined, role: node.ariaRole ?? (isFocusable ? 'group' : undefined), "aria-roledescription": "node", "aria-describedby": disableKeyboardA11y ? undefined : `${ARIA_NODE_DESC_KEY}-${rfId}`, "aria-label": node.ariaLabel, ...node.domAttributes, children: jsx(Provider, { value: id, children: jsx(NodeComponent, { id: id, data: node.data, type: nodeType, positionAbsoluteX: internals.positionAbsolute.x, positionAbsoluteY: internals.positionAbsolute.y, selected: node.selected ?? false, selectable: isSelectable, draggable: isDraggable, deletable: node.deletable ?? true, isConnectable: isConnectable, sourcePosition: node.sourcePosition, targetPosition: node.targetPosition, dragging: dragging, dragHandle: node.dragHandle, zIndex: internals.z, parentId: node.parentId, ...nodeDimensions }) }) }));
}
var NodeWrapper$1 = memo(NodeWrapper);

const selector$b = (s) => ({
    nodesDraggable: s.nodesDraggable,
    nodesConnectable: s.nodesConnectable,
    nodesFocusable: s.nodesFocusable,
    elementsSelectable: s.elementsSelectable,
    onError: s.onError,
});
function NodeRendererComponent(props) {
    const { nodesDraggable, nodesConnectable, nodesFocusable, elementsSelectable, onError } = useStore$1(selector$b, shallow$1);
    const nodeIds = useVisibleNodeIds(props.onlyRenderVisibleElements);
    const resizeObserver = useResizeObserver();
    return (jsx("div", { className: "react-flow__nodes", style: containerStyle, children: nodeIds.map((nodeId) => {
            return (
            /*
             * The split of responsibilities between NodeRenderer and
             * NodeComponentWrapper may appear weird. However, it’s designed to
             * minimize the cost of updates when individual nodes change.
             *
             * For example, when you’re dragging a single node, that node gets
             * updated multiple times per second. If `NodeRenderer` were to update
             * every time, it would have to re-run the `nodes.map()` loop every
             * time. This gets pricey with hundreds of nodes, especially if every
             * loop cycle does more than just rendering a JSX element!
             *
             * As a result of this choice, we took the following implementation
             * decisions:
             * - NodeRenderer subscribes *only* to node IDs – and therefore
             *   rerender *only* when visible nodes are added or removed.
             * - NodeRenderer performs all operations the result of which can be
             *   shared between nodes (such as creating the `ResizeObserver`
             *   instance, or subscribing to `selector`). This means extra prop
             *   drilling into `NodeComponentWrapper`, but it means we need to run
             *   these operations only once – instead of once per node.
             * - Any operations that you’d normally write inside `nodes.map` are
             *   moved into `NodeComponentWrapper`. This ensures they are
             *   memorized – so if `NodeRenderer` *has* to rerender, it only
             *   needs to regenerate the list of nodes, nothing else.
             */
            jsx(NodeWrapper$1, { id: nodeId, nodeTypes: props.nodeTypes, nodeExtent: props.nodeExtent, onClick: props.onNodeClick, onMouseEnter: props.onNodeMouseEnter, onMouseMove: props.onNodeMouseMove, onMouseLeave: props.onNodeMouseLeave, onContextMenu: props.onNodeContextMenu, onDoubleClick: props.onNodeDoubleClick, noDragClassName: props.noDragClassName, noPanClassName: props.noPanClassName, rfId: props.rfId, disableKeyboardA11y: props.disableKeyboardA11y, resizeObserver: resizeObserver, nodesDraggable: nodesDraggable, nodesConnectable: nodesConnectable, nodesFocusable: nodesFocusable, elementsSelectable: elementsSelectable, nodeClickDistance: props.nodeClickDistance, onError: onError }, nodeId));
        }) }));
}
NodeRendererComponent.displayName = 'NodeRenderer';
const NodeRenderer = memo(NodeRendererComponent);

/**
 * Hook for getting the visible edge ids from the store.
 *
 * @internal
 * @param onlyRenderVisible
 * @returns array with visible edge ids
 */
function useVisibleEdgeIds(onlyRenderVisible) {
    const edgeIds = useStore$1(useCallback((s) => {
        if (!onlyRenderVisible) {
            return s.edges.map((edge) => edge.id);
        }
        const visibleEdgeIds = [];
        if (s.width && s.height) {
            for (const edge of s.edges) {
                const sourceNode = s.nodeLookup.get(edge.source);
                const targetNode = s.nodeLookup.get(edge.target);
                if (sourceNode &&
                    targetNode &&
                    isEdgeVisible({
                        sourceNode,
                        targetNode,
                        width: s.width,
                        height: s.height,
                        transform: s.transform,
                    })) {
                    visibleEdgeIds.push(edge.id);
                }
            }
        }
        return visibleEdgeIds;
    }, [onlyRenderVisible]), shallow$1);
    return edgeIds;
}

const ArrowSymbol = ({ color = 'none', strokeWidth = 1 }) => {
    const style = {
        strokeWidth,
        ...(color && { stroke: color }),
    };
    return (jsx("polyline", { className: "arrow", style: style, strokeLinecap: "round", fill: "none", strokeLinejoin: "round", points: "-5,-4 0,0 -5,4" }));
};
const ArrowClosedSymbol = ({ color = 'none', strokeWidth = 1 }) => {
    const style = {
        strokeWidth,
        ...(color && { stroke: color, fill: color }),
    };
    return (jsx("polyline", { className: "arrowclosed", style: style, strokeLinecap: "round", strokeLinejoin: "round", points: "-5,-4 0,0 -5,4 -5,-4" }));
};
const MarkerSymbols = {
    [MarkerType.Arrow]: ArrowSymbol,
    [MarkerType.ArrowClosed]: ArrowClosedSymbol,
};
function useMarkerSymbol(type) {
    const store = useStoreApi();
    const symbol = useMemo(() => {
        const symbolExists = Object.prototype.hasOwnProperty.call(MarkerSymbols, type);
        if (!symbolExists) {
            store.getState().onError?.('009', errorMessages['error009'](type));
            return null;
        }
        return MarkerSymbols[type];
    }, [type]);
    return symbol;
}

const Marker = ({ id, type, color, width = 12.5, height = 12.5, markerUnits = 'strokeWidth', strokeWidth, orient = 'auto-start-reverse', }) => {
    const Symbol = useMarkerSymbol(type);
    if (!Symbol) {
        return null;
    }
    return (jsx("marker", { className: "react-flow__arrowhead", id: id, markerWidth: `${width}`, markerHeight: `${height}`, viewBox: "-10 -10 20 20", markerUnits: markerUnits, orient: orient, refX: "0", refY: "0", children: jsx(Symbol, { color: color, strokeWidth: strokeWidth }) }));
};
/*
 * when you have multiple flows on a page and you hide the first one, the other ones have no markers anymore
 * when they do have markers with the same ids. To prevent this the user can pass a unique id to the react flow wrapper
 * that we can then use for creating our unique marker ids
 */
const MarkerDefinitions = ({ defaultColor, rfId }) => {
    const edges = useStore$1((s) => s.edges);
    const defaultEdgeOptions = useStore$1((s) => s.defaultEdgeOptions);
    const markers = useMemo(() => {
        const markers = createMarkerIds(edges, {
            id: rfId,
            defaultColor,
            defaultMarkerStart: defaultEdgeOptions?.markerStart,
            defaultMarkerEnd: defaultEdgeOptions?.markerEnd,
        });
        return markers;
    }, [edges, defaultEdgeOptions, rfId, defaultColor]);
    if (!markers.length) {
        return null;
    }
    return (jsx("svg", { className: "react-flow__marker", "aria-hidden": "true", children: jsx("defs", { children: markers.map((marker) => (jsx(Marker, { id: marker.id, type: marker.type, color: marker.color, width: marker.width, height: marker.height, markerUnits: marker.markerUnits, strokeWidth: marker.strokeWidth, orient: marker.orient }, marker.id))) }) }));
};
MarkerDefinitions.displayName = 'MarkerDefinitions';
var MarkerDefinitions$1 = memo(MarkerDefinitions);

function EdgeTextComponent({ x, y, label, labelStyle, labelShowBg = true, labelBgStyle, labelBgPadding = [2, 4], labelBgBorderRadius = 2, children, className, ...rest }) {
    const [edgeTextBbox, setEdgeTextBbox] = useState({ x: 1, y: 0, width: 0, height: 0 });
    const edgeTextClasses = cc(['react-flow__edge-textwrapper', className]);
    const edgeTextRef = useRef(null);
    useEffect(() => {
        if (edgeTextRef.current) {
            const textBbox = edgeTextRef.current.getBBox();
            setEdgeTextBbox({
                x: textBbox.x,
                y: textBbox.y,
                width: textBbox.width,
                height: textBbox.height,
            });
        }
    }, [label]);
    if (!label) {
        return null;
    }
    return (jsxs("g", { transform: `translate(${x - edgeTextBbox.width / 2} ${y - edgeTextBbox.height / 2})`, className: edgeTextClasses, visibility: edgeTextBbox.width ? 'visible' : 'hidden', ...rest, children: [labelShowBg && (jsx("rect", { width: edgeTextBbox.width + 2 * labelBgPadding[0], x: -labelBgPadding[0], y: -labelBgPadding[1], height: edgeTextBbox.height + 2 * labelBgPadding[1], className: "react-flow__edge-textbg", style: labelBgStyle, rx: labelBgBorderRadius, ry: labelBgBorderRadius })), jsx("text", { className: "react-flow__edge-text", y: edgeTextBbox.height / 2, dy: "0.3em", ref: edgeTextRef, style: labelStyle, children: label }), children] }));
}
EdgeTextComponent.displayName = 'EdgeText';
/**
 * You can use the `<EdgeText />` component as a helper component to display text
 * within your custom edges.
 *
 * @public
 *
 * @example
 * ```jsx
 * import { EdgeText } from '@xyflow/react';
 *
 * export function CustomEdgeLabel({ label }) {
 *   return (
 *     <EdgeText
 *       x={100}
 *       y={100}
 *       label={label}
 *       labelStyle={{ fill: 'white' }}
 *       labelShowBg
 *       labelBgStyle={{ fill: 'red' }}
 *       labelBgPadding={[2, 4]}
 *       labelBgBorderRadius={2}
 *     />
 *   );
 * }
 *```
 */
const EdgeText = memo(EdgeTextComponent);

/**
 * The `<BaseEdge />` component gets used internally for all the edges. It can be
 * used inside a custom edge and handles the invisible helper edge and the edge label
 * for you.
 *
 * @public
 * @example
 * ```jsx
 *import { BaseEdge } from '@xyflow/react';
 *
 *export function CustomEdge({ sourceX, sourceY, targetX, targetY, ...props }) {
 *  const [edgePath] = getStraightPath({
 *    sourceX,
 *    sourceY,
 *    targetX,
 *    targetY,
 *  });
 *
 *  return <BaseEdge path={edgePath} {...props} />;
 *}
 *```
 *
 * @remarks If you want to use an edge marker with the [`<BaseEdge />`](/api-reference/components/base-edge) component,
 * you can pass the `markerStart` or `markerEnd` props passed to your custom edge
 * through to the [`<BaseEdge />`](/api-reference/components/base-edge) component.
 * You can see all the props passed to a custom edge by looking at the [`EdgeProps`](/api-reference/types/edge-props) type.
 */
function BaseEdge({ path, labelX, labelY, label, labelStyle, labelShowBg, labelBgStyle, labelBgPadding, labelBgBorderRadius, interactionWidth = 20, ...props }) {
    return (jsxs(Fragment, { children: [jsx("path", { ...props, d: path, fill: "none", className: cc(['react-flow__edge-path', props.className]) }), interactionWidth ? (jsx("path", { d: path, fill: "none", strokeOpacity: 0, strokeWidth: interactionWidth, className: "react-flow__edge-interaction" })) : null, label && isNumeric(labelX) && isNumeric(labelY) ? (jsx(EdgeText, { x: labelX, y: labelY, label: label, labelStyle: labelStyle, labelShowBg: labelShowBg, labelBgStyle: labelBgStyle, labelBgPadding: labelBgPadding, labelBgBorderRadius: labelBgBorderRadius })) : null] }));
}

function getControl({ pos, x1, y1, x2, y2 }) {
    if (pos === Position.Left || pos === Position.Right) {
        return [0.5 * (x1 + x2), y1];
    }
    return [x1, 0.5 * (y1 + y2)];
}
/**
 * The `getSimpleBezierPath` util returns everything you need to render a simple
 * bezier edge between two nodes.
 * @public
 * @returns
 * - `path`: the path to use in an SVG `<path>` element.
 * - `labelX`: the `x` position you can use to render a label for this edge.
 * - `labelY`: the `y` position you can use to render a label for this edge.
 * - `offsetX`: the absolute difference between the source `x` position and the `x` position of the
 * middle of this path.
 * - `offsetY`: the absolute difference between the source `y` position and the `y` position of the
 * middle of this path.
 */
function getSimpleBezierPath({ sourceX, sourceY, sourcePosition = Position.Bottom, targetX, targetY, targetPosition = Position.Top, }) {
    const [sourceControlX, sourceControlY] = getControl({
        pos: sourcePosition,
        x1: sourceX,
        y1: sourceY,
        x2: targetX,
        y2: targetY,
    });
    const [targetControlX, targetControlY] = getControl({
        pos: targetPosition,
        x1: targetX,
        y1: targetY,
        x2: sourceX,
        y2: sourceY,
    });
    const [labelX, labelY, offsetX, offsetY] = getBezierEdgeCenter({
        sourceX,
        sourceY,
        targetX,
        targetY,
        sourceControlX,
        sourceControlY,
        targetControlX,
        targetControlY,
    });
    return [
        `M${sourceX},${sourceY} C${sourceControlX},${sourceControlY} ${targetControlX},${targetControlY} ${targetX},${targetY}`,
        labelX,
        labelY,
        offsetX,
        offsetY,
    ];
}
function createSimpleBezierEdge(params) {
    // eslint-disable-next-line react/display-name
    return memo(({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, label, labelStyle, labelShowBg, labelBgStyle, labelBgPadding, labelBgBorderRadius, style, markerEnd, markerStart, interactionWidth, }) => {
        const [path, labelX, labelY] = getSimpleBezierPath({
            sourceX,
            sourceY,
            sourcePosition,
            targetX,
            targetY,
            targetPosition,
        });
        const _id = params.isInternal ? undefined : id;
        return (jsx(BaseEdge, { id: _id, path: path, labelX: labelX, labelY: labelY, label: label, labelStyle: labelStyle, labelShowBg: labelShowBg, labelBgStyle: labelBgStyle, labelBgPadding: labelBgPadding, labelBgBorderRadius: labelBgBorderRadius, style: style, markerEnd: markerEnd, markerStart: markerStart, interactionWidth: interactionWidth }));
    });
}
const SimpleBezierEdge = createSimpleBezierEdge({ isInternal: false });
const SimpleBezierEdgeInternal = createSimpleBezierEdge({ isInternal: true });
SimpleBezierEdge.displayName = 'SimpleBezierEdge';
SimpleBezierEdgeInternal.displayName = 'SimpleBezierEdgeInternal';

function createSmoothStepEdge(params) {
    // eslint-disable-next-line react/display-name
    return memo(({ id, sourceX, sourceY, targetX, targetY, label, labelStyle, labelShowBg, labelBgStyle, labelBgPadding, labelBgBorderRadius, style, sourcePosition = Position.Bottom, targetPosition = Position.Top, markerEnd, markerStart, pathOptions, interactionWidth, }) => {
        const [path, labelX, labelY] = getSmoothStepPath({
            sourceX,
            sourceY,
            sourcePosition,
            targetX,
            targetY,
            targetPosition,
            borderRadius: pathOptions?.borderRadius,
            offset: pathOptions?.offset,
            stepPosition: pathOptions?.stepPosition,
        });
        const _id = params.isInternal ? undefined : id;
        return (jsx(BaseEdge, { id: _id, path: path, labelX: labelX, labelY: labelY, label: label, labelStyle: labelStyle, labelShowBg: labelShowBg, labelBgStyle: labelBgStyle, labelBgPadding: labelBgPadding, labelBgBorderRadius: labelBgBorderRadius, style: style, markerEnd: markerEnd, markerStart: markerStart, interactionWidth: interactionWidth }));
    });
}
/**
 * Component that can be used inside a custom edge to render a smooth step edge.
 *
 * @public
 * @example
 *
 * ```tsx
 * import { SmoothStepEdge } from '@xyflow/react';
 *
 * function CustomEdge({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }) {
 *   return (
 *     <SmoothStepEdge
 *       sourceX={sourceX}
 *       sourceY={sourceY}
 *       targetX={targetX}
 *       targetY={targetY}
 *       sourcePosition={sourcePosition}
 *       targetPosition={targetPosition}
 *     />
 *   );
 * }
 * ```
 */
const SmoothStepEdge = createSmoothStepEdge({ isInternal: false });
/**
 * @internal
 */
const SmoothStepEdgeInternal = createSmoothStepEdge({ isInternal: true });
SmoothStepEdge.displayName = 'SmoothStepEdge';
SmoothStepEdgeInternal.displayName = 'SmoothStepEdgeInternal';

function createStepEdge(params) {
    // eslint-disable-next-line react/display-name
    return memo(({ id, ...props }) => {
        const _id = params.isInternal ? undefined : id;
        return (jsx(SmoothStepEdge, { ...props, id: _id, pathOptions: useMemo(() => ({ borderRadius: 0, offset: props.pathOptions?.offset }), [props.pathOptions?.offset]) }));
    });
}
/**
 * Component that can be used inside a custom edge to render a step edge.
 *
 * @public
 * @example
 *
 * ```tsx
 * import { StepEdge } from '@xyflow/react';
 *
 * function CustomEdge({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }) {
 *   return (
 *     <StepEdge
 *       sourceX={sourceX}
 *       sourceY={sourceY}
 *       targetX={targetX}
 *       targetY={targetY}
 *       sourcePosition={sourcePosition}
 *       targetPosition={targetPosition}
 *     />
 *   );
 * }
 * ```
 */
const StepEdge = createStepEdge({ isInternal: false });
/**
 * @internal
 */
const StepEdgeInternal = createStepEdge({ isInternal: true });
StepEdge.displayName = 'StepEdge';
StepEdgeInternal.displayName = 'StepEdgeInternal';

function createStraightEdge(params) {
    // eslint-disable-next-line react/display-name
    return memo(({ id, sourceX, sourceY, targetX, targetY, label, labelStyle, labelShowBg, labelBgStyle, labelBgPadding, labelBgBorderRadius, style, markerEnd, markerStart, interactionWidth, }) => {
        const [path, labelX, labelY] = getStraightPath({ sourceX, sourceY, targetX, targetY });
        const _id = params.isInternal ? undefined : id;
        return (jsx(BaseEdge, { id: _id, path: path, labelX: labelX, labelY: labelY, label: label, labelStyle: labelStyle, labelShowBg: labelShowBg, labelBgStyle: labelBgStyle, labelBgPadding: labelBgPadding, labelBgBorderRadius: labelBgBorderRadius, style: style, markerEnd: markerEnd, markerStart: markerStart, interactionWidth: interactionWidth }));
    });
}
/**
 * Component that can be used inside a custom edge to render a straight line.
 *
 * @public
 * @example
 *
 * ```tsx
 * import { StraightEdge } from '@xyflow/react';
 *
 * function CustomEdge({ sourceX, sourceY, targetX, targetY }) {
 *   return (
 *     <StraightEdge
 *       sourceX={sourceX}
 *       sourceY={sourceY}
 *       targetX={targetX}
 *       targetY={targetY}
 *     />
 *   );
 * }
 * ```
 */
const StraightEdge = createStraightEdge({ isInternal: false });
/**
 * @internal
 */
const StraightEdgeInternal = createStraightEdge({ isInternal: true });
StraightEdge.displayName = 'StraightEdge';
StraightEdgeInternal.displayName = 'StraightEdgeInternal';

function createBezierEdge(params) {
    // eslint-disable-next-line react/display-name
    return memo(({ id, sourceX, sourceY, targetX, targetY, sourcePosition = Position.Bottom, targetPosition = Position.Top, label, labelStyle, labelShowBg, labelBgStyle, labelBgPadding, labelBgBorderRadius, style, markerEnd, markerStart, pathOptions, interactionWidth, }) => {
        const [path, labelX, labelY] = getBezierPath({
            sourceX,
            sourceY,
            sourcePosition,
            targetX,
            targetY,
            targetPosition,
            curvature: pathOptions?.curvature,
        });
        const _id = params.isInternal ? undefined : id;
        return (jsx(BaseEdge, { id: _id, path: path, labelX: labelX, labelY: labelY, label: label, labelStyle: labelStyle, labelShowBg: labelShowBg, labelBgStyle: labelBgStyle, labelBgPadding: labelBgPadding, labelBgBorderRadius: labelBgBorderRadius, style: style, markerEnd: markerEnd, markerStart: markerStart, interactionWidth: interactionWidth }));
    });
}
/**
 * Component that can be used inside a custom edge to render a bezier curve.
 *
 * @public
 * @example
 *
 * ```tsx
 * import { BezierEdge } from '@xyflow/react';
 *
 * function CustomEdge({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }) {
 *   return (
 *     <BezierEdge
 *       sourceX={sourceX}
 *       sourceY={sourceY}
 *       targetX={targetX}
 *       targetY={targetY}
 *       sourcePosition={sourcePosition}
 *       targetPosition={targetPosition}
 *     />
 *   );
 * }
 * ```
 */
const BezierEdge = createBezierEdge({ isInternal: false });
/**
 * @internal
 */
const BezierEdgeInternal = createBezierEdge({ isInternal: true });
BezierEdge.displayName = 'BezierEdge';
BezierEdgeInternal.displayName = 'BezierEdgeInternal';

const builtinEdgeTypes = {
    default: BezierEdgeInternal,
    straight: StraightEdgeInternal,
    step: StepEdgeInternal,
    smoothstep: SmoothStepEdgeInternal,
    simplebezier: SimpleBezierEdgeInternal,
};
const nullPosition = {
    sourceX: null,
    sourceY: null,
    targetX: null,
    targetY: null,
    sourcePosition: null,
    targetPosition: null,
};

const shiftX = (x, shift, position) => {
    if (position === Position.Left)
        return x - shift;
    if (position === Position.Right)
        return x + shift;
    return x;
};
const shiftY = (y, shift, position) => {
    if (position === Position.Top)
        return y - shift;
    if (position === Position.Bottom)
        return y + shift;
    return y;
};
const EdgeUpdaterClassName = 'react-flow__edgeupdater';
/**
 * @internal
 */
function EdgeAnchor({ position, centerX, centerY, radius = 10, onMouseDown, onMouseEnter, onMouseOut, type, }) {
    return (jsx("circle", { onMouseDown: onMouseDown, onMouseEnter: onMouseEnter, onMouseOut: onMouseOut, className: cc([EdgeUpdaterClassName, `${EdgeUpdaterClassName}-${type}`]), cx: shiftX(centerX, radius, position), cy: shiftY(centerY, radius, position), r: radius, stroke: "transparent", fill: "transparent" }));
}

function EdgeUpdateAnchors({ isReconnectable, reconnectRadius, edge, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, onReconnect, onReconnectStart, onReconnectEnd, setReconnecting, setUpdateHover, }) {
    const store = useStoreApi();
    const handleEdgeUpdater = (event, oppositeHandle) => {
        // avoid triggering edge updater if mouse btn is not left
        if (event.button !== 0) {
            return;
        }
        const { autoPanOnConnect, domNode, connectionMode, connectionRadius, lib, onConnectStart, cancelConnection, nodeLookup, rfId: flowId, panBy, updateConnection, } = store.getState();
        const isTarget = oppositeHandle.type === 'target';
        const _onReconnectEnd = (evt, connectionState) => {
            setReconnecting(false);
            onReconnectEnd?.(evt, edge, oppositeHandle.type, connectionState);
        };
        const onConnectEdge = (connection) => onReconnect?.(edge, connection);
        const _onConnectStart = (_event, params) => {
            setReconnecting(true);
            onReconnectStart?.(event, edge, oppositeHandle.type);
            onConnectStart?.(_event, params);
        };
        XYHandle.onPointerDown(event.nativeEvent, {
            autoPanOnConnect,
            connectionMode,
            connectionRadius,
            domNode,
            handleId: oppositeHandle.id,
            nodeId: oppositeHandle.nodeId,
            nodeLookup,
            isTarget,
            edgeUpdaterType: oppositeHandle.type,
            lib,
            flowId,
            cancelConnection,
            panBy,
            isValidConnection: (...args) => store.getState().isValidConnection?.(...args) ?? true,
            onConnect: onConnectEdge,
            onConnectStart: _onConnectStart,
            onConnectEnd: (...args) => store.getState().onConnectEnd?.(...args),
            onReconnectEnd: _onReconnectEnd,
            updateConnection,
            getTransform: () => store.getState().transform,
            getFromHandle: () => store.getState().connection.fromHandle,
            dragThreshold: store.getState().connectionDragThreshold,
            handleDomNode: event.currentTarget,
        });
    };
    const onReconnectSourceMouseDown = (event) => handleEdgeUpdater(event, { nodeId: edge.target, id: edge.targetHandle ?? null, type: 'target' });
    const onReconnectTargetMouseDown = (event) => handleEdgeUpdater(event, { nodeId: edge.source, id: edge.sourceHandle ?? null, type: 'source' });
    const onReconnectMouseEnter = () => setUpdateHover(true);
    const onReconnectMouseOut = () => setUpdateHover(false);
    return (jsxs(Fragment, { children: [(isReconnectable === true || isReconnectable === 'source') && (jsx(EdgeAnchor, { position: sourcePosition, centerX: sourceX, centerY: sourceY, radius: reconnectRadius, onMouseDown: onReconnectSourceMouseDown, onMouseEnter: onReconnectMouseEnter, onMouseOut: onReconnectMouseOut, type: "source" })), (isReconnectable === true || isReconnectable === 'target') && (jsx(EdgeAnchor, { position: targetPosition, centerX: targetX, centerY: targetY, radius: reconnectRadius, onMouseDown: onReconnectTargetMouseDown, onMouseEnter: onReconnectMouseEnter, onMouseOut: onReconnectMouseOut, type: "target" }))] }));
}

function EdgeWrapper({ id, edgesFocusable, edgesReconnectable, elementsSelectable, onClick, onDoubleClick, onContextMenu, onMouseEnter, onMouseMove, onMouseLeave, reconnectRadius, onReconnect, onReconnectStart, onReconnectEnd, rfId, edgeTypes, noPanClassName, onError, disableKeyboardA11y, }) {
    let edge = useStore$1((s) => s.edgeLookup.get(id));
    const defaultEdgeOptions = useStore$1((s) => s.defaultEdgeOptions);
    edge = defaultEdgeOptions ? { ...defaultEdgeOptions, ...edge } : edge;
    let edgeType = edge.type || 'default';
    let EdgeComponent = edgeTypes?.[edgeType] || builtinEdgeTypes[edgeType];
    if (EdgeComponent === undefined) {
        onError?.('011', errorMessages['error011'](edgeType));
        edgeType = 'default';
        EdgeComponent = edgeTypes?.['default'] || builtinEdgeTypes.default;
    }
    const isFocusable = !!(edge.focusable || (edgesFocusable && typeof edge.focusable === 'undefined'));
    const isReconnectable = typeof onReconnect !== 'undefined' &&
        (edge.reconnectable || (edgesReconnectable && typeof edge.reconnectable === 'undefined'));
    const isSelectable = !!(edge.selectable || (elementsSelectable && typeof edge.selectable === 'undefined'));
    const edgeRef = useRef(null);
    const [updateHover, setUpdateHover] = useState(false);
    const [reconnecting, setReconnecting] = useState(false);
    const store = useStoreApi();
    const { zIndex, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition } = useStore$1(useCallback((store) => {
        const sourceNode = store.nodeLookup.get(edge.source);
        const targetNode = store.nodeLookup.get(edge.target);
        if (!sourceNode || !targetNode) {
            return {
                zIndex: edge.zIndex,
                ...nullPosition,
            };
        }
        const edgePosition = getEdgePosition({
            id,
            sourceNode,
            targetNode,
            sourceHandle: edge.sourceHandle || null,
            targetHandle: edge.targetHandle || null,
            connectionMode: store.connectionMode,
            onError,
        });
        const zIndex = getElevatedEdgeZIndex({
            selected: edge.selected,
            zIndex: edge.zIndex,
            sourceNode,
            targetNode,
            elevateOnSelect: store.elevateEdgesOnSelect,
            zIndexMode: store.zIndexMode,
        });
        return {
            zIndex,
            ...(edgePosition || nullPosition),
        };
    }, [edge.source, edge.target, edge.sourceHandle, edge.targetHandle, edge.selected, edge.zIndex]), shallow$1);
    const markerStartUrl = useMemo(() => (edge.markerStart ? `url('#${getMarkerId(edge.markerStart, rfId)}')` : undefined), [edge.markerStart, rfId]);
    const markerEndUrl = useMemo(() => (edge.markerEnd ? `url('#${getMarkerId(edge.markerEnd, rfId)}')` : undefined), [edge.markerEnd, rfId]);
    if (edge.hidden || sourceX === null || sourceY === null || targetX === null || targetY === null) {
        return null;
    }
    const onEdgeClick = (event) => {
        const { addSelectedEdges, unselectNodesAndEdges, multiSelectionActive } = store.getState();
        if (isSelectable) {
            store.setState({ nodesSelectionActive: false });
            if (edge.selected && multiSelectionActive) {
                unselectNodesAndEdges({ nodes: [], edges: [edge] });
                edgeRef.current?.blur();
            }
            else {
                addSelectedEdges([id]);
            }
        }
        if (onClick) {
            onClick(event, edge);
        }
    };
    const onEdgeDoubleClick = onDoubleClick
        ? (event) => {
            onDoubleClick(event, { ...edge });
        }
        : undefined;
    const onEdgeContextMenu = onContextMenu
        ? (event) => {
            onContextMenu(event, { ...edge });
        }
        : undefined;
    const onEdgeMouseEnter = onMouseEnter
        ? (event) => {
            onMouseEnter(event, { ...edge });
        }
        : undefined;
    const onEdgeMouseMove = onMouseMove
        ? (event) => {
            onMouseMove(event, { ...edge });
        }
        : undefined;
    const onEdgeMouseLeave = onMouseLeave
        ? (event) => {
            onMouseLeave(event, { ...edge });
        }
        : undefined;
    const onKeyDown = (event) => {
        if (!disableKeyboardA11y && elementSelectionKeys.includes(event.key) && isSelectable) {
            const { unselectNodesAndEdges, addSelectedEdges } = store.getState();
            const unselect = event.key === 'Escape';
            if (unselect) {
                edgeRef.current?.blur();
                unselectNodesAndEdges({ edges: [edge] });
            }
            else {
                addSelectedEdges([id]);
            }
        }
    };
    return (jsx("svg", { style: { zIndex }, children: jsxs("g", { className: cc([
                'react-flow__edge',
                `react-flow__edge-${edgeType}`,
                edge.className,
                noPanClassName,
                {
                    selected: edge.selected,
                    animated: edge.animated,
                    inactive: !isSelectable && !onClick,
                    updating: updateHover,
                    selectable: isSelectable,
                },
            ]), onClick: onEdgeClick, onDoubleClick: onEdgeDoubleClick, onContextMenu: onEdgeContextMenu, onMouseEnter: onEdgeMouseEnter, onMouseMove: onEdgeMouseMove, onMouseLeave: onEdgeMouseLeave, onKeyDown: isFocusable ? onKeyDown : undefined, tabIndex: isFocusable ? 0 : undefined, role: edge.ariaRole ?? (isFocusable ? 'group' : 'img'), "aria-roledescription": "edge", "data-id": id, "data-testid": `rf__edge-${id}`, "aria-label": edge.ariaLabel === null ? undefined : edge.ariaLabel || `Edge from ${edge.source} to ${edge.target}`, "aria-describedby": isFocusable ? `${ARIA_EDGE_DESC_KEY}-${rfId}` : undefined, ref: edgeRef, ...edge.domAttributes, children: [!reconnecting && (jsx(EdgeComponent, { id: id, source: edge.source, target: edge.target, type: edge.type, selected: edge.selected, animated: edge.animated, selectable: isSelectable, deletable: edge.deletable ?? true, label: edge.label, labelStyle: edge.labelStyle, labelShowBg: edge.labelShowBg, labelBgStyle: edge.labelBgStyle, labelBgPadding: edge.labelBgPadding, labelBgBorderRadius: edge.labelBgBorderRadius, sourceX: sourceX, sourceY: sourceY, targetX: targetX, targetY: targetY, sourcePosition: sourcePosition, targetPosition: targetPosition, data: edge.data, style: edge.style, sourceHandleId: edge.sourceHandle, targetHandleId: edge.targetHandle, markerStart: markerStartUrl, markerEnd: markerEndUrl, pathOptions: 'pathOptions' in edge ? edge.pathOptions : undefined, interactionWidth: edge.interactionWidth })), isReconnectable && (jsx(EdgeUpdateAnchors, { edge: edge, isReconnectable: isReconnectable, reconnectRadius: reconnectRadius, onReconnect: onReconnect, onReconnectStart: onReconnectStart, onReconnectEnd: onReconnectEnd, sourceX: sourceX, sourceY: sourceY, targetX: targetX, targetY: targetY, sourcePosition: sourcePosition, targetPosition: targetPosition, setUpdateHover: setUpdateHover, setReconnecting: setReconnecting }))] }) }));
}
var EdgeWrapper$1 = memo(EdgeWrapper);

const selector$a = (s) => ({
    edgesFocusable: s.edgesFocusable,
    edgesReconnectable: s.edgesReconnectable,
    elementsSelectable: s.elementsSelectable,
    connectionMode: s.connectionMode,
    onError: s.onError,
});
function EdgeRendererComponent({ defaultMarkerColor, onlyRenderVisibleElements, rfId, edgeTypes, noPanClassName, onReconnect, onEdgeContextMenu, onEdgeMouseEnter, onEdgeMouseMove, onEdgeMouseLeave, onEdgeClick, reconnectRadius, onEdgeDoubleClick, onReconnectStart, onReconnectEnd, disableKeyboardA11y, }) {
    const { edgesFocusable, edgesReconnectable, elementsSelectable, onError } = useStore$1(selector$a, shallow$1);
    const edgeIds = useVisibleEdgeIds(onlyRenderVisibleElements);
    return (jsxs("div", { className: "react-flow__edges", children: [jsx(MarkerDefinitions$1, { defaultColor: defaultMarkerColor, rfId: rfId }), edgeIds.map((id) => {
                return (jsx(EdgeWrapper$1, { id: id, edgesFocusable: edgesFocusable, edgesReconnectable: edgesReconnectable, elementsSelectable: elementsSelectable, noPanClassName: noPanClassName, onReconnect: onReconnect, onContextMenu: onEdgeContextMenu, onMouseEnter: onEdgeMouseEnter, onMouseMove: onEdgeMouseMove, onMouseLeave: onEdgeMouseLeave, onClick: onEdgeClick, reconnectRadius: reconnectRadius, onDoubleClick: onEdgeDoubleClick, onReconnectStart: onReconnectStart, onReconnectEnd: onReconnectEnd, rfId: rfId, onError: onError, edgeTypes: edgeTypes, disableKeyboardA11y: disableKeyboardA11y }, id));
            })] }));
}
EdgeRendererComponent.displayName = 'EdgeRenderer';
const EdgeRenderer = memo(EdgeRendererComponent);

const selector$9 = (s) => `translate(${s.transform[0]}px,${s.transform[1]}px) scale(${s.transform[2]})`;
function Viewport({ children }) {
    const transform = useStore$1(selector$9);
    return (jsx("div", { className: "react-flow__viewport xyflow__viewport react-flow__container", style: { transform }, children: children }));
}

/**
 * Hook for calling onInit handler.
 *
 * @internal
 */
function useOnInitHandler(onInit) {
    const rfInstance = useReactFlow();
    const isInitialized = useRef(false);
    useEffect(() => {
        if (!isInitialized.current && rfInstance.viewportInitialized && onInit) {
            setTimeout(() => onInit(rfInstance), 1);
            isInitialized.current = true;
        }
    }, [onInit, rfInstance.viewportInitialized]);
}

const selector$8 = (state) => state.panZoom?.syncViewport;
/**
 * Hook for syncing the viewport with the panzoom instance.
 *
 * @internal
 * @param viewport
 */
function useViewportSync(viewport) {
    const syncViewport = useStore$1(selector$8);
    const store = useStoreApi();
    useEffect(() => {
        if (viewport) {
            syncViewport?.(viewport);
            store.setState({ transform: [viewport.x, viewport.y, viewport.zoom] });
        }
    }, [viewport, syncViewport]);
    return null;
}

function storeSelector$1(s) {
    return s.connection.inProgress
        ? { ...s.connection, to: pointToRendererPoint(s.connection.to, s.transform) }
        : { ...s.connection };
}
function getSelector(connectionSelector) {
    return storeSelector$1;
}
/**
 * The `useConnection` hook returns the current connection when there is an active
 * connection interaction. If no connection interaction is active, it returns null
 * for every property. A typical use case for this hook is to colorize handles
 * based on a certain condition (e.g. if the connection is valid or not).
 *
 * @public
 * @param connectionSelector - An optional selector function used to extract a slice of the
 * `ConnectionState` data. Using a selector can prevent component re-renders where data you don't
 * otherwise care about might change. If a selector is not provided, the entire `ConnectionState`
 * object is returned unchanged.
 * @example
 *
 * ```tsx
 *import { useConnection } from '@xyflow/react';
 *
 *function App() {
 *  const connection = useConnection();
 *
 *  return (
 *    <div> {connection ? `Someone is trying to make a connection from ${connection.fromNode} to this one.` : 'There are currently no incoming connections!'}
 *
 *   </div>
 *   );
 * }
 * ```
 *
 * @returns ConnectionState
 */
function useConnection(connectionSelector) {
    const combinedSelector = getSelector();
    return useStore$1(combinedSelector, shallow$1);
}

const selector$7 = (s) => ({
    nodesConnectable: s.nodesConnectable,
    isValid: s.connection.isValid,
    inProgress: s.connection.inProgress,
    width: s.width,
    height: s.height,
});
function ConnectionLineWrapper({ containerStyle, style, type, component, }) {
    const { nodesConnectable, width, height, isValid, inProgress } = useStore$1(selector$7, shallow$1);
    const renderConnection = !!(width && nodesConnectable && inProgress);
    if (!renderConnection) {
        return null;
    }
    return (jsx("svg", { style: containerStyle, width: width, height: height, className: "react-flow__connectionline react-flow__container", children: jsx("g", { className: cc(['react-flow__connection', getConnectionStatus(isValid)]), children: jsx(ConnectionLine, { style: style, type: type, CustomComponent: component, isValid: isValid }) }) }));
}
const ConnectionLine = ({ style, type = ConnectionLineType.Bezier, CustomComponent, isValid, }) => {
    const { inProgress, from, fromNode, fromHandle, fromPosition, to, toNode, toHandle, toPosition, pointer } = useConnection();
    if (!inProgress) {
        return;
    }
    if (CustomComponent) {
        return (jsx(CustomComponent, { connectionLineType: type, connectionLineStyle: style, fromNode: fromNode, fromHandle: fromHandle, fromX: from.x, fromY: from.y, toX: to.x, toY: to.y, fromPosition: fromPosition, toPosition: toPosition, connectionStatus: getConnectionStatus(isValid), toNode: toNode, toHandle: toHandle, pointer: pointer }));
    }
    let path = '';
    const pathParams = {
        sourceX: from.x,
        sourceY: from.y,
        sourcePosition: fromPosition,
        targetX: to.x,
        targetY: to.y,
        targetPosition: toPosition,
    };
    switch (type) {
        case ConnectionLineType.Bezier:
            [path] = getBezierPath(pathParams);
            break;
        case ConnectionLineType.SimpleBezier:
            [path] = getSimpleBezierPath(pathParams);
            break;
        case ConnectionLineType.Step:
            [path] = getSmoothStepPath({
                ...pathParams,
                borderRadius: 0,
            });
            break;
        case ConnectionLineType.SmoothStep:
            [path] = getSmoothStepPath(pathParams);
            break;
        default:
            [path] = getStraightPath(pathParams);
    }
    return jsx("path", { d: path, fill: "none", className: "react-flow__connection-path", style: style });
};
ConnectionLine.displayName = 'ConnectionLine';

const emptyTypes = {};
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function useNodeOrEdgeTypesWarning(nodeOrEdgeTypes = emptyTypes) {
    const typesRef = useRef(nodeOrEdgeTypes);
    const store = useStoreApi();
    useEffect(() => {
        if (process.env.NODE_ENV === 'development') {
            const usedKeys = new Set([...Object.keys(typesRef.current), ...Object.keys(nodeOrEdgeTypes)]);
            for (const key of usedKeys) {
                if (typesRef.current[key] !== nodeOrEdgeTypes[key]) {
                    store.getState().onError?.('002', errorMessages['error002']());
                    break;
                }
            }
            typesRef.current = nodeOrEdgeTypes;
        }
    }, [nodeOrEdgeTypes]);
}

function useStylesLoadedWarning() {
    const store = useStoreApi();
    const checked = useRef(false);
    useEffect(() => {
        if (process.env.NODE_ENV === 'development') {
            if (!checked.current) {
                const pane = document.querySelector('.react-flow__pane');
                if (pane && !(window.getComputedStyle(pane).zIndex === '1')) {
                    store.getState().onError?.('013', errorMessages['error013']('react'));
                }
                checked.current = true;
            }
        }
    }, []);
}

function GraphViewComponent({ nodeTypes, edgeTypes, onInit, onNodeClick, onEdgeClick, onNodeDoubleClick, onEdgeDoubleClick, onNodeMouseEnter, onNodeMouseMove, onNodeMouseLeave, onNodeContextMenu, onSelectionContextMenu, onSelectionStart, onSelectionEnd, connectionLineType, connectionLineStyle, connectionLineComponent, connectionLineContainerStyle, selectionKeyCode, selectionOnDrag, selectionMode, multiSelectionKeyCode, panActivationKeyCode, zoomActivationKeyCode, deleteKeyCode, onlyRenderVisibleElements, elementsSelectable, defaultViewport, translateExtent, minZoom, maxZoom, preventScrolling, defaultMarkerColor, zoomOnScroll, zoomOnPinch, panOnScroll, panOnScrollSpeed, panOnScrollMode, zoomOnDoubleClick, panOnDrag, onPaneClick, onPaneMouseEnter, onPaneMouseMove, onPaneMouseLeave, onPaneScroll, onPaneContextMenu, paneClickDistance, nodeClickDistance, onEdgeContextMenu, onEdgeMouseEnter, onEdgeMouseMove, onEdgeMouseLeave, reconnectRadius, onReconnect, onReconnectStart, onReconnectEnd, noDragClassName, noWheelClassName, noPanClassName, disableKeyboardA11y, nodeExtent, rfId, viewport, onViewportChange, }) {
    useNodeOrEdgeTypesWarning(nodeTypes);
    useNodeOrEdgeTypesWarning(edgeTypes);
    useStylesLoadedWarning();
    useOnInitHandler(onInit);
    useViewportSync(viewport);
    return (jsx(FlowRenderer, { onPaneClick: onPaneClick, onPaneMouseEnter: onPaneMouseEnter, onPaneMouseMove: onPaneMouseMove, onPaneMouseLeave: onPaneMouseLeave, onPaneContextMenu: onPaneContextMenu, onPaneScroll: onPaneScroll, paneClickDistance: paneClickDistance, deleteKeyCode: deleteKeyCode, selectionKeyCode: selectionKeyCode, selectionOnDrag: selectionOnDrag, selectionMode: selectionMode, onSelectionStart: onSelectionStart, onSelectionEnd: onSelectionEnd, multiSelectionKeyCode: multiSelectionKeyCode, panActivationKeyCode: panActivationKeyCode, zoomActivationKeyCode: zoomActivationKeyCode, elementsSelectable: elementsSelectable, zoomOnScroll: zoomOnScroll, zoomOnPinch: zoomOnPinch, zoomOnDoubleClick: zoomOnDoubleClick, panOnScroll: panOnScroll, panOnScrollSpeed: panOnScrollSpeed, panOnScrollMode: panOnScrollMode, panOnDrag: panOnDrag, defaultViewport: defaultViewport, translateExtent: translateExtent, minZoom: minZoom, maxZoom: maxZoom, onSelectionContextMenu: onSelectionContextMenu, preventScrolling: preventScrolling, noDragClassName: noDragClassName, noWheelClassName: noWheelClassName, noPanClassName: noPanClassName, disableKeyboardA11y: disableKeyboardA11y, onViewportChange: onViewportChange, isControlledViewport: !!viewport, children: jsxs(Viewport, { children: [jsx(EdgeRenderer, { edgeTypes: edgeTypes, onEdgeClick: onEdgeClick, onEdgeDoubleClick: onEdgeDoubleClick, onReconnect: onReconnect, onReconnectStart: onReconnectStart, onReconnectEnd: onReconnectEnd, onlyRenderVisibleElements: onlyRenderVisibleElements, onEdgeContextMenu: onEdgeContextMenu, onEdgeMouseEnter: onEdgeMouseEnter, onEdgeMouseMove: onEdgeMouseMove, onEdgeMouseLeave: onEdgeMouseLeave, reconnectRadius: reconnectRadius, defaultMarkerColor: defaultMarkerColor, noPanClassName: noPanClassName, disableKeyboardA11y: disableKeyboardA11y, rfId: rfId }), jsx(ConnectionLineWrapper, { style: connectionLineStyle, type: connectionLineType, component: connectionLineComponent, containerStyle: connectionLineContainerStyle }), jsx("div", { className: "react-flow__edgelabel-renderer" }), jsx(NodeRenderer, { nodeTypes: nodeTypes, onNodeClick: onNodeClick, onNodeDoubleClick: onNodeDoubleClick, onNodeMouseEnter: onNodeMouseEnter, onNodeMouseMove: onNodeMouseMove, onNodeMouseLeave: onNodeMouseLeave, onNodeContextMenu: onNodeContextMenu, nodeClickDistance: nodeClickDistance, onlyRenderVisibleElements: onlyRenderVisibleElements, noPanClassName: noPanClassName, noDragClassName: noDragClassName, disableKeyboardA11y: disableKeyboardA11y, nodeExtent: nodeExtent, rfId: rfId }), jsx("div", { className: "react-flow__viewport-portal" })] }) }));
}
GraphViewComponent.displayName = 'GraphView';
const GraphView = memo(GraphViewComponent);

const getInitialState = ({ nodes, edges, defaultNodes, defaultEdges, width, height, fitView, fitViewOptions, minZoom = 0.5, maxZoom = 2, nodeOrigin, nodeExtent, zIndexMode = 'basic', } = {}) => {
    const nodeLookup = new Map();
    const parentLookup = new Map();
    const connectionLookup = new Map();
    const edgeLookup = new Map();
    const storeEdges = defaultEdges ?? edges ?? [];
    const storeNodes = defaultNodes ?? nodes ?? [];
    const storeNodeOrigin = nodeOrigin ?? [0, 0];
    const storeNodeExtent = nodeExtent ?? infiniteExtent;
    updateConnectionLookup(connectionLookup, edgeLookup, storeEdges);
    const { nodesInitialized } = adoptUserNodes(storeNodes, nodeLookup, parentLookup, {
        nodeOrigin: storeNodeOrigin,
        nodeExtent: storeNodeExtent,
        zIndexMode,
    });
    let transform = [0, 0, 1];
    if (fitView && width && height) {
        const bounds = getInternalNodesBounds(nodeLookup, {
            filter: (node) => !!((node.width || node.initialWidth) && (node.height || node.initialHeight)),
        });
        const { x, y, zoom } = getViewportForBounds(bounds, width, height, minZoom, maxZoom, fitViewOptions?.padding ?? 0.1);
        transform = [x, y, zoom];
    }
    return {
        rfId: '1',
        width: width ?? 0,
        height: height ?? 0,
        transform,
        nodes: storeNodes,
        nodesInitialized,
        nodeLookup,
        parentLookup,
        edges: storeEdges,
        edgeLookup,
        connectionLookup,
        onNodesChange: null,
        onEdgesChange: null,
        hasDefaultNodes: defaultNodes !== undefined,
        hasDefaultEdges: defaultEdges !== undefined,
        panZoom: null,
        minZoom,
        maxZoom,
        translateExtent: infiniteExtent,
        nodeExtent: storeNodeExtent,
        nodesSelectionActive: false,
        userSelectionActive: false,
        userSelectionRect: null,
        connectionMode: ConnectionMode.Strict,
        domNode: null,
        paneDragging: false,
        noPanClassName: 'nopan',
        nodeOrigin: storeNodeOrigin,
        nodeDragThreshold: 1,
        connectionDragThreshold: 1,
        snapGrid: [15, 15],
        snapToGrid: false,
        nodesDraggable: true,
        nodesConnectable: true,
        nodesFocusable: true,
        edgesFocusable: true,
        edgesReconnectable: true,
        elementsSelectable: true,
        elevateNodesOnSelect: true,
        elevateEdgesOnSelect: true,
        selectNodesOnDrag: true,
        multiSelectionActive: false,
        fitViewQueued: fitView ?? false,
        fitViewOptions,
        fitViewResolver: null,
        connection: { ...initialConnection },
        connectionClickStartHandle: null,
        connectOnClick: true,
        ariaLiveMessage: '',
        autoPanOnConnect: true,
        autoPanOnNodeDrag: true,
        autoPanOnNodeFocus: true,
        autoPanSpeed: 15,
        connectionRadius: 20,
        onError: devWarn,
        isValidConnection: undefined,
        onSelectionChangeHandlers: [],
        lib: 'react',
        debug: false,
        ariaLabelConfig: defaultAriaLabelConfig,
        zIndexMode,
        onNodesChangeMiddlewareMap: new Map(),
        onEdgesChangeMiddlewareMap: new Map(),
    };
};

const createStore$1 = ({ nodes, edges, defaultNodes, defaultEdges, width, height, fitView, fitViewOptions, minZoom, maxZoom, nodeOrigin, nodeExtent, zIndexMode, }) => createWithEqualityFn((set, get) => {
    async function resolveFitView() {
        const { nodeLookup, panZoom, fitViewOptions, fitViewResolver, width, height, minZoom, maxZoom } = get();
        if (!panZoom) {
            return;
        }
        await fitViewport({
            nodes: nodeLookup,
            width,
            height,
            panZoom,
            minZoom,
            maxZoom,
        }, fitViewOptions);
        fitViewResolver?.resolve(true);
        /**
         * wait for the fitViewport to resolve before deleting the resolver,
         * we want to reuse the old resolver if the user calls fitView again in the mean time
         */
        set({ fitViewResolver: null });
    }
    return {
        ...getInitialState({
            nodes,
            edges,
            width,
            height,
            fitView,
            fitViewOptions,
            minZoom,
            maxZoom,
            nodeOrigin,
            nodeExtent,
            defaultNodes,
            defaultEdges,
            zIndexMode,
        }),
        setNodes: (nodes) => {
            const { nodeLookup, parentLookup, nodeOrigin, elevateNodesOnSelect, fitViewQueued, zIndexMode, nodesSelectionActive, } = get();
            /*
             * setNodes() is called exclusively in response to user actions:
             * - either when the `<ReactFlow nodes>` prop is updated in the controlled ReactFlow setup,
             * - or when the user calls something like `reactFlowInstance.setNodes()` in an uncontrolled ReactFlow setup.
             *
             * When this happens, we take the note objects passed by the user and extend them with fields
             * relevant for internal React Flow operations.
             */
            const { nodesInitialized, hasSelectedNodes } = adoptUserNodes(nodes, nodeLookup, parentLookup, {
                nodeOrigin,
                nodeExtent,
                elevateNodesOnSelect,
                checkEquality: true,
                zIndexMode,
            });
            const nextNodesSelectionActive = nodesSelectionActive && hasSelectedNodes;
            if (fitViewQueued && nodesInitialized) {
                resolveFitView();
                set({
                    nodes,
                    nodesInitialized,
                    fitViewQueued: false,
                    fitViewOptions: undefined,
                    nodesSelectionActive: nextNodesSelectionActive
                });
            }
            else {
                set({ nodes, nodesInitialized, nodesSelectionActive: nextNodesSelectionActive });
            }
        },
        setEdges: (edges) => {
            const { connectionLookup, edgeLookup } = get();
            updateConnectionLookup(connectionLookup, edgeLookup, edges);
            set({ edges });
        },
        setDefaultNodesAndEdges: (nodes, edges) => {
            if (nodes) {
                const { setNodes } = get();
                setNodes(nodes);
                set({ hasDefaultNodes: true });
            }
            if (edges) {
                const { setEdges } = get();
                setEdges(edges);
                set({ hasDefaultEdges: true });
            }
        },
        /*
         * Every node gets registered at a ResizeObserver. Whenever a node
         * changes its dimensions, this function is called to measure the
         * new dimensions and update the nodes.
         */
        updateNodeInternals: (updates) => {
            const { triggerNodeChanges, nodeLookup, parentLookup, domNode, nodeOrigin, nodeExtent, debug, fitViewQueued, zIndexMode, } = get();
            const { changes, updatedInternals } = updateNodeInternals(updates, nodeLookup, parentLookup, domNode, nodeOrigin, nodeExtent, zIndexMode);
            if (!updatedInternals) {
                return;
            }
            updateAbsolutePositions(nodeLookup, parentLookup, { nodeOrigin, nodeExtent, zIndexMode });
            if (fitViewQueued) {
                resolveFitView();
                set({ fitViewQueued: false, fitViewOptions: undefined });
            }
            else {
                // we always want to trigger useStore calls whenever updateNodeInternals is called
                set({});
            }
            if (changes?.length > 0) {
                if (debug) {
                    console.log('React Flow: trigger node changes', changes);
                }
                triggerNodeChanges?.(changes);
            }
        },
        updateNodePositions: (nodeDragItems, dragging = false) => {
            const parentExpandChildren = [];
            let changes = [];
            const { nodeLookup, triggerNodeChanges, connection, updateConnection, onNodesChangeMiddlewareMap } = get();
            for (const [id, dragItem] of nodeDragItems) {
                // we are using the nodelookup to be sure to use the current expandParent and parentId value
                const node = nodeLookup.get(id);
                const expandParent = !!(node?.expandParent && node?.parentId && dragItem?.position);
                const change = {
                    id,
                    type: 'position',
                    position: expandParent
                        ? {
                            x: Math.max(0, dragItem.position.x),
                            y: Math.max(0, dragItem.position.y),
                        }
                        : dragItem.position,
                    dragging,
                };
                if (node && connection.inProgress && connection.fromNode.id === node.id) {
                    const updatedFrom = getHandlePosition(node, connection.fromHandle, Position.Left, true);
                    updateConnection({ ...connection, from: updatedFrom });
                }
                if (expandParent && node.parentId) {
                    parentExpandChildren.push({
                        id,
                        parentId: node.parentId,
                        rect: {
                            ...dragItem.internals.positionAbsolute,
                            width: dragItem.measured.width ?? 0,
                            height: dragItem.measured.height ?? 0,
                        },
                    });
                }
                changes.push(change);
            }
            if (parentExpandChildren.length > 0) {
                const { parentLookup, nodeOrigin } = get();
                const parentExpandChanges = handleExpandParent(parentExpandChildren, nodeLookup, parentLookup, nodeOrigin);
                changes.push(...parentExpandChanges);
            }
            for (const middleware of onNodesChangeMiddlewareMap.values()) {
                changes = middleware(changes);
            }
            triggerNodeChanges(changes);
        },
        triggerNodeChanges: (changes) => {
            const { onNodesChange, setNodes, nodes, hasDefaultNodes, debug } = get();
            if (changes?.length) {
                if (hasDefaultNodes) {
                    const updatedNodes = applyNodeChanges(changes, nodes);
                    setNodes(updatedNodes);
                }
                if (debug) {
                    console.log('React Flow: trigger node changes', changes);
                }
                onNodesChange?.(changes);
            }
        },
        triggerEdgeChanges: (changes) => {
            const { onEdgesChange, setEdges, edges, hasDefaultEdges, debug } = get();
            if (changes?.length) {
                if (hasDefaultEdges) {
                    const updatedEdges = applyEdgeChanges(changes, edges);
                    setEdges(updatedEdges);
                }
                if (debug) {
                    console.log('React Flow: trigger edge changes', changes);
                }
                onEdgesChange?.(changes);
            }
        },
        addSelectedNodes: (selectedNodeIds) => {
            const { multiSelectionActive, edgeLookup, nodeLookup, triggerNodeChanges, triggerEdgeChanges } = get();
            if (multiSelectionActive) {
                const nodeChanges = selectedNodeIds.map((nodeId) => createSelectionChange(nodeId, true));
                triggerNodeChanges(nodeChanges);
                return;
            }
            triggerNodeChanges(getSelectionChanges(nodeLookup, new Set([...selectedNodeIds]), true));
            triggerEdgeChanges(getSelectionChanges(edgeLookup));
        },
        addSelectedEdges: (selectedEdgeIds) => {
            const { multiSelectionActive, edgeLookup, nodeLookup, triggerNodeChanges, triggerEdgeChanges } = get();
            if (multiSelectionActive) {
                const changedEdges = selectedEdgeIds.map((edgeId) => createSelectionChange(edgeId, true));
                triggerEdgeChanges(changedEdges);
                return;
            }
            triggerEdgeChanges(getSelectionChanges(edgeLookup, new Set([...selectedEdgeIds])));
            triggerNodeChanges(getSelectionChanges(nodeLookup, new Set(), true));
        },
        unselectNodesAndEdges: ({ nodes, edges } = {}) => {
            const { edges: storeEdges, nodes: storeNodes, nodeLookup, triggerNodeChanges, triggerEdgeChanges } = get();
            const nodesToUnselect = nodes ? nodes : storeNodes;
            const edgesToUnselect = edges ? edges : storeEdges;
            const nodeChanges = [];
            for (const node of nodesToUnselect) {
                if (!node.selected) {
                    continue; // skip changing nodes that are not selected
                }
                const internalNode = nodeLookup.get(node.id);
                if (internalNode) {
                    /*
                     * we need to unselect the internal node that was selected previously before we
                     * send the change to the user to prevent it to be selected while dragging the new node
                     */
                    internalNode.selected = false;
                }
                nodeChanges.push(createSelectionChange(node.id, false));
            }
            const edgeChanges = [];
            for (const edge of edgesToUnselect) {
                if (!edge.selected) {
                    continue; // skip changing edges that are not selected
                }
                edgeChanges.push(createSelectionChange(edge.id, false));
            }
            triggerNodeChanges(nodeChanges);
            triggerEdgeChanges(edgeChanges);
        },
        setMinZoom: (minZoom) => {
            const { panZoom, maxZoom } = get();
            panZoom?.setScaleExtent([minZoom, maxZoom]);
            set({ minZoom });
        },
        setMaxZoom: (maxZoom) => {
            const { panZoom, minZoom } = get();
            panZoom?.setScaleExtent([minZoom, maxZoom]);
            set({ maxZoom });
        },
        setTranslateExtent: (translateExtent) => {
            get().panZoom?.setTranslateExtent(translateExtent);
            set({ translateExtent });
        },
        resetSelectedElements: () => {
            const { edges, nodes, triggerNodeChanges, triggerEdgeChanges, elementsSelectable } = get();
            if (!elementsSelectable) {
                return;
            }
            const nodeChanges = nodes.reduce((res, node) => (node.selected ? [...res, createSelectionChange(node.id, false)] : res), []);
            const edgeChanges = edges.reduce((res, edge) => (edge.selected ? [...res, createSelectionChange(edge.id, false)] : res), []);
            triggerNodeChanges(nodeChanges);
            triggerEdgeChanges(edgeChanges);
        },
        setNodeExtent: (nextNodeExtent) => {
            const { nodes, nodeLookup, parentLookup, nodeOrigin, elevateNodesOnSelect, nodeExtent, zIndexMode } = get();
            if (nextNodeExtent[0][0] === nodeExtent[0][0] &&
                nextNodeExtent[0][1] === nodeExtent[0][1] &&
                nextNodeExtent[1][0] === nodeExtent[1][0] &&
                nextNodeExtent[1][1] === nodeExtent[1][1]) {
                return;
            }
            adoptUserNodes(nodes, nodeLookup, parentLookup, {
                nodeOrigin,
                nodeExtent: nextNodeExtent,
                elevateNodesOnSelect,
                checkEquality: false,
                zIndexMode,
            });
            set({ nodeExtent: nextNodeExtent });
        },
        panBy: (delta) => {
            const { transform, width, height, panZoom, translateExtent } = get();
            return panBy({ delta, panZoom, transform, translateExtent, width, height });
        },
        setCenter: async (x, y, options) => {
            const { width, height, maxZoom, panZoom } = get();
            if (!panZoom) {
                return Promise.resolve(false);
            }
            const nextZoom = typeof options?.zoom !== 'undefined' ? options.zoom : maxZoom;
            await panZoom.setViewport({
                x: width / 2 - x * nextZoom,
                y: height / 2 - y * nextZoom,
                zoom: nextZoom,
            }, { duration: options?.duration, ease: options?.ease, interpolate: options?.interpolate });
            return Promise.resolve(true);
        },
        cancelConnection: () => {
            set({
                connection: { ...initialConnection },
            });
        },
        updateConnection: (connection) => {
            set({ connection });
        },
        reset: () => set({ ...getInitialState() }),
    };
}, Object.is);

/**
 * The `<ReactFlowProvider />` component is a [context provider](https://react.dev/learn/passing-data-deeply-with-context#)
 * that makes it possible to access a flow's internal state outside of the
 * [`<ReactFlow />`](/api-reference/react-flow) component. Many of the hooks we
 * provide rely on this component to work.
 * @public
 *
 * @example
 * ```tsx
 *import { ReactFlow, ReactFlowProvider, useNodes } from '@xyflow/react'
 *
 *export default function Flow() {
 *  return (
 *    <ReactFlowProvider>
 *      <ReactFlow nodes={...} edges={...} />
 *      <Sidebar />
 *    </ReactFlowProvider>
 *  );
 *}
 *
 *function Sidebar() {
 *  // This hook will only work if the component it's used in is a child of a
 *  // <ReactFlowProvider />.
 *  const nodes = useNodes()
 *
 *  return <aside>do something with nodes</aside>;
 *}
 *```
 *
 * @remarks If you're using a router and want your flow's state to persist across routes,
 * it's vital that you place the `<ReactFlowProvider />` component _outside_ of
 * your router. If you have multiple flows on the same page you will need to use a separate
 * `<ReactFlowProvider />` for each flow.
 */
function ReactFlowProvider({ initialNodes: nodes, initialEdges: edges, defaultNodes, defaultEdges, initialWidth: width, initialHeight: height, initialMinZoom: minZoom, initialMaxZoom: maxZoom, initialFitViewOptions: fitViewOptions, fitView, nodeOrigin, nodeExtent, zIndexMode, children, }) {
    const [store] = useState(() => createStore$1({
        nodes,
        edges,
        defaultNodes,
        defaultEdges,
        width,
        height,
        fitView,
        minZoom,
        maxZoom,
        fitViewOptions,
        nodeOrigin,
        nodeExtent,
        zIndexMode,
    }));
    return (jsx(Provider$1, { value: store, children: jsx(BatchProvider, { children: children }) }));
}

function Wrapper({ children, nodes, edges, defaultNodes, defaultEdges, width, height, fitView, fitViewOptions, minZoom, maxZoom, nodeOrigin, nodeExtent, zIndexMode, }) {
    const isWrapped = useContext(StoreContext);
    if (isWrapped) {
        /*
         * we need to wrap it with a fragment because it's not allowed for children to be a ReactNode
         * https://github.com/DefinitelyTyped/DefinitelyTyped/issues/18051
         */
        return jsx(Fragment, { children: children });
    }
    return (jsx(ReactFlowProvider, { initialNodes: nodes, initialEdges: edges, defaultNodes: defaultNodes, defaultEdges: defaultEdges, initialWidth: width, initialHeight: height, fitView: fitView, initialFitViewOptions: fitViewOptions, initialMinZoom: minZoom, initialMaxZoom: maxZoom, nodeOrigin: nodeOrigin, nodeExtent: nodeExtent, zIndexMode: zIndexMode, children: children }));
}

const wrapperStyle = {
    width: '100%',
    height: '100%',
    overflow: 'hidden',
    position: 'relative',
    zIndex: 0,
};
function ReactFlow({ nodes, edges, defaultNodes, defaultEdges, className, nodeTypes, edgeTypes, onNodeClick, onEdgeClick, onInit, onMove, onMoveStart, onMoveEnd, onConnect, onConnectStart, onConnectEnd, onClickConnectStart, onClickConnectEnd, onNodeMouseEnter, onNodeMouseMove, onNodeMouseLeave, onNodeContextMenu, onNodeDoubleClick, onNodeDragStart, onNodeDrag, onNodeDragStop, onNodesDelete, onEdgesDelete, onDelete, onSelectionChange, onSelectionDragStart, onSelectionDrag, onSelectionDragStop, onSelectionContextMenu, onSelectionStart, onSelectionEnd, onBeforeDelete, connectionMode, connectionLineType = ConnectionLineType.Bezier, connectionLineStyle, connectionLineComponent, connectionLineContainerStyle, deleteKeyCode = 'Backspace', selectionKeyCode = 'Shift', selectionOnDrag = false, selectionMode = SelectionMode.Full, panActivationKeyCode = 'Space', multiSelectionKeyCode = isMacOs() ? 'Meta' : 'Control', zoomActivationKeyCode = isMacOs() ? 'Meta' : 'Control', snapToGrid, snapGrid, onlyRenderVisibleElements = false, selectNodesOnDrag, nodesDraggable, autoPanOnNodeFocus, nodesConnectable, nodesFocusable, nodeOrigin = defaultNodeOrigin, edgesFocusable, edgesReconnectable, elementsSelectable = true, defaultViewport: defaultViewport$1 = defaultViewport, minZoom = 0.5, maxZoom = 2, translateExtent = infiniteExtent, preventScrolling = true, nodeExtent, defaultMarkerColor = '#b1b1b7', zoomOnScroll = true, zoomOnPinch = true, panOnScroll = false, panOnScrollSpeed = 0.5, panOnScrollMode = PanOnScrollMode.Free, zoomOnDoubleClick = true, panOnDrag = true, onPaneClick, onPaneMouseEnter, onPaneMouseMove, onPaneMouseLeave, onPaneScroll, onPaneContextMenu, paneClickDistance = 1, nodeClickDistance = 0, children, onReconnect, onReconnectStart, onReconnectEnd, onEdgeContextMenu, onEdgeDoubleClick, onEdgeMouseEnter, onEdgeMouseMove, onEdgeMouseLeave, reconnectRadius = 10, onNodesChange, onEdgesChange, noDragClassName = 'nodrag', noWheelClassName = 'nowheel', noPanClassName = 'nopan', fitView, fitViewOptions, connectOnClick, attributionPosition, proOptions, defaultEdgeOptions, elevateNodesOnSelect = true, elevateEdgesOnSelect = false, disableKeyboardA11y = false, autoPanOnConnect, autoPanOnNodeDrag, autoPanSpeed, connectionRadius, isValidConnection, onError, style, id, nodeDragThreshold, connectionDragThreshold, viewport, onViewportChange, width, height, colorMode = 'light', debug, onScroll, ariaLabelConfig, zIndexMode = 'basic', ...rest }, ref) {
    const rfId = id || '1';
    const colorModeClassName = useColorModeClass(colorMode);
    // Undo scroll events, preventing viewport from shifting when nodes outside of it are focused
    const wrapperOnScroll = useCallback((e) => {
        e.currentTarget.scrollTo({ top: 0, left: 0, behavior: 'instant' });
        onScroll?.(e);
    }, [onScroll]);
    return (jsx("div", { "data-testid": "rf__wrapper", ...rest, onScroll: wrapperOnScroll, style: { ...style, ...wrapperStyle }, ref: ref, className: cc(['react-flow', className, colorModeClassName]), id: id, role: "application", children: jsxs(Wrapper, { nodes: nodes, edges: edges, width: width, height: height, fitView: fitView, fitViewOptions: fitViewOptions, minZoom: minZoom, maxZoom: maxZoom, nodeOrigin: nodeOrigin, nodeExtent: nodeExtent, zIndexMode: zIndexMode, children: [jsx(StoreUpdater, { nodes: nodes, edges: edges, defaultNodes: defaultNodes, defaultEdges: defaultEdges, onConnect: onConnect, onConnectStart: onConnectStart, onConnectEnd: onConnectEnd, onClickConnectStart: onClickConnectStart, onClickConnectEnd: onClickConnectEnd, nodesDraggable: nodesDraggable, autoPanOnNodeFocus: autoPanOnNodeFocus, nodesConnectable: nodesConnectable, nodesFocusable: nodesFocusable, edgesFocusable: edgesFocusable, edgesReconnectable: edgesReconnectable, elementsSelectable: elementsSelectable, elevateNodesOnSelect: elevateNodesOnSelect, elevateEdgesOnSelect: elevateEdgesOnSelect, minZoom: minZoom, maxZoom: maxZoom, nodeExtent: nodeExtent, onNodesChange: onNodesChange, onEdgesChange: onEdgesChange, snapToGrid: snapToGrid, snapGrid: snapGrid, connectionMode: connectionMode, translateExtent: translateExtent, connectOnClick: connectOnClick, defaultEdgeOptions: defaultEdgeOptions, fitView: fitView, fitViewOptions: fitViewOptions, onNodesDelete: onNodesDelete, onEdgesDelete: onEdgesDelete, onDelete: onDelete, onNodeDragStart: onNodeDragStart, onNodeDrag: onNodeDrag, onNodeDragStop: onNodeDragStop, onSelectionDrag: onSelectionDrag, onSelectionDragStart: onSelectionDragStart, onSelectionDragStop: onSelectionDragStop, onMove: onMove, onMoveStart: onMoveStart, onMoveEnd: onMoveEnd, noPanClassName: noPanClassName, nodeOrigin: nodeOrigin, rfId: rfId, autoPanOnConnect: autoPanOnConnect, autoPanOnNodeDrag: autoPanOnNodeDrag, autoPanSpeed: autoPanSpeed, onError: onError, connectionRadius: connectionRadius, isValidConnection: isValidConnection, selectNodesOnDrag: selectNodesOnDrag, nodeDragThreshold: nodeDragThreshold, connectionDragThreshold: connectionDragThreshold, onBeforeDelete: onBeforeDelete, debug: debug, ariaLabelConfig: ariaLabelConfig, zIndexMode: zIndexMode }), jsx(GraphView, { onInit: onInit, onNodeClick: onNodeClick, onEdgeClick: onEdgeClick, onNodeMouseEnter: onNodeMouseEnter, onNodeMouseMove: onNodeMouseMove, onNodeMouseLeave: onNodeMouseLeave, onNodeContextMenu: onNodeContextMenu, onNodeDoubleClick: onNodeDoubleClick, nodeTypes: nodeTypes, edgeTypes: edgeTypes, connectionLineType: connectionLineType, connectionLineStyle: connectionLineStyle, connectionLineComponent: connectionLineComponent, connectionLineContainerStyle: connectionLineContainerStyle, selectionKeyCode: selectionKeyCode, selectionOnDrag: selectionOnDrag, selectionMode: selectionMode, deleteKeyCode: deleteKeyCode, multiSelectionKeyCode: multiSelectionKeyCode, panActivationKeyCode: panActivationKeyCode, zoomActivationKeyCode: zoomActivationKeyCode, onlyRenderVisibleElements: onlyRenderVisibleElements, defaultViewport: defaultViewport$1, translateExtent: translateExtent, minZoom: minZoom, maxZoom: maxZoom, preventScrolling: preventScrolling, zoomOnScroll: zoomOnScroll, zoomOnPinch: zoomOnPinch, zoomOnDoubleClick: zoomOnDoubleClick, panOnScroll: panOnScroll, panOnScrollSpeed: panOnScrollSpeed, panOnScrollMode: panOnScrollMode, panOnDrag: panOnDrag, onPaneClick: onPaneClick, onPaneMouseEnter: onPaneMouseEnter, onPaneMouseMove: onPaneMouseMove, onPaneMouseLeave: onPaneMouseLeave, onPaneScroll: onPaneScroll, onPaneContextMenu: onPaneContextMenu, paneClickDistance: paneClickDistance, nodeClickDistance: nodeClickDistance, onSelectionContextMenu: onSelectionContextMenu, onSelectionStart: onSelectionStart, onSelectionEnd: onSelectionEnd, onReconnect: onReconnect, onReconnectStart: onReconnectStart, onReconnectEnd: onReconnectEnd, onEdgeContextMenu: onEdgeContextMenu, onEdgeDoubleClick: onEdgeDoubleClick, onEdgeMouseEnter: onEdgeMouseEnter, onEdgeMouseMove: onEdgeMouseMove, onEdgeMouseLeave: onEdgeMouseLeave, reconnectRadius: reconnectRadius, defaultMarkerColor: defaultMarkerColor, noDragClassName: noDragClassName, noWheelClassName: noWheelClassName, noPanClassName: noPanClassName, rfId: rfId, disableKeyboardA11y: disableKeyboardA11y, nodeExtent: nodeExtent, viewport: viewport, onViewportChange: onViewportChange }), jsx(SelectionListener, { onSelectionChange: onSelectionChange }), children, jsx(Attribution, { proOptions: proOptions, position: attributionPosition }), jsx(A11yDescriptions, { rfId: rfId, disableKeyboardA11y: disableKeyboardA11y })] }) }));
}
/**
 * The `<ReactFlow />` component is the heart of your React Flow application.
 * It renders your nodes and edges and handles user interaction
 *
 * @public
 *
 * @example
 * ```tsx
 *import { ReactFlow } from '@xyflow/react'
 *
 *export default function Flow() {
 *  return (<ReactFlow
 *    nodes={...}
 *    edges={...}
 *    onNodesChange={...}
 *    ...
 *  />);
 *}
 *```
 */
var index = fixedForwardRef(ReactFlow);

function LinePattern({ dimensions, lineWidth, variant, className }) {
    return (jsx("path", { strokeWidth: lineWidth, d: `M${dimensions[0] / 2} 0 V${dimensions[1]} M0 ${dimensions[1] / 2} H${dimensions[0]}`, className: cc(['react-flow__background-pattern', variant, className]) }));
}
function DotPattern({ radius, className }) {
    return (jsx("circle", { cx: radius, cy: radius, r: radius, className: cc(['react-flow__background-pattern', 'dots', className]) }));
}

/**
 * The three variants are exported as an enum for convenience. You can either import
 * the enum and use it like `BackgroundVariant.Lines` or you can use the raw string
 * value directly.
 * @public
 */
var BackgroundVariant;
(function (BackgroundVariant) {
    BackgroundVariant["Lines"] = "lines";
    BackgroundVariant["Dots"] = "dots";
    BackgroundVariant["Cross"] = "cross";
})(BackgroundVariant || (BackgroundVariant = {}));

const defaultSize = {
    [BackgroundVariant.Dots]: 1,
    [BackgroundVariant.Lines]: 1,
    [BackgroundVariant.Cross]: 6,
};
const selector$3 = (s) => ({ transform: s.transform, patternId: `pattern-${s.rfId}` });
function BackgroundComponent({ id, variant = BackgroundVariant.Dots, 
// only used for dots and cross
gap = 20, 
// only used for lines and cross
size, lineWidth = 1, offset = 0, color, bgColor, style, className, patternClassName, }) {
    const ref = useRef(null);
    const { transform, patternId } = useStore$1(selector$3, shallow$1);
    const patternSize = size || defaultSize[variant];
    const isDots = variant === BackgroundVariant.Dots;
    const isCross = variant === BackgroundVariant.Cross;
    const gapXY = Array.isArray(gap) ? gap : [gap, gap];
    const scaledGap = [gapXY[0] * transform[2] || 1, gapXY[1] * transform[2] || 1];
    const scaledSize = patternSize * transform[2];
    const offsetXY = Array.isArray(offset) ? offset : [offset, offset];
    const patternDimensions = isCross ? [scaledSize, scaledSize] : scaledGap;
    const scaledOffset = [
        offsetXY[0] * transform[2] || 1 + patternDimensions[0] / 2,
        offsetXY[1] * transform[2] || 1 + patternDimensions[1] / 2,
    ];
    const _patternId = `${patternId}${id ? id : ''}`;
    return (jsxs("svg", { className: cc(['react-flow__background', className]), style: {
            ...style,
            ...containerStyle,
            '--xy-background-color-props': bgColor,
            '--xy-background-pattern-color-props': color,
        }, ref: ref, "data-testid": "rf__background", children: [jsx("pattern", { id: _patternId, x: transform[0] % scaledGap[0], y: transform[1] % scaledGap[1], width: scaledGap[0], height: scaledGap[1], patternUnits: "userSpaceOnUse", patternTransform: `translate(-${scaledOffset[0]},-${scaledOffset[1]})`, children: isDots ? (jsx(DotPattern, { radius: scaledSize / 2, className: patternClassName })) : (jsx(LinePattern, { dimensions: patternDimensions, lineWidth: lineWidth, variant: variant, className: patternClassName })) }), jsx("rect", { x: "0", y: "0", width: "100%", height: "100%", fill: `url(#${_patternId})` })] }));
}
BackgroundComponent.displayName = 'Background';
/**
 * The `<Background />` component makes it convenient to render different types of backgrounds common in node-based UIs. It comes with three variants: lines, dots and cross.
 *
 * @example
 *
 * A simple example of how to use the Background component.
 *
 * ```tsx
 * import { useState } from 'react';
 * import { ReactFlow, Background, BackgroundVariant } from '@xyflow/react';
 *
 * export default function Flow() {
 *   return (
 *     <ReactFlow defaultNodes={[...]} defaultEdges={[...]}>
 *       <Background color="#ccc" variant={BackgroundVariant.Dots} />
 *     </ReactFlow>
 *   );
 * }
 * ```
 *
 * @example
 *
 * In this example you can see how to combine multiple backgrounds
 *
 * ```tsx
 * import { ReactFlow, Background, BackgroundVariant } from '@xyflow/react';
 * import '@xyflow/react/dist/style.css';
 *
 * export default function Flow() {
 *   return (
 *     <ReactFlow defaultNodes={[...]} defaultEdges={[...]}>
 *       <Background
 *         id="1"
 *         gap={10}
 *         color="#f1f1f1"
 *         variant={BackgroundVariant.Lines}
 *       />
 *       <Background
 *         id="2"
 *         gap={100}
 *         color="#ccc"
 *         variant={BackgroundVariant.Lines}
 *       />
 *     </ReactFlow>
 *   );
 * }
 * ```
 *
 * @remarks
 *
 * When combining multiple <Background /> components it’s important to give each of them a unique id prop!
 *
 */
const Background = memo(BackgroundComponent);

function PlusIcon() {
    return (jsx("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 32 32", children: jsx("path", { d: "M32 18.133H18.133V32h-4.266V18.133H0v-4.266h13.867V0h4.266v13.867H32z" }) }));
}

function MinusIcon() {
    return (jsx("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 32 5", children: jsx("path", { d: "M0 0h32v4.2H0z" }) }));
}

function FitViewIcon() {
    return (jsx("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 32 30", children: jsx("path", { d: "M3.692 4.63c0-.53.4-.938.939-.938h5.215V0H4.708C2.13 0 0 2.054 0 4.63v5.216h3.692V4.631zM27.354 0h-5.2v3.692h5.17c.53 0 .984.4.984.939v5.215H32V4.631A4.624 4.624 0 0027.354 0zm.954 24.83c0 .532-.4.94-.939.94h-5.215v3.768h5.215c2.577 0 4.631-2.13 4.631-4.707v-5.139h-3.692v5.139zm-23.677.94c-.531 0-.939-.4-.939-.94v-5.138H0v5.139c0 2.577 2.13 4.707 4.708 4.707h5.138V25.77H4.631z" }) }));
}

function LockIcon() {
    return (jsx("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 25 32", children: jsx("path", { d: "M21.333 10.667H19.81V7.619C19.81 3.429 16.38 0 12.19 0 8 0 4.571 3.429 4.571 7.619v3.048H3.048A3.056 3.056 0 000 13.714v15.238A3.056 3.056 0 003.048 32h18.285a3.056 3.056 0 003.048-3.048V13.714a3.056 3.056 0 00-3.048-3.047zM12.19 24.533a3.056 3.056 0 01-3.047-3.047 3.056 3.056 0 013.047-3.048 3.056 3.056 0 013.048 3.048 3.056 3.056 0 01-3.048 3.047zm4.724-13.866H7.467V7.619c0-2.59 2.133-4.724 4.723-4.724 2.591 0 4.724 2.133 4.724 4.724v3.048z" }) }));
}

function UnlockIcon() {
    return (jsx("svg", { xmlns: "http://www.w3.org/2000/svg", viewBox: "0 0 25 32", children: jsx("path", { d: "M21.333 10.667H19.81V7.619C19.81 3.429 16.38 0 12.19 0c-4.114 1.828-1.37 2.133.305 2.438 1.676.305 4.42 2.59 4.42 5.181v3.048H3.047A3.056 3.056 0 000 13.714v15.238A3.056 3.056 0 003.048 32h18.285a3.056 3.056 0 003.048-3.048V13.714a3.056 3.056 0 00-3.048-3.047zM12.19 24.533a3.056 3.056 0 01-3.047-3.047 3.056 3.056 0 013.047-3.048 3.056 3.056 0 013.048 3.048 3.056 3.056 0 01-3.048 3.047z" }) }));
}

/**
 * You can add buttons to the control panel by using the `<ControlButton />` component
 * and pass it as a child to the [`<Controls />`](/api-reference/components/controls) component.
 *
 * @public
 * @example
 *```jsx
 *import { MagicWand } from '@radix-ui/react-icons'
 *import { ReactFlow, Controls, ControlButton } from '@xyflow/react'
 *
 *export default function Flow() {
 *  return (
 *    <ReactFlow nodes={[...]} edges={[...]}>
 *      <Controls>
 *        <ControlButton onClick={() => alert('Something magical just happened. ✨')}>
 *          <MagicWand />
 *        </ControlButton>
 *      </Controls>
 *    </ReactFlow>
 *  )
 *}
 *```
 */
function ControlButton({ children, className, ...rest }) {
    return (jsx("button", { type: "button", className: cc(['react-flow__controls-button', className]), ...rest, children: children }));
}

const selector$2 = (s) => ({
    isInteractive: s.nodesDraggable || s.nodesConnectable || s.elementsSelectable,
    minZoomReached: s.transform[2] <= s.minZoom,
    maxZoomReached: s.transform[2] >= s.maxZoom,
    ariaLabelConfig: s.ariaLabelConfig,
});
function ControlsComponent({ style, showZoom = true, showFitView = true, showInteractive = true, fitViewOptions, onZoomIn, onZoomOut, onFitView, onInteractiveChange, className, children, position = 'bottom-left', orientation = 'vertical', 'aria-label': ariaLabel, }) {
    const store = useStoreApi();
    const { isInteractive, minZoomReached, maxZoomReached, ariaLabelConfig } = useStore$1(selector$2, shallow$1);
    const { zoomIn, zoomOut, fitView } = useReactFlow();
    const onZoomInHandler = () => {
        zoomIn();
        onZoomIn?.();
    };
    const onZoomOutHandler = () => {
        zoomOut();
        onZoomOut?.();
    };
    const onFitViewHandler = () => {
        fitView(fitViewOptions);
        onFitView?.();
    };
    const onToggleInteractivity = () => {
        store.setState({
            nodesDraggable: !isInteractive,
            nodesConnectable: !isInteractive,
            elementsSelectable: !isInteractive,
        });
        onInteractiveChange?.(!isInteractive);
    };
    const orientationClass = orientation === 'horizontal' ? 'horizontal' : 'vertical';
    return (jsxs(Panel, { className: cc(['react-flow__controls', orientationClass, className]), position: position, style: style, "data-testid": "rf__controls", "aria-label": ariaLabel ?? ariaLabelConfig['controls.ariaLabel'], children: [showZoom && (jsxs(Fragment, { children: [jsx(ControlButton, { onClick: onZoomInHandler, className: "react-flow__controls-zoomin", title: ariaLabelConfig['controls.zoomIn.ariaLabel'], "aria-label": ariaLabelConfig['controls.zoomIn.ariaLabel'], disabled: maxZoomReached, children: jsx(PlusIcon, {}) }), jsx(ControlButton, { onClick: onZoomOutHandler, className: "react-flow__controls-zoomout", title: ariaLabelConfig['controls.zoomOut.ariaLabel'], "aria-label": ariaLabelConfig['controls.zoomOut.ariaLabel'], disabled: minZoomReached, children: jsx(MinusIcon, {}) })] })), showFitView && (jsx(ControlButton, { className: "react-flow__controls-fitview", onClick: onFitViewHandler, title: ariaLabelConfig['controls.fitView.ariaLabel'], "aria-label": ariaLabelConfig['controls.fitView.ariaLabel'], children: jsx(FitViewIcon, {}) })), showInteractive && (jsx(ControlButton, { className: "react-flow__controls-interactive", onClick: onToggleInteractivity, title: ariaLabelConfig['controls.interactive.ariaLabel'], "aria-label": ariaLabelConfig['controls.interactive.ariaLabel'], children: isInteractive ? jsx(UnlockIcon, {}) : jsx(LockIcon, {}) })), children] }));
}
ControlsComponent.displayName = 'Controls';
/**
 * The `<Controls />` component renders a small panel that contains convenient
 * buttons to zoom in, zoom out, fit the view, and lock the viewport.
 *
 * @public
 * @example
 *```tsx
 *import { ReactFlow, Controls } from '@xyflow/react'
 *
 *export default function Flow() {
 *  return (
 *    <ReactFlow nodes={[...]} edges={[...]}>
 *      <Controls />
 *    </ReactFlow>
 *  )
 *}
 *```
 *
 * @remarks To extend or customise the controls, you can use the [`<ControlButton />`](/api-reference/components/control-button) component
 *
 */
memo(ControlsComponent);

function MiniMapNodeComponent({ id, x, y, width, height, style, color, strokeColor, strokeWidth, className, borderRadius, shapeRendering, selected, onClick, }) {
    const { background, backgroundColor } = style || {};
    const fill = (color || background || backgroundColor);
    return (jsx("rect", { className: cc(['react-flow__minimap-node', { selected }, className]), x: x, y: y, rx: borderRadius, ry: borderRadius, width: width, height: height, style: {
            fill,
            stroke: strokeColor,
            strokeWidth,
        }, shapeRendering: shapeRendering, onClick: onClick ? (event) => onClick(event, id) : undefined }));
}
const MiniMapNode = memo(MiniMapNodeComponent);

const selectorNodeIds = (s) => s.nodes.map((node) => node.id);
const getAttrFunction = (func) => func instanceof Function ? func : () => func;
function MiniMapNodes({ nodeStrokeColor, nodeColor, nodeClassName = '', nodeBorderRadius = 5, nodeStrokeWidth, 
/*
 * We need to rename the prop to be `CapitalCase` so that JSX will render it as
 * a component properly.
 */
nodeComponent: NodeComponent = MiniMapNode, onClick, }) {
    const nodeIds = useStore$1(selectorNodeIds, shallow$1);
    const nodeColorFunc = getAttrFunction(nodeColor);
    const nodeStrokeColorFunc = getAttrFunction(nodeStrokeColor);
    const nodeClassNameFunc = getAttrFunction(nodeClassName);
    const shapeRendering = typeof window === 'undefined' || !!window.chrome ? 'crispEdges' : 'geometricPrecision';
    return (jsx(Fragment, { children: nodeIds.map((nodeId) => (
        /*
         * The split of responsibilities between MiniMapNodes and
         * NodeComponentWrapper may appear weird. However, it’s designed to
         * minimize the cost of updates when individual nodes change.
         *
         * For more details, see a similar commit in `NodeRenderer/index.tsx`.
         */
        jsx(NodeComponentWrapper, { id: nodeId, nodeColorFunc: nodeColorFunc, nodeStrokeColorFunc: nodeStrokeColorFunc, nodeClassNameFunc: nodeClassNameFunc, nodeBorderRadius: nodeBorderRadius, nodeStrokeWidth: nodeStrokeWidth, NodeComponent: NodeComponent, onClick: onClick, shapeRendering: shapeRendering }, nodeId))) }));
}
function NodeComponentWrapperInner({ id, nodeColorFunc, nodeStrokeColorFunc, nodeClassNameFunc, nodeBorderRadius, nodeStrokeWidth, shapeRendering, NodeComponent, onClick, }) {
    const { node, x, y, width, height } = useStore$1((s) => {
        const node = s.nodeLookup.get(id);
        if (!node) {
            return { node: undefined, x: 0, y: 0, width: 0, height: 0 };
        }
        const userNode = node.internals.userNode;
        const { x, y } = node.internals.positionAbsolute;
        const { width, height } = getNodeDimensions(userNode);
        return {
            node: userNode,
            x,
            y,
            width,
            height,
        };
    }, shallow$1);
    if (!node || node.hidden || !nodeHasDimensions(node)) {
        return null;
    }
    return (jsx(NodeComponent, { x: x, y: y, width: width, height: height, style: node.style, selected: !!node.selected, className: nodeClassNameFunc(node), color: nodeColorFunc(node), borderRadius: nodeBorderRadius, strokeColor: nodeStrokeColorFunc(node), strokeWidth: nodeStrokeWidth, shapeRendering: shapeRendering, onClick: onClick, id: node.id }));
}
const NodeComponentWrapper = memo(NodeComponentWrapperInner);
var MiniMapNodes$1 = memo(MiniMapNodes);

const defaultWidth = 200;
const defaultHeight = 150;
const filterHidden = (node) => !node.hidden;
const selector$1 = (s) => {
    const viewBB = {
        x: -s.transform[0] / s.transform[2],
        y: -s.transform[1] / s.transform[2],
        width: s.width / s.transform[2],
        height: s.height / s.transform[2],
    };
    return {
        viewBB,
        boundingRect: s.nodeLookup.size > 0
            ? getBoundsOfRects(getInternalNodesBounds(s.nodeLookup, { filter: filterHidden }), viewBB)
            : viewBB,
        rfId: s.rfId,
        panZoom: s.panZoom,
        translateExtent: s.translateExtent,
        flowWidth: s.width,
        flowHeight: s.height,
        ariaLabelConfig: s.ariaLabelConfig,
    };
};
const ARIA_LABEL_KEY = 'react-flow__minimap-desc';
function MiniMapComponent({ style, className, nodeStrokeColor, nodeColor, nodeClassName = '', nodeBorderRadius = 5, nodeStrokeWidth, 
/*
 * We need to rename the prop to be `CapitalCase` so that JSX will render it as
 * a component properly.
 */
nodeComponent, bgColor, maskColor, maskStrokeColor, maskStrokeWidth, position = 'bottom-right', onClick, onNodeClick, pannable = false, zoomable = false, ariaLabel, inversePan, zoomStep = 1, offsetScale = 5, }) {
    const store = useStoreApi();
    const svg = useRef(null);
    const { boundingRect, viewBB, rfId, panZoom, translateExtent, flowWidth, flowHeight, ariaLabelConfig } = useStore$1(selector$1, shallow$1);
    const elementWidth = style?.width ?? defaultWidth;
    const elementHeight = style?.height ?? defaultHeight;
    const scaledWidth = boundingRect.width / elementWidth;
    const scaledHeight = boundingRect.height / elementHeight;
    const viewScale = Math.max(scaledWidth, scaledHeight);
    const viewWidth = viewScale * elementWidth;
    const viewHeight = viewScale * elementHeight;
    const offset = offsetScale * viewScale;
    const x = boundingRect.x - (viewWidth - boundingRect.width) / 2 - offset;
    const y = boundingRect.y - (viewHeight - boundingRect.height) / 2 - offset;
    const width = viewWidth + offset * 2;
    const height = viewHeight + offset * 2;
    const labelledBy = `${ARIA_LABEL_KEY}-${rfId}`;
    const viewScaleRef = useRef(0);
    const minimapInstance = useRef();
    viewScaleRef.current = viewScale;
    useEffect(() => {
        if (svg.current && panZoom) {
            minimapInstance.current = XYMinimap({
                domNode: svg.current,
                panZoom,
                getTransform: () => store.getState().transform,
                getViewScale: () => viewScaleRef.current,
            });
            return () => {
                minimapInstance.current?.destroy();
            };
        }
    }, [panZoom]);
    useEffect(() => {
        minimapInstance.current?.update({
            translateExtent,
            width: flowWidth,
            height: flowHeight,
            inversePan,
            pannable,
            zoomStep,
            zoomable,
        });
    }, [pannable, zoomable, inversePan, zoomStep, translateExtent, flowWidth, flowHeight]);
    const onSvgClick = onClick
        ? (event) => {
            const [x, y] = minimapInstance.current?.pointer(event) || [0, 0];
            onClick(event, { x, y });
        }
        : undefined;
    const onSvgNodeClick = onNodeClick
        ? useCallback((event, nodeId) => {
            const node = store.getState().nodeLookup.get(nodeId).internals.userNode;
            onNodeClick(event, node);
        }, [])
        : undefined;
    const _ariaLabel = ariaLabel ?? ariaLabelConfig['minimap.ariaLabel'];
    return (jsx(Panel, { position: position, style: {
            ...style,
            '--xy-minimap-background-color-props': typeof bgColor === 'string' ? bgColor : undefined,
            '--xy-minimap-mask-background-color-props': typeof maskColor === 'string' ? maskColor : undefined,
            '--xy-minimap-mask-stroke-color-props': typeof maskStrokeColor === 'string' ? maskStrokeColor : undefined,
            '--xy-minimap-mask-stroke-width-props': typeof maskStrokeWidth === 'number' ? maskStrokeWidth * viewScale : undefined,
            '--xy-minimap-node-background-color-props': typeof nodeColor === 'string' ? nodeColor : undefined,
            '--xy-minimap-node-stroke-color-props': typeof nodeStrokeColor === 'string' ? nodeStrokeColor : undefined,
            '--xy-minimap-node-stroke-width-props': typeof nodeStrokeWidth === 'number' ? nodeStrokeWidth : undefined,
        }, className: cc(['react-flow__minimap', className]), "data-testid": "rf__minimap", children: jsxs("svg", { width: elementWidth, height: elementHeight, viewBox: `${x} ${y} ${width} ${height}`, className: "react-flow__minimap-svg", role: "img", "aria-labelledby": labelledBy, ref: svg, onClick: onSvgClick, children: [_ariaLabel && jsx("title", { id: labelledBy, children: _ariaLabel }), jsx(MiniMapNodes$1, { onClick: onSvgNodeClick, nodeColor: nodeColor, nodeStrokeColor: nodeStrokeColor, nodeBorderRadius: nodeBorderRadius, nodeClassName: nodeClassName, nodeStrokeWidth: nodeStrokeWidth, nodeComponent: nodeComponent }), jsx("path", { className: "react-flow__minimap-mask", d: `M${x - offset},${y - offset}h${width + offset * 2}v${height + offset * 2}h${-width - offset * 2}z
        M${viewBB.x},${viewBB.y}h${viewBB.width}v${viewBB.height}h${-viewBB.width}z`, fillRule: "evenodd", pointerEvents: "none" })] }) }));
}
MiniMapComponent.displayName = 'MiniMap';
/**
 * The `<MiniMap />` component can be used to render an overview of your flow. It
 * renders each node as an SVG element and visualizes where the current viewport is
 * in relation to the rest of the flow.
 *
 * @public
 * @example
 *
 * ```jsx
 *import { ReactFlow, MiniMap } from '@xyflow/react';
 *
 *export default function Flow() {
 *  return (
 *    <ReactFlow nodes={[...]} edges={[...]}>
 *      <MiniMap nodeStrokeWidth={3} />
 *    </ReactFlow>
 *  );
 *}
 *```
 */
const MiniMap = memo(MiniMapComponent);

const scaleSelector = (calculateScale) => (store) => calculateScale ? `${Math.max(1 / store.transform[2], 1)}` : undefined;
const defaultPositions = {
    [ResizeControlVariant.Line]: 'right',
    [ResizeControlVariant.Handle]: 'bottom-right',
};
function ResizeControl({ nodeId, position, variant = ResizeControlVariant.Handle, className, style = undefined, children, color, minWidth = 10, minHeight = 10, maxWidth = Number.MAX_VALUE, maxHeight = Number.MAX_VALUE, keepAspectRatio = false, resizeDirection, autoScale = true, shouldResize, onResizeStart, onResize, onResizeEnd, }) {
    const contextNodeId = useNodeId();
    const id = typeof nodeId === 'string' ? nodeId : contextNodeId;
    const store = useStoreApi();
    const resizeControlRef = useRef(null);
    const isHandleControl = variant === ResizeControlVariant.Handle;
    const scale = useStore$1(useCallback(scaleSelector(isHandleControl && autoScale), [isHandleControl, autoScale]), shallow$1);
    const resizer = useRef(null);
    const controlPosition = position ?? defaultPositions[variant];
    useEffect(() => {
        if (!resizeControlRef.current || !id) {
            return;
        }
        if (!resizer.current) {
            resizer.current = XYResizer({
                domNode: resizeControlRef.current,
                nodeId: id,
                getStoreItems: () => {
                    const { nodeLookup, transform, snapGrid, snapToGrid, nodeOrigin, domNode } = store.getState();
                    return {
                        nodeLookup,
                        transform,
                        snapGrid,
                        snapToGrid,
                        nodeOrigin,
                        paneDomNode: domNode,
                    };
                },
                onChange: (change, childChanges) => {
                    const { triggerNodeChanges, nodeLookup, parentLookup, nodeOrigin } = store.getState();
                    const changes = [];
                    const nextPosition = { x: change.x, y: change.y };
                    const node = nodeLookup.get(id);
                    if (node && node.expandParent && node.parentId) {
                        const origin = node.origin ?? nodeOrigin;
                        const width = change.width ?? node.measured.width ?? 0;
                        const height = change.height ?? node.measured.height ?? 0;
                        const child = {
                            id: node.id,
                            parentId: node.parentId,
                            rect: {
                                width,
                                height,
                                ...evaluateAbsolutePosition({
                                    x: change.x ?? node.position.x,
                                    y: change.y ?? node.position.y,
                                }, { width, height }, node.parentId, nodeLookup, origin),
                            },
                        };
                        const parentExpandChanges = handleExpandParent([child], nodeLookup, parentLookup, nodeOrigin);
                        changes.push(...parentExpandChanges);
                        /*
                         * when the parent was expanded by the child node, its position will be clamped at
                         * 0,0 when node origin is 0,0 and to width, height if it's 1,1
                         */
                        nextPosition.x = change.x ? Math.max(origin[0] * width, change.x) : undefined;
                        nextPosition.y = change.y ? Math.max(origin[1] * height, change.y) : undefined;
                    }
                    if (nextPosition.x !== undefined && nextPosition.y !== undefined) {
                        const positionChange = {
                            id,
                            type: 'position',
                            position: { ...nextPosition },
                        };
                        changes.push(positionChange);
                    }
                    if (change.width !== undefined && change.height !== undefined) {
                        const setAttributes = !resizeDirection ? true : resizeDirection === 'horizontal' ? 'width' : 'height';
                        const dimensionChange = {
                            id,
                            type: 'dimensions',
                            resizing: true,
                            setAttributes,
                            dimensions: {
                                width: change.width,
                                height: change.height,
                            },
                        };
                        changes.push(dimensionChange);
                    }
                    for (const childChange of childChanges) {
                        const positionChange = {
                            ...childChange,
                            type: 'position',
                        };
                        changes.push(positionChange);
                    }
                    triggerNodeChanges(changes);
                },
                onEnd: ({ width, height }) => {
                    const dimensionChange = {
                        id: id,
                        type: 'dimensions',
                        resizing: false,
                        dimensions: {
                            width,
                            height,
                        },
                    };
                    store.getState().triggerNodeChanges([dimensionChange]);
                },
            });
        }
        resizer.current.update({
            controlPosition,
            boundaries: {
                minWidth,
                minHeight,
                maxWidth,
                maxHeight,
            },
            keepAspectRatio,
            resizeDirection,
            onResizeStart,
            onResize,
            onResizeEnd,
            shouldResize,
        });
        return () => {
            resizer.current?.destroy();
        };
    }, [
        controlPosition,
        minWidth,
        minHeight,
        maxWidth,
        maxHeight,
        keepAspectRatio,
        onResizeStart,
        onResize,
        onResizeEnd,
        shouldResize,
    ]);
    const positionClassNames = controlPosition.split('-');
    return (jsx("div", { className: cc(['react-flow__resize-control', 'nodrag', ...positionClassNames, variant, className]), ref: resizeControlRef, style: {
            ...style,
            scale,
            ...(color && { [isHandleControl ? 'backgroundColor' : 'borderColor']: color }),
        }, children: children }));
}
/**
 * To create your own resizing UI, you can use the `NodeResizeControl` component where you can pass children (such as icons).
 * @public
 *
 */
memo(ResizeControl);

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


const ArrowDown = createLucideIcon("ArrowDown", [
  ["path", { d: "M12 5v14", key: "s699le" }],
  ["path", { d: "m19 12-7 7-7-7", key: "1idqje" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ArrowUp = createLucideIcon("ArrowUp", [
  ["path", { d: "m5 12 7-7 7 7", key: "hav0vg" }],
  ["path", { d: "M12 19V5", key: "x0mq9r" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ChevronDown = createLucideIcon("ChevronDown", [
  ["path", { d: "m6 9 6 6 6-6", key: "qrunsl" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const ChevronLeft = createLucideIcon("ChevronLeft", [
  ["path", { d: "m15 18-6-6 6-6", key: "1wnfg3" }]
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


const CornerDownRight = createLucideIcon("CornerDownRight", [
  ["polyline", { points: "15 10 20 15 15 20", key: "1q7qjw" }],
  ["path", { d: "M4 4v7a4 4 0 0 0 4 4h12", key: "z08zvw" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const EyeOff = createLucideIcon("EyeOff", [
  [
    "path",
    {
      d: "M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49",
      key: "ct8e1f"
    }
  ],
  ["path", { d: "M14.084 14.158a3 3 0 0 1-4.242-4.242", key: "151rxh" }],
  [
    "path",
    {
      d: "M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143",
      key: "13bj9a"
    }
  ],
  ["path", { d: "m2 2 20 20", key: "1ooewy" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Layers = createLucideIcon("Layers", [
  [
    "path",
    {
      d: "M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z",
      key: "zw3jo"
    }
  ],
  [
    "path",
    {
      d: "M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12",
      key: "1wduqc"
    }
  ],
  [
    "path",
    {
      d: "M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17",
      key: "kqbvx6"
    }
  ]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Maximize2 = createLucideIcon("Maximize2", [
  ["polyline", { points: "15 3 21 3 21 9", key: "mznyad" }],
  ["polyline", { points: "9 21 3 21 3 15", key: "1avn1i" }],
  ["line", { x1: "21", x2: "14", y1: "3", y2: "10", key: "ota7mn" }],
  ["line", { x1: "3", x2: "10", y1: "21", y2: "14", key: "1atl0r" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Minimize2 = createLucideIcon("Minimize2", [
  ["polyline", { points: "4 14 10 14 10 20", key: "11kfnr" }],
  ["polyline", { points: "20 10 14 10 14 4", key: "rlmsce" }],
  ["line", { x1: "14", x2: "21", y1: "10", y2: "3", key: "o5lafz" }],
  ["line", { x1: "3", x2: "10", y1: "21", y2: "14", key: "1atl0r" }]
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


const X = createLucideIcon("X", [
  ["path", { d: "M18 6 6 18", key: "1bl5f8" }],
  ["path", { d: "m6 6 12 12", key: "d8bk6v" }]
]);

/**
 * @license lucide-react v0.469.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */


const Zap = createLucideIcon("Zap", [
  [
    "path",
    {
      d: "M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z",
      key: "1xq2db"
    }
  ]
]);

const isIterable = (obj) => Symbol.iterator in obj;
const hasIterableEntries = (value) => (
  // HACK: avoid checking entries type
  "entries" in value
);
const compareEntries = (valueA, valueB) => {
  const mapA = valueA instanceof Map ? valueA : new Map(valueA.entries());
  const mapB = valueB instanceof Map ? valueB : new Map(valueB.entries());
  if (mapA.size !== mapB.size) {
    return false;
  }
  for (const [key, value] of mapA) {
    if (!mapB.has(key) || !Object.is(value, mapB.get(key))) {
      return false;
    }
  }
  return true;
};
const compareIterables = (valueA, valueB) => {
  const iteratorA = valueA[Symbol.iterator]();
  const iteratorB = valueB[Symbol.iterator]();
  let nextA = iteratorA.next();
  let nextB = iteratorB.next();
  while (!nextA.done && !nextB.done) {
    if (!Object.is(nextA.value, nextB.value)) {
      return false;
    }
    nextA = iteratorA.next();
    nextB = iteratorB.next();
  }
  return !!nextA.done && !!nextB.done;
};
function shallow(valueA, valueB) {
  if (Object.is(valueA, valueB)) {
    return true;
  }
  if (typeof valueA !== "object" || valueA === null || typeof valueB !== "object" || valueB === null) {
    return false;
  }
  if (Object.getPrototypeOf(valueA) !== Object.getPrototypeOf(valueB)) {
    return false;
  }
  if (isIterable(valueA) && isIterable(valueB)) {
    if (hasIterableEntries(valueA) && hasIterableEntries(valueB)) {
      return compareEntries(valueA, valueB);
    }
    return compareIterables(valueA, valueB);
  }
  return compareEntries(
    { entries: () => Object.entries(valueA) },
    { entries: () => Object.entries(valueB) }
  );
}

function useShallow(selector) {
  const prev = React__default.useRef(void 0);
  return (state) => {
    const next = selector(state);
    return shallow(prev.current, next) ? prev.current : prev.current = next;
  };
}

const createStoreImpl = (createState) => {
  let state;
  const listeners = /* @__PURE__ */ new Set();
  const setState = (partial, replace) => {
    const nextState = typeof partial === "function" ? partial(state) : partial;
    if (!Object.is(nextState, state)) {
      const previousState = state;
      state = (replace != null ? replace : typeof nextState !== "object" || nextState === null) ? nextState : Object.assign({}, state, nextState);
      listeners.forEach((listener) => listener(state, previousState));
    }
  };
  const getState = () => state;
  const getInitialState = () => initialState;
  const subscribe = (listener) => {
    listeners.add(listener);
    return () => listeners.delete(listener);
  };
  const api = { setState, getState, getInitialState, subscribe };
  const initialState = state = createState(setState, getState, api);
  return api;
};
const createStore = ((createState) => createState ? createStoreImpl(createState) : createStoreImpl);

const identity = (arg) => arg;
function useStore(api, selector = identity) {
  const slice = React__default.useSyncExternalStore(
    api.subscribe,
    React__default.useCallback(() => selector(api.getState()), [api, selector]),
    React__default.useCallback(() => selector(api.getInitialState()), [api, selector])
  );
  React__default.useDebugValue(slice);
  return slice;
}
const createImpl = (createState) => {
  const api = createStore(createState);
  const useBoundStore = (selector) => useStore(api, selector);
  Object.assign(useBoundStore, api);
  return useBoundStore;
};
const create = ((createState) => createState ? createImpl(createState) : createImpl);

const propertyToComponent = /* @__PURE__ */ new Map();
function indexComponentProperties(c) {
  for (const p of Object.values(c.properties)) {
    propertyToComponent.set(p.uid, c.uid);
  }
}
function unindexComponentProperties(c) {
  for (const p of Object.values(c.properties)) {
    propertyToComponent.delete(p.uid);
  }
}
const useStructural = create((set) => ({
  components: /* @__PURE__ */ new Map(),
  componentsByPath: /* @__PURE__ */ new Map(),
  edges: /* @__PURE__ */ new Map(),
  setNodes: (comps, edges) => {
    const cByUid = /* @__PURE__ */ new Map();
    const cByPath = /* @__PURE__ */ new Map();
    propertyToComponent.clear();
    const walk = (c) => {
      cByUid.set(c.uid, c);
      cByPath.set(c.path, c);
      indexComponentProperties(c);
      c.children?.forEach(walk);
    };
    comps.forEach(walk);
    const eByUid = /* @__PURE__ */ new Map();
    edges.forEach((e) => eByUid.set(e.uid, e));
    set({ components: cByUid, componentsByPath: cByPath, edges: eByUid });
  },
  upsertComponent: (c) => set((s) => {
    const components = new Map(s.components);
    const componentsByPath = new Map(s.componentsByPath);
    const prev = components.get(c.uid);
    if (prev) unindexComponentProperties(prev);
    components.set(c.uid, c);
    componentsByPath.set(c.path, c);
    indexComponentProperties(c);
    return { components, componentsByPath };
  }),
  removeComponent: (uid) => set((s) => {
    const components = new Map(s.components);
    const old = components.get(uid);
    if (old) unindexComponentProperties(old);
    components.delete(uid);
    const componentsByPath = new Map(s.componentsByPath);
    if (old) componentsByPath.delete(old.path);
    const edges = new Map(s.edges);
    for (const [eid, e] of edges) {
      if (e.sourceUid === uid || e.targetUid === uid) edges.delete(eid);
    }
    return { components, componentsByPath, edges };
  }),
  upsertEdge: (e) => set((s) => {
    const edges = new Map(s.edges);
    edges.set(e.uid, e);
    return { edges };
  }),
  removeEdge: (uid) => set((s) => {
    const edges = new Map(s.edges);
    edges.delete(uid);
    return { edges };
  })
}));
const propertyDataType = /* @__PURE__ */ new Map();
function makeRafBump(set) {
  let scheduled = false;
  return () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      set((s) => ({ version: s.version + 1 }));
    });
  };
}
const useStatusFlags = create((set, get) => {
  const bump = makeRafBump(set);
  return {
    flags: /* @__PURE__ */ new Map(),
    version: 0,
    applyStatus: (uids, flags) => {
      const m = get().flags;
      for (let i = 0; i < uids.length; i++) m.set(uids[i], flags[i]);
      bump();
    },
    reset: () => set({ flags: /* @__PURE__ */ new Map(), version: 0 })
  };
});
const useSchemaVersion = create((set) => ({
  version: 0,
  bump: () => set((s) => ({ version: s.version + 1 }))
}));
function loadSchemaIndices(properties) {
  propertyDataType.clear();
  const seeded = /* @__PURE__ */ new Map();
  for (const p of properties) {
    propertyDataType.set(p.uid, p.dataType);
    seeded.set(p.uid, p.statusFlags >>> 0);
  }
  useStatusFlags.setState({ flags: seeded, version: useStatusFlags.getState().version + 1 });
  useSchemaVersion.getState().bump();
}
const useValues = create((set, get) => {
  const bump = makeRafBump(set);
  return {
    values: /* @__PURE__ */ new Map(),
    version: 0,
    // Mutate in place immediately; coalesce the version bump to one rAF (see
    // makeRafBump). Components select specific UIDs from the Map and observe
    // changes on version bump — now at most once per frame regardless of how
    // many WS frames landed since the last paint.
    apply: (uids, vs) => {
      const m = get().values;
      for (let i = 0; i < uids.length; i++) {
        m.set(uids[i], vs[i]);
      }
      bump();
    },
    reset: () => set({ values: /* @__PURE__ */ new Map(), version: 0 })
  };
});

const store = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  loadSchemaIndices,
  propertyDataType,
  propertyToComponent,
  useSchemaVersion,
  useStatusFlags,
  useStructural,
  useValues
}, Symbol.toStringTag, { value: 'Module' }));

const CATEGORY_INPUT = 0;
const CATEGORY_OUTPUT = 1;
const CATEGORY_CONFIG = 2;
const DATATYPE_NUMBER = 0;
const DATATYPE_BOOL = 1;
const DATATYPE_STRING = 2;
const ROLE_NORMAL = 0;
const ROLE_STATUS = 1;
const STATUS_OVERRIDDEN = 1 << 5;
const TYPE_BOOL = 1;
const TYPE_U32 = 16;
const TYPE_I32 = 17;
const TYPE_F32 = 18;
const TYPE_U64 = 32;
const TYPE_I64 = 33;
const TYPE_F64 = 34;
const TYPE_STR = 48;
const TYPE_STATUS = 64;
const MSG_UPDATE = 1;
const MSG_SNAPSHOT = 2;

const RS = "";
const US = "";
const GS = "";
const FS = "";
const FACET_PROP = "__facets";
function parseFacet(raw) {
  const out = /* @__PURE__ */ new Map();
  if (!raw) return out;
  for (const rec of raw.split(RS)) {
    if (!rec) continue;
    const fields = rec.split(US);
    const uid = Number(fields[0]);
    if (!Number.isFinite(uid)) continue;
    const f = {};
    for (let i = 1; i < fields.length; i++) {
      const fld = fields[i];
      if (!fld) continue;
      const v = fld.slice(1);
      switch (fld[0]) {
        case "l":
          f.label = v;
          break;
        case "u":
          f.unit = v;
          break;
        case "d":
          f.decimals = Number(v);
          break;
        case "n":
          f.min = Number(v);
          break;
        case "x":
          f.max = Number(v);
          break;
        case "h":
          f.hidden = v !== "0";
          break;
        case "r":
          f.order = Number(v);
          break;
        case "a":
          f.action = v;
          break;
        case "e":
          f.expose = v === "o" ? "output" : "input";
          break;
        case "c":
          f.childComponent = Number(v);
          break;
        case "f":
          f.facetProp = Number(v);
          break;
        case "o":
          f.aliases = v.split(GS).map((o) => {
            const j = o.indexOf(FS);
            return j < 0 ? { code: Number(o), label: o } : { code: Number(o.slice(0, j)), label: o.slice(j + 1) };
          });
          break;
      }
    }
    out.set(uid, f);
  }
  return out;
}
function serializeFacet(facet) {
  const recs = [];
  for (const [uid, f] of facet) {
    const fields = [String(uid)];
    if (f.label) fields.push("l" + f.label);
    if (f.unit) fields.push("u" + f.unit);
    if (f.decimals != null) fields.push("d" + f.decimals);
    if (f.min != null) fields.push("n" + f.min);
    if (f.max != null) fields.push("x" + f.max);
    if (f.hidden) fields.push("h1");
    if (f.order != null) fields.push("r" + f.order);
    if (f.action) fields.push("a" + f.action);
    if (f.expose) fields.push("e" + (f.expose === "output" ? "o" : "i"));
    if (f.childComponent != null) fields.push("c" + f.childComponent);
    if (f.facetProp != null) fields.push("f" + f.facetProp);
    if (f.aliases && f.aliases.length) {
      fields.push("o" + f.aliases.map((a) => a.code + FS + a.label).join(GS));
    }
    if (fields.length > 1) recs.push(fields.join(US));
  }
  return recs.join(RS);
}
const mapUid = (m, k) => (m instanceof Map ? m.get(k) : m[k]) ?? k;
function remapFacetUids(raw, compMap, propMap) {
  const facet = parseFacet(raw);
  if (facet.size === 0) return raw;
  const out = /* @__PURE__ */ new Map();
  for (const [propUid, f] of facet) {
    const nf = { ...f };
    if (nf.childComponent != null) nf.childComponent = mapUid(compMap, nf.childComponent);
    if (nf.facetProp != null) nf.facetProp = mapUid(propMap, nf.facetProp);
    out.set(mapUid(propMap, propUid), nf);
  }
  return serializeFacet(out);
}
const cache = /* @__PURE__ */ new Map();
function facetFor(componentUid, raw) {
  const key = raw ?? "";
  const hit = cache.get(componentUid);
  if (hit && hit.raw === key) return hit.parsed;
  const parsed = parseFacet(key);
  cache.set(componentUid, { raw: key, parsed });
  return parsed;
}
function rawFacet(properties) {
  const v = properties?.[FACET_PROP]?.value;
  return typeof v === "string" ? v : void 0;
}
function exposedPorts(facet) {
  const out = [];
  for (const [uid, f] of facet) {
    if (f.expose) out.push({ childUid: uid, side: f.expose, facet: f });
  }
  return out;
}
function parseAliasInput(s) {
  const out = [];
  for (const part of s.split(",")) {
    const t = part.trim();
    if (!t) continue;
    const j = t.indexOf("=");
    if (j < 0) continue;
    const code = Number(t.slice(0, j).trim());
    const label = t.slice(j + 1).trim();
    if (Number.isFinite(code) && label) out.push({ code, label });
  }
  return out;
}
function aliasLabel(aliases, value) {
  if (!aliases || aliases.length === 0) return void 0;
  const code = value === true ? 1 : value === false ? 0 : typeof value === "number" ? value : Number(value);
  return aliases.find((a) => a.code === code)?.label;
}

const fmtCell = (v, facet) => {
  if (v === void 0 || v === null) return "—";
  const al = aliasLabel(facet?.aliases, v);
  if (al) return al;
  let s;
  if (typeof v === "number" && facet?.decimals != null) s = v.toFixed(facet.decimals);
  else s = String(v);
  return facet?.unit ? `${s} ${facet.unit}` : s;
};
const catRank = (c) => c === CATEGORY_INPUT ? 0 : c === CATEGORY_OUTPUT ? 1 : 2;
const catLabel = (c) => c === CATEGORY_INPUT ? "Inputs" : c === CATEGORY_OUTPUT ? "Outputs" : "Config";
function ComponentTable({
  currentParentUid,
  selectedUids,
  onSelectRow,
  onDrillIn,
  onRowsChange
}) {
  const [showHidden, setShowHidden] = useState(false);
  const [query, setQuery] = useState("");
  const [dir, setDir] = useState(1);
  const scrollRef = useRef(null);
  const components = useStructural((s) => s.components);
  const allRows = useMemo(
    () => Array.from(components.values()).filter((c) => c.parent === currentParentUid),
    [components, currentParentUid]
  );
  const facets = useMemo(
    () => new Map(allRows.map((c) => [c.uid, facetFor(c.uid, rawFacet(c.properties))])),
    [allRows]
  );
  const fieldsFor = (c) => {
    const facet = facets.get(c.uid);
    const out = [];
    for (const [name, p] of Object.entries(c.properties)) {
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      const f = facet?.get(p.uid);
      if (!showHidden && f?.hidden) continue;
      out.push({ key: name, uid: p.uid, label: f?.label || name, category: p.category, exposed: false, facet: f });
    }
    if (facet) {
      for (const ep of exposedPorts(facet)) {
        const label = ep.facet.label || `port ${ep.childUid}`;
        out.push({
          key: `↪${label}`,
          uid: ep.childUid,
          label,
          category: ep.side === "input" ? CATEGORY_INPUT : CATEGORY_OUTPUT,
          exposed: true,
          facet: ep.facet
        });
      }
    }
    return out;
  };
  const q = query.trim().toLowerCase();
  const matches = (c) => {
    if (!q) return true;
    if ((c.name || c.type).toLowerCase().includes(q) || c.type.toLowerCase().includes(q)) return true;
    for (const f of fieldsFor(c)) {
      if (f.label.toLowerCase().includes(q) || f.key.toLowerCase().includes(q)) return true;
      if (f.facet?.aliases?.some((a) => a.label.toLowerCase().includes(q))) return true;
    }
    return false;
  };
  const rows = useMemo(
    () => allRows.filter(matches).sort((a, b) => (a.name || a.type).localeCompare(b.name || b.type) * dir),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allRows, q, dir, showHidden, facets]
  );
  const columns = useMemo(() => {
    const map = /* @__PURE__ */ new Map();
    for (const c of rows) {
      for (const f of fieldsFor(c)) {
        let e = map.get(f.key);
        if (!e) {
          e = { category: f.category, labels: /* @__PURE__ */ new Set(), exposed: f.exposed };
          map.set(f.key, e);
        }
        e.labels.add(f.label);
      }
    }
    return [...map.entries()].map(([key, e]) => ({
      key,
      category: e.category,
      exposed: e.exposed,
      header: e.labels.size === 1 ? [...e.labels][0] : key.replace(/^↪/, "")
    })).sort((a, b) => catRank(a.category) - catRank(b.category) || a.header.localeCompare(b.header));
  }, [rows, showHidden, facets]);
  const bands = useMemo(() => {
    const out = [];
    for (const col of columns) {
      const last = out[out.length - 1];
      if (last && last.category === col.category) last.count++;
      else out.push({ category: col.category, count: 1 });
    }
    return out;
  }, [columns]);
  const watchUids = useMemo(() => {
    const set = /* @__PURE__ */ new Set();
    for (const c of allRows) {
      for (const p of Object.values(c.properties))
        if ((p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL || p.systemRole === ROLE_STATUS) set.add(p.uid);
      for (const ep of exposedPorts(facets.get(c.uid) ?? /* @__PURE__ */ new Map())) set.add(ep.childUid);
    }
    return [...set];
  }, [allRows, facets]);
  const values = useValues(
    useShallow((s) => {
      const out = {};
      for (const uid of watchUids) out[uid] = s.values.get(uid);
      return out;
    })
  );
  useEffect(() => {
    onRowsChange(allRows.map((c) => c.uid));
    return () => onRowsChange([]);
  }, [allRows, onRowsChange]);
  const firstSel = selectedUids[0];
  useEffect(() => {
    if (firstSel == null) return;
    scrollRef.current?.querySelector(`[data-uid="${firstSel}"]`)?.scrollIntoView({ block: "nearest" });
  }, [firstSel, rows]);
  const sel = new Set(selectedUids);
  const orphans = q ? allRows.filter((c) => sel.has(c.uid) && !matches(c)) : [];
  const renderRow = (c) => {
    const isFolder = (c.childrenCount ?? 0) > 0;
    const byKey = new Map(fieldsFor(c).map((f) => [f.key, f]));
    return /* @__PURE__ */ jsxs(
      "tr",
      {
        "data-uid": c.uid,
        onClick: (e) => onSelectRow(c.uid, e.shiftKey || e.metaKey || e.ctrlKey),
        onDoubleClick: () => isFolder && onDrillIn(c.uid),
        style: {
          cursor: "pointer",
          background: sel.has(c.uid) ? "#2c3a55" : "transparent",
          borderBottom: "1px solid #1f232b"
        },
        children: [
          /* @__PURE__ */ jsx(Td, { sticky: true, children: /* @__PURE__ */ jsxs("span", { style: { display: "flex", alignItems: "center", gap: 4, fontWeight: 600 }, children: [
            isFolder && /* @__PURE__ */ jsx(Layers, { size: 12, color: "#9ecbff" }),
            /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb" }, children: c.name || c.type }),
            isFolder && /* @__PURE__ */ jsx(ChevronRight, { size: 12, color: "#5a6172" })
          ] }) }),
          columns.map((col) => {
            const f = byKey.get(col.key);
            if (!f) return /* @__PURE__ */ jsx(Td, { numeric: true, muted: true }, col.key);
            return /* @__PURE__ */ jsx(Td, { numeric: true, input: f.category === CATEGORY_INPUT, children: /* @__PURE__ */ jsxs("span", { style: { display: "inline-flex", alignItems: "center", gap: 3, justifyContent: "flex-end" }, children: [
              f.exposed && /* @__PURE__ */ jsx(CornerDownRight, { size: 10, color: "#7a8a9f" }),
              fmtCell(values[f.uid], f.facet)
            ] }) }, col.key);
          })
        ]
      },
      c.uid
    );
  };
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: "#15181e",
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        fontSize: 12,
        minWidth: 0
      },
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "6px 10px",
              borderBottom: "1px solid #2c313c",
              flexShrink: 0
            },
            children: [
              /* @__PURE__ */ jsx(
                "input",
                {
                  value: query,
                  onChange: (e) => setQuery(e.target.value),
                  placeholder: "search name, prop, alias…",
                  spellCheck: false,
                  style: {
                    flex: 1,
                    minWidth: 0,
                    background: "#222731",
                    color: "#cbd3e0",
                    border: "1px solid #2c313c",
                    borderRadius: 3,
                    padding: "4px 7px",
                    fontSize: 11,
                    fontFamily: "ui-monospace, monospace",
                    outline: "none"
                  }
                }
              ),
              /* @__PURE__ */ jsxs(
                "button",
                {
                  onClick: () => setDir((d) => d === 1 ? -1 : 1),
                  title: `Sort by name ${dir === 1 ? "ascending" : "descending"}`,
                  style: {
                    display: "flex",
                    alignItems: "center",
                    gap: 3,
                    background: "transparent",
                    border: "1px solid #2c313c",
                    borderRadius: 3,
                    color: "#8892a0",
                    cursor: "pointer",
                    padding: "3px 6px",
                    fontSize: 11,
                    flexShrink: 0
                  },
                  children: [
                    "name ",
                    dir === 1 ? /* @__PURE__ */ jsx(ArrowUp, { size: 11 }) : /* @__PURE__ */ jsx(ArrowDown, { size: 11 })
                  ]
                }
              ),
              /* @__PURE__ */ jsxs("label", { style: { display: "flex", alignItems: "center", gap: 4, color: "#8892a0", flexShrink: 0 }, children: [
                /* @__PURE__ */ jsx("input", { type: "checkbox", checked: showHidden, onChange: (e) => setShowHidden(e.target.checked) }),
                "hidden"
              ] })
            ]
          }
        ),
        /* @__PURE__ */ jsx("div", { ref: scrollRef, style: { overflow: "auto", flex: 1 }, children: rows.length === 0 && orphans.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: 12, color: "#5a6172" }, children: allRows.length === 0 ? "no components in this folder" : "no matches" }) : /* @__PURE__ */ jsxs("table", { style: { borderCollapse: "collapse", width: "100%", whiteSpace: "nowrap" }, children: [
          /* @__PURE__ */ jsxs("thead", { children: [
            /* @__PURE__ */ jsxs("tr", { style: { position: "sticky", top: 0, zIndex: 2 }, children: [
              /* @__PURE__ */ jsx(BandTh, { sticky: true }),
              bands.map((b, i) => /* @__PURE__ */ jsx(BandTh, { span: b.count, children: catLabel(b.category) }, i))
            ] }),
            /* @__PURE__ */ jsxs("tr", { style: { position: "sticky", top: 22, zIndex: 2 }, children: [
              /* @__PURE__ */ jsx(ColTh, { sticky: true, children: "name" }),
              columns.map((col) => /* @__PURE__ */ jsx(ColTh, { title: col.key.replace(/^↪/, ""), exposed: col.exposed, children: col.header }, col.key))
            ] })
          ] }),
          /* @__PURE__ */ jsxs("tbody", { children: [
            rows.map(renderRow),
            orphans.length > 0 && /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx("tr", { children: /* @__PURE__ */ jsxs(
                "td",
                {
                  colSpan: columns.length + 1,
                  style: {
                    padding: "5px 10px",
                    background: "#221a1a",
                    borderTop: "1px solid #3a2a2a",
                    borderBottom: "1px solid #3a2a2a",
                    color: "#c9a86a",
                    fontSize: 10,
                    textTransform: "uppercase",
                    letterSpacing: 0.4
                  },
                  children: [
                    "selected · filtered out (",
                    orphans.length,
                    ")"
                  ]
                }
              ) }),
              orphans.map(renderRow)
            ] })
          ] })
        ] }) })
      ]
    }
  );
}
function BandTh({ children, span, sticky }) {
  return /* @__PURE__ */ jsx(
    "th",
    {
      colSpan: span,
      style: {
        textAlign: "center",
        padding: "3px 10px",
        background: "#181c23",
        borderBottom: "1px solid #2c313c",
        borderLeft: span ? "1px solid #2c313c" : void 0,
        color: "#7a8aa0",
        fontWeight: 600,
        fontSize: 9,
        textTransform: "uppercase",
        letterSpacing: 0.5,
        ...sticky ? { position: "sticky", left: 0, zIndex: 1, textAlign: "left" } : {}
      },
      children
    }
  );
}
function ColTh({
  children,
  sticky,
  title,
  exposed
}) {
  return /* @__PURE__ */ jsx(
    "th",
    {
      title,
      style: {
        textAlign: sticky ? "left" : "right",
        padding: "4px 10px",
        background: "#1a1d24",
        borderBottom: "1px solid #2c313c",
        color: "#8892a0",
        fontWeight: 600,
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        ...sticky ? { position: "sticky", left: 0, zIndex: 1 } : {}
      },
      children: /* @__PURE__ */ jsxs("span", { style: { display: "inline-flex", alignItems: "center", gap: 3, justifyContent: "flex-end" }, children: [
        exposed && /* @__PURE__ */ jsx(CornerDownRight, { size: 10, color: "#7a8a9f" }),
        children
      ] })
    }
  );
}
function Td({
  children,
  numeric,
  muted,
  input,
  sticky
}) {
  return /* @__PURE__ */ jsx(
    "td",
    {
      style: {
        textAlign: numeric ? "right" : "left",
        padding: "4px 10px",
        color: muted ? "#5a6172" : input ? "#cbd3e0" : "#e6e8eb",
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        ...sticky ? { position: "sticky", left: 0, background: "inherit", zIndex: 1 } : {}
      },
      children
    }
  );
}

const EVENT_CAP = 500;
const events = [];
const eventsVersion = { v: 0 };
function recordEvent(kind, text) {
  events.push({ t: performance.now(), kind, text });
  if (events.length > EVENT_CAP) events.splice(0, events.length - EVENT_CAP);
  eventsVersion.v++;
}
function clearEvents() {
  events.length = 0;
  eventsVersion.v++;
}
const metrics = {
  // WS state
  wsConnected: false,
  reconnectCount: 0,
  sessionId: "",
  lastSeq: 0,
  // Counters since last 1s sample
  msgsThisSec: 0,
  bytesThisSec: 0,
  valuesThisSec: 0,
  framesThisSec: 0,
  // 1s rolling samples
  msgsPerSec: 0,
  bytesPerSec: 0,
  valuesPerSec: 0,
  framesPerSec: 0,
  // Frame timing
  fps: 0,
  frameMs: 0,
  maxFrameMs: 0,
  longFramesPerSec: 0,
  // Last binary frame stats
  lastFrameValues: 0,
  lastFrameSections: 0,
  lastFrameBytes: 0,
  // Wire parse time rolling avg (ms)
  parseAvgMs: 0,
  // Topology event counts
  topoAdded: 0,
  topoRemoved: 0,
  topoChanged: 0,
  // Viewport
  zoom: 1,
  panX: 0,
  panY: 0,
  // Last batch of select-change events from React Flow's onNodesChange. Captured here
  // so the ClickDebugger can show them next to each click without needing console.
  lastSelChange: "",
  lastSelChangeAt: 0,
  totalComponents: 0,
  domNodes: 0,
  domEdges: 0,
  // Bytes-per-sec sparkline, 30 samples deep.
  bytesSpark: new Float32Array(30)
};
function recordMessage(bytes, isBinary) {
  metrics.msgsThisSec++;
  metrics.bytesThisSec += bytes;
  if (isBinary) metrics.framesThisSec++;
}
function recordValueCount(n, sections, bytes) {
  metrics.valuesThisSec += n;
  metrics.lastFrameValues = n;
  metrics.lastFrameSections = sections;
  metrics.lastFrameBytes = bytes;
}
function recordParse(ms) {
  const alpha = 0.1;
  metrics.parseAvgMs = metrics.parseAvgMs === 0 ? ms : metrics.parseAvgMs * (1 - alpha) + ms * alpha;
}
function recordTopology(kind) {
  if (kind === "added") metrics.topoAdded++;
  else if (kind === "removed") metrics.topoRemoved++;
  else metrics.topoChanged++;
}
let lastSec = performance.now();
let frames = 0;
let lastFrameTs$1 = performance.now();
let frameMsAccum = 0;
let frameMsMax = 0;
let longFramesAccum = 0;
const LONG_FRAME_MS = 25;
function tickInstrumentation(now) {
  const dt = now - lastFrameTs$1;
  lastFrameTs$1 = now;
  frames++;
  frameMsAccum += dt;
  if (dt > frameMsMax) frameMsMax = dt;
  if (dt > LONG_FRAME_MS) longFramesAccum++;
  if (now - lastSec >= 1e3) {
    const sec = (now - lastSec) / 1e3;
    metrics.fps = frames / sec;
    metrics.frameMs = frameMsAccum / frames;
    metrics.maxFrameMs = frameMsMax;
    metrics.longFramesPerSec = longFramesAccum;
    frames = 0;
    frameMsAccum = 0;
    frameMsMax = 0;
    longFramesAccum = 0;
    lastSec = now;
    metrics.msgsPerSec = metrics.msgsThisSec;
    metrics.bytesPerSec = metrics.bytesThisSec;
    metrics.valuesPerSec = metrics.valuesThisSec;
    metrics.framesPerSec = metrics.framesThisSec;
    metrics.msgsThisSec = 0;
    metrics.bytesThisSec = 0;
    metrics.valuesThisSec = 0;
    metrics.framesThisSec = 0;
    const s = metrics.bytesSpark;
    s.copyWithin(0, 1);
    s[s.length - 1] = metrics.bytesPerSec;
    metrics.domNodes = document.querySelectorAll(".react-flow__node").length;
    metrics.domEdges = document.querySelectorAll(".react-flow__edge").length;
  }
}

const RING_LIFETIME_MS = 800;
const LOG_LIMIT = 8;
function describeTarget(el) {
  if (!el) return { nodeUid: null, targetTag: "<none>" };
  let kind = "pane";
  let node = el;
  let nodeEl = null;
  while (node) {
    if (node.classList?.contains("react-flow__handle")) kind = "handle";
    if (node.classList?.contains("react-flow__node")) {
      nodeEl = node;
      if (kind === "pane") kind = "node body";
      break;
    }
    if (node.classList?.contains("react-flow__pane")) kind = "pane";
    node = node.parentElement;
  }
  const uid = nodeEl?.dataset?.id ? Number(nodeEl.dataset.id) : null;
  return { nodeUid: Number.isFinite(uid) ? uid : null, targetTag: kind };
}
let nextId = 1;
function ClickDebugger() {
  const [events, setEvents] = useState([]);
  const [rings, setRings] = useState([]);
  const downRef = useRef(null);
  useStructural((s) => s.components);
  const rSelChange = useRef(null);
  useEffect(() => {
    let raf = 0;
    let lastTs = 0;
    const tick = () => {
      if (rSelChange.current && metrics.lastSelChangeAt !== lastTs) {
        lastTs = metrics.lastSelChangeAt;
        rSelChange.current.textContent = metrics.lastSelChange || "—";
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);
  useEffect(() => {
    const onDown = (e) => {
      const id = nextId++;
      const info = describeTarget(e.target);
      downRef.current = {
        x: e.clientX,
        y: e.clientY,
        ts: performance.now(),
        button: e.button,
        nodeUid: info.nodeUid,
        targetTag: info.targetTag
      };
      setRings((r) => [...r, { id, x: e.clientX, y: e.clientY, ts: performance.now() }]);
      setTimeout(() => setRings((r) => r.filter((x) => x.id !== id)), RING_LIFETIME_MS);
    };
    const onUp = (e) => {
      const d = downRef.current;
      if (!d) return;
      downRef.current = null;
      const dx = e.clientX - d.x;
      const dy = e.clientY - d.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      setTimeout(() => {
        const selected = document.querySelectorAll(".react-flow__node.selected").length;
        const name = d.nodeUid != null ? useStructural.getState().components.get(d.nodeUid)?.name ?? null : null;
        const ev = {
          id: nextId++,
          x: d.x,
          y: d.y,
          button: d.button,
          nodeName: name,
          nodeUid: d.nodeUid,
          targetTag: d.targetTag,
          distance: dist,
          duration: performance.now() - d.ts,
          selectedAfter: selected,
          ts: Date.now()
        };
        setEvents((es) => [ev, ...es].slice(0, LOG_LIMIT));
      }, 60);
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("pointerup", onUp, true);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("pointerup", onUp, true);
    };
  }, []);
  return /* @__PURE__ */ jsxs(Fragment, { children: [
    /* @__PURE__ */ jsx("div", { style: { position: "fixed", inset: 0, pointerEvents: "none", zIndex: 9998 }, children: rings.map((r) => /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          position: "absolute",
          left: r.x - 14,
          top: r.y - 14,
          width: 28,
          height: 28,
          borderRadius: 14,
          border: "2px solid #4a9eff",
          animation: "ce-ring-fade 800ms ease-out forwards",
          boxSizing: "border-box"
        }
      },
      r.id
    )) }),
    /* @__PURE__ */ jsxs(
      "div",
      {
        style: {
          position: "fixed",
          right: 12,
          bottom: 12,
          zIndex: 30,
          background: "rgba(20,23,30,0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          padding: 8,
          color: "#cbd3e0",
          fontSize: 10,
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
          minWidth: 320,
          maxWidth: 460
        },
        children: [
          /* @__PURE__ */ jsxs("div", { style: { color: "#8892a0", marginBottom: 4, display: "flex", gap: 8 }, children: [
            /* @__PURE__ */ jsxs("span", { children: [
              "click debugger · last ",
              LOG_LIMIT
            ] }),
            /* @__PURE__ */ jsxs("span", { style: { marginLeft: "auto", color: "#cbd3e0" }, children: [
              "rf: ",
              /* @__PURE__ */ jsx("span", { ref: rSelChange, children: "—" })
            ] })
          ] }),
          events.length === 0 && /* @__PURE__ */ jsx("div", { style: { color: "#5a6172" }, children: "click somewhere…" }),
          events.map((e) => {
            const buttonName = e.button === 0 ? "L" : e.button === 2 ? "R" : `M${e.button}`;
            const kind = e.distance < 4 ? "click" : "drag";
            const sel = e.selectedAfter > 0 ? `sel=${e.selectedAfter}` : "sel=0";
            const onWhat = e.nodeName != null ? `${e.nodeName}  [${e.targetTag}]` : `(${e.targetTag})`;
            return /* @__PURE__ */ jsxs("div", { style: { display: "flex", gap: 6 }, children: [
              /* @__PURE__ */ jsx("span", { style: { color: e.button === 2 ? "#f59e0b" : "#9ecbff", width: 14 }, children: buttonName }),
              /* @__PURE__ */ jsx("span", { style: { width: 36 }, children: kind }),
              /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", width: 70 }, children: [
                e.distance.toFixed(0),
                "px · ",
                e.duration.toFixed(0),
                "ms"
              ] }),
              /* @__PURE__ */ jsx("span", { style: { color: e.selectedAfter > 0 ? "#4ade80" : "#ef4444", width: 38 }, children: sel }),
              /* @__PURE__ */ jsx(
                "span",
                {
                  style: {
                    flex: 1,
                    minWidth: 0,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    color: e.nodeName ? "#e6e8eb" : "#9aa3b2"
                  },
                  title: e.nodeUid != null ? `uid ${e.nodeUid}` : void 0,
                  children: onWhat
                }
              )
            ] }, e.id);
          })
        ]
      }
    ),
    /* @__PURE__ */ jsx("style", { children: `
        @keyframes ce-ring-fade {
          0%   { opacity: 1; transform: scale(0.7); }
          80%  { opacity: 0.7; transform: scale(1.6); }
          100% { opacity: 0;   transform: scale(2);   }
        }
      ` })
  ] });
}

class Ring {
  buf;
  i = 0;
  filled = 0;
  constructor(size) {
    this.buf = new Float64Array(size);
  }
  push(v) {
    this.buf[this.i] = v;
    this.i = (this.i + 1) % this.buf.length;
    if (this.filled < this.buf.length) this.filled++;
  }
  /** Sorted copy of the live values. */
  sorted() {
    const out = [];
    for (let k = 0; k < this.filled; k++) out.push(this.buf[k]);
    out.sort((a, b) => a - b);
    return out;
  }
  get count() {
    return this.filled;
  }
  clear() {
    this.i = 0;
    this.filled = 0;
  }
}
function pct(sorted, p) {
  if (sorted.length === 0) return 0;
  const idx = Math.min(sorted.length - 1, Math.floor(p / 100 * sorted.length));
  return sorted[idx];
}
const frameDurations = new Ring(600);
let lastFrameTs = 0;
let frameRafRunning = false;
function frameTick(now) {
  if (lastFrameTs !== 0) frameDurations.push(now - lastFrameTs);
  lastFrameTs = now;
  if (frameRafRunning) requestAnimationFrame(frameTick);
}
const longTasks = [];
const LONGTASK_KEEP = 100;
let longTaskTotalMs = 0;
let longTaskCount = 0;
let longTaskCountWindow = 0;
let longTaskMsWindow = 0;
const msgStats = /* @__PURE__ */ new Map();
function diagRecordMessage(type, bytes) {
  let s = msgStats.get(type);
  if (!s) {
    s = { count: 0, bytes: 0 };
    msgStats.set(type, s);
  }
  s.count++;
  s.bytes += bytes;
}
const uidUpdateCounts = /* @__PURE__ */ new Map();
let valueUpdatesThisWindow = 0;
let statusUpdatesThisWindow = 0;
function diagRecordValueUids(uids) {
  for (let i = 0; i < uids.length; i++) {
    const u = uids[i];
    uidUpdateCounts.set(u, (uidUpdateCounts.get(u) ?? 0) + 1);
    valueUpdatesThisWindow++;
  }
}
function diagRecordStatusUids(uids) {
  statusUpdatesThisWindow += uids.length;
}
const frameValueCounts = new Ring(300);
const frameStatusCounts = new Ring(300);
const frameByteSizes = new Ring(300);
let framesThisWindow = 0;
function diagRecordFrame(valueUids, statusUids, bytes) {
  frameValueCounts.push(valueUids);
  frameStatusCounts.push(statusUids);
  frameByteSizes.push(bytes);
  framesThisWindow++;
}
const renderCounts = /* @__PURE__ */ new Map();
let totalRendersThisWindow = 0;
function diagRecordRender(label) {
  renderCounts.set(label, (renderCounts.get(label) ?? 0) + 1);
  totalRendersThisWindow++;
}
const diagGauges = {
  subscribedComponents: 0,
  visibleNodes: 0,
  totalComponents: 0,
  domNodes: 0,
  ghostNodes: 0,
  edges: 0,
  wsConnected: false,
  tickHz: 0,
  reconnects: 0,
  lastSeq: 0
};
function avg(r) {
  const s = r.sorted();
  if (s.length === 0) return 0;
  let sum = 0;
  for (const v of s) sum += v;
  return sum / s.length;
}
let windowStart = 0;
function diagSnapshot(nowMs) {
  const windowMs = windowStart === 0 ? 1e3 : nowMs - windowStart;
  const secs = Math.max(1e-3, windowMs / 1e3);
  const fs = frameDurations.sorted();
  const p50 = pct(fs, 50);
  const fps = p50 > 0 ? 1e3 / p50 : 0;
  const messages = [...msgStats.entries()].map(([type, s]) => ({
    type,
    perSec: s.count / secs,
    bytesPerSec: s.bytes / secs
  })).sort((a, b) => b.bytesPerSec - a.bytesPerSec);
  const topChattyUids = [...uidUpdateCounts.entries()].map(([uid, c]) => ({ uid, updatesPerSec: c / secs })).sort((a, b) => b.updatesPerSec - a.updatesPerSec).slice(0, 20);
  const topRenderers = [...renderCounts.entries()].map(([label, c]) => ({ label, rendersPerSec: c / secs })).sort((a, b) => b.rendersPerSec - a.rendersPerSec).slice(0, 20);
  const snap = {
    ts: nowMs,
    windowMs,
    frame: {
      fps,
      p50,
      p95: pct(fs, 95),
      p99: pct(fs, 99),
      max: fs.length ? fs[fs.length - 1] : 0,
      samples: fs.length
    },
    longTasks: {
      countTotal: longTaskCount,
      totalMs: longTaskTotalMs,
      countWindow: longTaskCountWindow,
      msWindow: longTaskMsWindow,
      recent: longTasks.slice(-8).reverse()
    },
    perSec: {
      frames: framesThisWindow / secs,
      valueUpdates: valueUpdatesThisWindow / secs,
      statusUpdates: statusUpdatesThisWindow / secs,
      renders: totalRendersThisWindow / secs
    },
    frameAnatomy: {
      avgValueUids: avg(frameValueCounts),
      avgStatusUids: avg(frameStatusCounts),
      avgBytes: avg(frameByteSizes),
      maxBytes: frameByteSizes.sorted().pop() ?? 0
    },
    messages,
    topChattyUids,
    topRenderers,
    gauges: { ...diagGauges }
  };
  return snap;
}
function diagResetWindow(nowMs) {
  windowStart = nowMs;
  msgStats.clear();
  uidUpdateCounts.clear();
  renderCounts.clear();
  valueUpdatesThisWindow = 0;
  statusUpdatesThisWindow = 0;
  framesThisWindow = 0;
  totalRendersThisWindow = 0;
  longTaskCountWindow = 0;
  longTaskMsWindow = 0;
  frameValueCounts.clear();
  frameStatusCounts.clear();
  frameByteSizes.clear();
}
let started = false;
let observer = null;
function startDiagnostics() {
  if (started) return;
  started = true;
  windowStart = performance.now();
  frameRafRunning = true;
  lastFrameTs = 0;
  requestAnimationFrame(frameTick);
  if (typeof PerformanceObserver !== "undefined") {
    try {
      observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          longTasks.push({ start: entry.startTime, duration: entry.duration });
          if (longTasks.length > LONGTASK_KEEP) longTasks.shift();
          longTaskCount++;
          longTaskTotalMs += entry.duration;
          longTaskCountWindow++;
          longTaskMsWindow += entry.duration;
        }
      });
      observer.observe({ entryTypes: ["longtask"] });
    } catch {
    }
  }
}
function stopDiagnostics() {
  frameRafRunning = false;
  started = false;
  observer?.disconnect();
  observer = null;
}
const snapshotHistory = [];
const SNAPSHOT_HISTORY = 60;
let reporterTimer = null;
function getSnapshotHistory() {
  return snapshotHistory;
}
function startDiagReporter(intervalMs = 1e3) {
  if (reporterTimer != null) return;
  const tick = () => {
    const now = performance.now();
    const snap = diagSnapshot(now);
    snapshotHistory.push(snap);
    if (snapshotHistory.length > SNAPSHOT_HISTORY) snapshotHistory.shift();
    diagResetWindow(now);
    reporterTimer = window.setTimeout(tick, intervalMs);
  };
  reporterTimer = window.setTimeout(tick, intervalMs);
}
function stopDiagReporter() {
  if (reporterTimer != null) {
    window.clearTimeout(reporterTimer);
    reporterTimer = null;
  }
}

const COLLAPSED_KEY$1 = "ce-ui.diagpanel.collapsed";
function fmtBytes(n) {
  if (n < 1024) return `${n.toFixed(0)} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}
const rowS = { display: "flex", justifyContent: "space-between", padding: "1px 0" };
const lab = { color: "#8892a0" };
const val = { color: "#e6e8eb", fontVariantNumeric: "tabular-nums" };
function DiagPanel({
  wsRef,
  autoRate,
  manualRate,
  onSetManualRate,
  onToggleAutoRate
}) {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem(COLLAPSED_KEY$1) !== "0";
    } catch {
      return true;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem(COLLAPSED_KEY$1, collapsed ? "1" : "0");
    } catch {
    }
  }, [collapsed]);
  const [snap, setSnap] = useState(null);
  const [copied, setCopied] = useState(false);
  const storeApi = useStoreApi();
  const rDot = useRef(null);
  const rConn = useRef(null);
  const rReconn = useRef(null);
  const rSession = useRef(null);
  const rSeq = useRef(null);
  const rTopo = useRef(null);
  const rFps = useRef(null);
  const rFrame = useRef(null);
  const rJank = useRef(null);
  const rZoom = useRef(null);
  const rMsgs = useRef(null);
  const rBytes = useRef(null);
  const rValues = useRef(null);
  const rFrameLast = useRef(null);
  const rParse = useRef(null);
  const rNodes = useRef(null);
  const rEdges = useRef(null);
  const rCanvas = useRef(null);
  useEffect(() => {
    let raf = 0;
    const last = /* @__PURE__ */ new Map();
    const set = (el, v) => {
      if (!el) return;
      if (last.get(el) === v) return;
      last.set(el, v);
      el.textContent = v;
    };
    let lastConn = null;
    const tick = (t) => {
      tickInstrumentation(t);
      const tr = storeApi.getState().transform;
      metrics.zoom = tr[2];
      metrics.panX = tr[0];
      metrics.panY = tr[1];
      metrics.totalComponents = useStructural.getState().components.size;
      if (rDot.current && lastConn !== metrics.wsConnected) {
        rDot.current.style.background = metrics.wsConnected ? "#4ade80" : "#ef4444";
        lastConn = metrics.wsConnected;
      }
      set(rConn.current, metrics.wsConnected ? "connected" : "disconnected");
      set(rReconn.current, String(metrics.reconnectCount));
      set(rSession.current, metrics.sessionId ? metrics.sessionId.slice(0, 8) : "—");
      set(rSeq.current, String(metrics.lastSeq));
      set(rTopo.current, `+${metrics.topoAdded} -${metrics.topoRemoved} ~${metrics.topoChanged}`);
      set(rFps.current, metrics.fps.toFixed(1));
      set(rFrame.current, metrics.frameMs.toFixed(2) + " ms");
      set(rJank.current, `${metrics.longFramesPerSec} (max ${metrics.maxFrameMs.toFixed(1)} ms)`);
      set(rZoom.current, metrics.zoom.toFixed(3));
      set(rMsgs.current, `${metrics.msgsPerSec} /s  (${metrics.framesPerSec} bin)`);
      set(rBytes.current, fmtBytes(metrics.bytesPerSec) + "/s");
      set(rValues.current, `${metrics.valuesPerSec} /s`);
      set(
        rFrameLast.current,
        `${metrics.lastFrameValues} v · ${metrics.lastFrameSections} sec · ${fmtBytes(metrics.lastFrameBytes)}`
      );
      set(rParse.current, metrics.parseAvgMs.toFixed(3) + " ms");
      set(rNodes.current, `${metrics.domNodes} / ${metrics.totalComponents}`);
      set(rEdges.current, String(metrics.domEdges));
      drawSparkline(rCanvas.current);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [storeApi]);
  useEffect(() => {
    if (collapsed) return;
    const id = window.setInterval(() => {
      const hist = getSnapshotHistory();
      setSnap(hist.length ? hist[hist.length - 1] : null);
    }, 500);
    return () => window.clearInterval(id);
  }, [collapsed]);
  const labelForUid = (uid) => {
    const comps = useStructural.getState().components;
    for (const c of comps.values()) {
      for (const [pname, p] of Object.entries(c.properties)) {
        if (p.uid === uid) return `${c.name || c.type}.${pname}`;
      }
    }
    return String(uid);
  };
  if (collapsed) {
    return /* @__PURE__ */ jsxs(
      "button",
      {
        onClick: () => setCollapsed(false),
        title: "Show diagnostics",
        style: {
          position: "fixed",
          top: 12,
          right: 12,
          zIndex: 31,
          display: "flex",
          alignItems: "center",
          gap: 7,
          padding: "6px 11px",
          background: "rgba(20,23,30,0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          color: "#cbd3e0",
          fontSize: 11,
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
          cursor: "pointer"
        },
        children: [
          /* @__PURE__ */ jsx(
            "span",
            {
              ref: rDot,
              style: { width: 8, height: 8, borderRadius: 4, background: "#ef4444", display: "inline-block" }
            }
          ),
          /* @__PURE__ */ jsx("span", { ref: rConn, children: "disconnected" }),
          /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 10 }, children: "▾" })
        ]
      }
    );
  }
  const rate = wsRef?.getRate() ?? null;
  const storms = snap != null && snap.perSec.renders > Math.max(120, snap.perSec.frames * 4) && snap.perSec.renders > (snap.gauges.visibleNodes || 1) * 2;
  const renderStorm = !!storms;
  const slowP95 = snap != null && snap.frame.p95 > 32;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        position: "fixed",
        top: 12,
        right: 12,
        bottom: 12,
        width: 320,
        zIndex: 31,
        background: "rgba(20,23,30,0.96)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        color: "#e6e8eb",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        display: "flex",
        flexDirection: "column"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              display: "flex",
              alignItems: "center",
              gap: 7,
              padding: "8px 10px",
              borderBottom: "1px solid #2c313c"
            },
            children: [
              /* @__PURE__ */ jsx(
                "span",
                {
                  ref: rDot,
                  style: { width: 8, height: 8, borderRadius: 4, background: "#ef4444", display: "inline-block" }
                }
              ),
              /* @__PURE__ */ jsx("span", { ref: rConn, style: { fontWeight: 600 }, children: "disconnected" }),
              /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", fontSize: 10 }, children: [
                "reconn ",
                /* @__PURE__ */ jsx("span", { ref: rReconn, children: "0" })
              ] }),
              /* @__PURE__ */ jsx("span", { style: { flex: 1 } }),
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => {
                    const text = snap ? formatReport(snap, rate, labelForUid) : "(no snapshot yet)";
                    void navigator.clipboard?.writeText(text).then(() => {
                      setCopied(true);
                      window.setTimeout(() => setCopied(false), 1200);
                    }).catch(() => {
                    });
                  },
                  title: "Copy a diagnostics report to clipboard",
                  style: {
                    background: copied ? "#3b6eff" : "#222731",
                    border: "1px solid #2c313c",
                    borderRadius: 3,
                    color: copied ? "#fff" : "#cbd3e0",
                    cursor: "pointer",
                    fontSize: 10,
                    fontFamily: "inherit",
                    padding: "2px 8px",
                    marginRight: 4
                  },
                  children: copied ? "copied ✓" : "copy"
                }
              ),
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setCollapsed(true),
                  title: "Collapse to indicator",
                  style: {
                    background: "transparent",
                    border: "none",
                    color: "#8892a0",
                    cursor: "pointer",
                    fontSize: 14
                  },
                  children: "▴"
                }
              )
            ]
          }
        ),
        /* @__PURE__ */ jsxs("div", { style: { flex: 1, overflowY: "auto", padding: "8px 10px" }, children: [
          /* @__PURE__ */ jsxs(Section, { title: autoRate ? "push rate (auto: follows zoom)" : "push rate (manual)", children: [
            /* @__PURE__ */ jsx("div", { style: { display: "flex", gap: 4, flexWrap: "wrap" }, children: [1, 5, 10, 30, 60].map((hz) => /* @__PURE__ */ jsxs(
              "button",
              {
                onClick: () => !autoRate && onSetManualRate(hz),
                disabled: autoRate,
                title: autoRate ? "turn off auto-scale to set manually" : `${hz} Hz`,
                style: {
                  flex: 1,
                  padding: "3px 0",
                  background: !autoRate && manualRate === hz ? "#3b6eff" : "#222731",
                  color: !autoRate && manualRate === hz ? "#fff" : "#cbd3e0",
                  border: "1px solid #2c313c",
                  borderRadius: 3,
                  cursor: autoRate ? "default" : "pointer",
                  opacity: autoRate ? 0.45 : 1,
                  fontSize: 10,
                  fontFamily: "inherit"
                },
                children: [
                  hz,
                  "Hz"
                ]
              },
              hz
            )) }),
            /* @__PURE__ */ jsxs(
              "label",
              {
                style: {
                  display: "flex",
                  alignItems: "center",
                  gap: 5,
                  marginTop: 5,
                  color: "#cbd3e0",
                  fontSize: 10,
                  cursor: "pointer"
                },
                children: [
                  /* @__PURE__ */ jsx("input", { type: "checkbox", checked: autoRate, onChange: onToggleAutoRate }),
                  "auto-scale rate with zoom"
                ]
              }
            ),
            /* @__PURE__ */ jsxs("div", { style: { color: "#5a6172", fontSize: 9, marginTop: 3 }, children: [
              "current: ",
              rate ?? "engine default",
              " Hz",
              autoRate ? " (zoom-driven)" : " (manual)"
            ] })
          ] }),
          /* @__PURE__ */ jsxs(Section, { title: "live", children: [
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "FPS" }),
              /* @__PURE__ */ jsxs("span", { style: val, children: [
                /* @__PURE__ */ jsx("span", { ref: rFps, children: "0" }),
                "  ",
                "(",
                /* @__PURE__ */ jsx("span", { ref: rFrame, children: "0.00 ms" }),
                ")"
              ] })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "long frames /s" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rJank, children: "0 (max 0.0 ms)" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "zoom" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rZoom, children: "1.000" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "messages" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rMsgs, children: "0 /s" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "bytes" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rBytes, children: "0 B/s" })
            ] }),
            /* @__PURE__ */ jsx(
              "canvas",
              {
                ref: rCanvas,
                width: 296,
                height: 36,
                style: { width: "100%", height: 36, background: "#0f1115", border: "1px solid #1f242e", margin: "2px 0" }
              }
            ),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "values" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rValues, children: "0 /s" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "last frame" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rFrameLast, children: "—" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "parse (avg)" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rParse, children: "0.000 ms" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "nodes (DOM/total)" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rNodes, children: "0 / 0" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "edges (DOM)" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rEdges, children: "0" })
            ] })
          ] }),
          /* @__PURE__ */ jsxs(Section, { title: "topology", children: [
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "session" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rSession, children: "—" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "seq" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rSeq, children: "0" })
            ] }),
            /* @__PURE__ */ jsxs("div", { style: rowS, children: [
              /* @__PURE__ */ jsx("span", { style: lab, children: "events (+/-/~)" }),
              /* @__PURE__ */ jsx("span", { style: val, ref: rTopo, children: "+0 -0 ~0" })
            ] })
          ] }),
          snap == null ? /* @__PURE__ */ jsx("div", { style: { color: "#5a6172", padding: "8px 0" }, children: "collecting…" }) : /* @__PURE__ */ jsxs(Fragment, { children: [
            /* @__PURE__ */ jsxs(Section, { title: "frames (windowed)", children: [
              /* @__PURE__ */ jsx(Row, { k: "fps (from p50)", v: snap.frame.fps.toFixed(1) }),
              /* @__PURE__ */ jsx(Row, { k: "p50", v: `${snap.frame.p50.toFixed(1)} ms` }),
              /* @__PURE__ */ jsx(Row, { k: "p95", v: `${snap.frame.p95.toFixed(1)} ms`, warn: slowP95 }),
              /* @__PURE__ */ jsx(Row, { k: "p99", v: `${snap.frame.p99.toFixed(1)} ms` }),
              /* @__PURE__ */ jsx(Row, { k: "max", v: `${snap.frame.max.toFixed(1)} ms`, warn: snap.frame.max > 100 })
            ] }),
            /* @__PURE__ */ jsxs(Section, { title: "long tasks (≥50ms blocks)", children: [
              /* @__PURE__ */ jsx(
                Row,
                {
                  k: "this window",
                  v: `${snap.longTasks.countWindow} (${snap.longTasks.msWindow.toFixed(0)}ms)`,
                  warn: snap.longTasks.countWindow > 0
                }
              ),
              /* @__PURE__ */ jsx(
                Row,
                {
                  k: "lifetime",
                  v: `${snap.longTasks.countTotal} / ${snap.longTasks.totalMs.toFixed(0)}ms`
                }
              ),
              snap.longTasks.recent.slice(0, 3).map((lt, i) => /* @__PURE__ */ jsx(Row, { k: `  recent ${i + 1}`, v: `${lt.duration.toFixed(0)} ms`, warn: true }, i))
            ] }),
            /* @__PURE__ */ jsxs(Section, { title: "render pressure", children: [
              /* @__PURE__ */ jsx(Row, { k: "frames/sec", v: snap.perSec.frames.toFixed(1) }),
              /* @__PURE__ */ jsx(Row, { k: "renders/sec", v: snap.perSec.renders.toFixed(0), warn: renderStorm }),
              renderStorm && /* @__PURE__ */ jsx("div", { style: { color: "#ef4444", fontSize: 9, marginTop: 2 }, children: "⚠ re-render storm — renders ≫ frames" })
            ] }),
            /* @__PURE__ */ jsxs(Section, { title: "value plane", children: [
              /* @__PURE__ */ jsx(Row, { k: "value updates/sec", v: snap.perSec.valueUpdates.toFixed(0) }),
              /* @__PURE__ */ jsx(Row, { k: "status updates/sec", v: snap.perSec.statusUpdates.toFixed(0) }),
              /* @__PURE__ */ jsx(Row, { k: "avg val uids/frame", v: snap.frameAnatomy.avgValueUids.toFixed(0) }),
              /* @__PURE__ */ jsx(Row, { k: "avg status uids/frame", v: snap.frameAnatomy.avgStatusUids.toFixed(0) }),
              /* @__PURE__ */ jsx(Row, { k: "avg frame bytes", v: fmtBytes(snap.frameAnatomy.avgBytes) }),
              /* @__PURE__ */ jsx(Row, { k: "max frame bytes", v: fmtBytes(snap.frameAnatomy.maxBytes) })
            ] }),
            /* @__PURE__ */ jsx(Section, { title: "bytes/sec by message", children: snap.messages.slice(0, 6).map((m) => /* @__PURE__ */ jsx(Row, { k: m.type, v: `${fmtBytes(m.bytesPerSec)}/s` }, m.type)) }),
            /* @__PURE__ */ jsxs(Section, { title: "chattiest props", children: [
              snap.topChattyUids.slice(0, 8).map((u) => /* @__PURE__ */ jsx(Row, { k: labelForUid(u.uid), v: `${u.updatesPerSec.toFixed(0)}/s` }, u.uid)),
              snap.topChattyUids.length === 0 && /* @__PURE__ */ jsx("div", { style: { color: "#5a6172", fontSize: 9 }, children: "nothing updating" })
            ] }),
            /* @__PURE__ */ jsxs(Section, { title: "structure", children: [
              /* @__PURE__ */ jsx(Row, { k: "visible nodes", v: String(snap.gauges.visibleNodes) }),
              /* @__PURE__ */ jsx(Row, { k: "subscribed (streaming)", v: String(snap.gauges.subscribedComponents) }),
              /* @__PURE__ */ jsx(Row, { k: "ghost nodes", v: String(snap.gauges.ghostNodes) }),
              /* @__PURE__ */ jsx(Row, { k: "edges", v: String(snap.gauges.edges) }),
              /* @__PURE__ */ jsx(Row, { k: "total components", v: String(snap.gauges.totalComponents) }),
              /* @__PURE__ */ jsx(Row, { k: "reconnects", v: String(snap.gauges.reconnects), warn: snap.gauges.reconnects > 0 })
            ] })
          ] })
        ] })
      ]
    }
  );
}
function formatReport(s, rate, labelForUid) {
  const lines = [];
  lines.push("=== ce-ui diagnostics ===");
  lines.push(`pushRate=${rate ?? "engine-default"}`);
  lines.push(
    `frame: fps=${s.frame.fps.toFixed(1)} p50=${s.frame.p50.toFixed(1)}ms p95=${s.frame.p95.toFixed(1)}ms p99=${s.frame.p99.toFixed(1)}ms max=${s.frame.max.toFixed(1)}ms`
  );
  lines.push(
    `longTasks: window=${s.longTasks.countWindow} (${s.longTasks.msWindow.toFixed(0)}ms) lifetime=${s.longTasks.countTotal}/${s.longTasks.totalMs.toFixed(0)}ms`
  );
  lines.push(
    `render: frames/s=${s.perSec.frames.toFixed(1)} renders/s=${s.perSec.renders.toFixed(0)}`
  );
  lines.push(
    `value: upd/s=${s.perSec.valueUpdates.toFixed(0)} status/s=${s.perSec.statusUpdates.toFixed(0)} avgValUids=${s.frameAnatomy.avgValueUids.toFixed(0)} avgStatusUids=${s.frameAnatomy.avgStatusUids.toFixed(0)} avgBytes=${s.frameAnatomy.avgBytes.toFixed(0)} maxBytes=${s.frameAnatomy.maxBytes.toFixed(0)}`
  );
  lines.push("bytes/s by msg:");
  for (const m of s.messages.slice(0, 6)) {
    lines.push(`  ${m.type}: ${m.bytesPerSec.toFixed(0)} B/s (${m.perSec.toFixed(0)}/s)`);
  }
  lines.push("chattiest props:");
  for (const u of s.topChattyUids.slice(0, 10)) {
    lines.push(`  ${labelForUid(u.uid)} (${u.uid}): ${u.updatesPerSec.toFixed(0)}/s`);
  }
  const g = s.gauges;
  lines.push(
    `structure: visible=${g.visibleNodes} ghost=${g.ghostNodes} edges=${g.edges} total=${g.totalComponents} reconnects=${g.reconnects} ws=${g.wsConnected ? "up" : "down"}`
  );
  lines.push("");
  lines.push("--- raw ---");
  lines.push(JSON.stringify(s));
  return lines.join("\n");
}
function Section({ title, children }) {
  return /* @__PURE__ */ jsxs("div", { style: { marginBottom: 10 }, children: [
    /* @__PURE__ */ jsx(
      "div",
      {
        style: {
          color: "#5a6172",
          fontSize: 9,
          textTransform: "uppercase",
          letterSpacing: 0.4,
          marginBottom: 3
        },
        children: title
      }
    ),
    children
  ] });
}
function Row({ k, v, warn }) {
  return /* @__PURE__ */ jsxs("div", { style: { display: "flex", justifyContent: "space-between", padding: "1px 0" }, children: [
    /* @__PURE__ */ jsx("span", { style: { color: "#8892a0", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }, children: k }),
    /* @__PURE__ */ jsx("span", { style: { color: warn ? "#ef4444" : "#e6e8eb", flexShrink: 0, marginLeft: 8 }, children: v })
  ] });
}
function drawSparkline(canvas) {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);
  const data = metrics.bytesSpark;
  let max = 1;
  for (let i = 0; i < data.length; i++) if (data[i] > max) max = data[i];
  ctx.strokeStyle = "#4a9eff";
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let i = 0; i < data.length; i++) {
    const x = i / (data.length - 1) * w;
    const y = h - data[i] / max * (h - 2) - 1;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();
}

const COLLAPSED_KEY = "ce-ui.eventspanel.collapsed";
const KIND_COLOR = {
  "ws-open": "#4ade80",
  "ws-close": "#ef4444",
  schema: "#9ecbff",
  subscribe: "#9ecbff",
  unsubscribe: "#8892a0",
  frame: "#a8b3c7",
  topology: "#ffd166",
  rest: "#f59e0b"
};
const KIND_LABEL = {
  "ws-open": "OPEN",
  "ws-close": "CLOSE",
  schema: "SCHEMA",
  subscribe: "SUB",
  unsubscribe: "UNSUB",
  frame: "FRAME",
  topology: "TOPO",
  rest: "REST"
};
const ALL_KINDS = [
  "ws-open",
  "ws-close",
  "schema",
  "subscribe",
  "unsubscribe",
  "frame",
  "topology",
  "rest"
];
function EventsPanel() {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem(COLLAPSED_KEY) !== "0";
    } catch {
      return true;
    }
  });
  const [paused, setPaused] = useState(false);
  const [excluded, setExcluded] = useState(() => /* @__PURE__ */ new Set(["frame"]));
  useEffect(() => {
    try {
      window.localStorage.setItem(COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
    }
  }, [collapsed]);
  const listRef = useRef(null);
  const lastVersion = useRef(-1);
  const stickToBottom = useRef(true);
  useEffect(() => {
    if (collapsed) return;
    let raf = 0;
    const tick = () => {
      raf = requestAnimationFrame(tick);
      if (paused) return;
      if (eventsVersion.v === lastVersion.current) return;
      lastVersion.current = eventsVersion.v;
      renderList(listRef.current, events, excluded, stickToBottom.current);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [collapsed, paused, excluded]);
  const onScroll = () => {
    const el = listRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 12;
    stickToBottom.current = atBottom;
  };
  if (collapsed) {
    return /* @__PURE__ */ jsx(
      "button",
      {
        onClick: () => setCollapsed(false),
        title: "Show events log",
        style: {
          position: "fixed",
          top: 12,
          right: 304,
          zIndex: 30,
          padding: "6px 10px",
          background: "rgba(20, 23, 30, 0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          color: "#cbd3e0",
          fontSize: 11,
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
          cursor: "pointer"
        },
        children: "events ▾"
      }
    );
  }
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        position: "fixed",
        top: 12,
        right: 304,
        bottom: 12,
        width: 420,
        zIndex: 30,
        background: "rgba(20, 23, 30, 0.96)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        color: "#e6e8eb",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        display: "flex",
        flexDirection: "column"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "8px 10px",
              borderBottom: "1px solid #2c313c"
            },
            children: [
              /* @__PURE__ */ jsx("span", { style: { fontWeight: 600, fontSize: 12, flex: 1 }, children: "events" }),
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setPaused((p) => !p),
                  title: paused ? "Resume" : "Pause",
                  style: btn(paused ? "#3b6eff" : "transparent"),
                  children: paused ? "▶ resume" : "❚❚ pause"
                }
              ),
              /* @__PURE__ */ jsx("button", { onClick: () => clearEvents(), title: "Clear log", style: btn("transparent"), children: "clear" }),
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setCollapsed(true),
                  title: "Hide events log",
                  style: btn("transparent"),
                  children: "▴"
                }
              )
            ]
          }
        ),
        /* @__PURE__ */ jsx(
          "div",
          {
            style: {
              display: "flex",
              flexWrap: "wrap",
              gap: 4,
              padding: "6px 10px",
              borderBottom: "1px solid #2c313c"
            },
            children: ALL_KINDS.map((k) => {
              const on = !excluded.has(k);
              return /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setExcluded((cur) => {
                    const next = new Set(cur);
                    if (next.has(k)) next.delete(k);
                    else next.add(k);
                    return next;
                  }),
                  style: {
                    fontSize: 10,
                    padding: "2px 6px",
                    background: on ? KIND_COLOR[k] : "transparent",
                    color: on ? "#0f1115" : KIND_COLOR[k],
                    border: `1px solid ${KIND_COLOR[k]}`,
                    borderRadius: 2,
                    cursor: "pointer",
                    fontFamily: "inherit",
                    opacity: on ? 1 : 0.55
                  },
                  children: KIND_LABEL[k]
                },
                k
              );
            })
          }
        ),
        /* @__PURE__ */ jsx(
          "div",
          {
            ref: listRef,
            onScroll,
            style: {
              flex: 1,
              overflowY: "auto",
              padding: "4px 8px",
              lineHeight: 1.45,
              whiteSpace: "pre"
            }
          }
        )
      ]
    }
  );
}
function btn(bg) {
  return {
    fontSize: 11,
    padding: "2px 8px",
    background: bg,
    color: "#cbd3e0",
    border: "1px solid #2c313c",
    borderRadius: 3,
    cursor: "pointer",
    fontFamily: "inherit"
  };
}
function renderList(el, buf, excluded, stickToBottom) {
  if (!el) return;
  const t0 = buf.length > 0 ? buf[0].t : 0;
  let html = "";
  for (let i = 0; i < buf.length; i++) {
    const e = buf[i];
    if (excluded.has(e.kind)) continue;
    const rel = ((e.t - t0) / 1e3).toFixed(2).padStart(7);
    const color = KIND_COLOR[e.kind];
    const label = KIND_LABEL[e.kind].padEnd(6);
    html += `<div><span style="color:#5a6172">${rel}s</span> <span style="color:${color}">${label}</span> <span style="color:#cbd3e0">${escapeHtml(e.text)}</span></div>`;
  }
  el.innerHTML = html;
  if (stickToBottom) el.scrollTop = el.scrollHeight;
}
function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

let BASE = "/api/v0";
function setEngineBase(origin) {
  BASE = `${origin.replace(/\/+$/, "")}/api/v0`;
}
class RestError extends Error {
  constructor(status, message, url, method = "", requestBody, responseBody) {
    super(message);
    this.status = status;
    this.url = url;
    this.method = method;
    this.requestBody = requestBody;
    this.responseBody = responseBody;
  }
  // A copy-pasteable dump of the failed round-trip for debugging.
  get debug() {
    const fmt = (v) => v === void 0 ? "" : typeof v === "string" ? v : JSON.stringify(v, null, 2);
    const lines = [`${this.method} ${this.url}`, `→ ${this.status} ${this.message}`];
    if (this.requestBody !== void 0) lines.push("", "Request:", fmt(this.requestBody));
    if (this.responseBody !== void 0) lines.push("", "Response:", fmt(this.responseBody));
    return lines.join("\n");
  }
}
let currentSessionId = null;
function setRestSessionId(id) {
  currentSessionId = id;
}
async function http(method, path, body) {
  const url = `${BASE}${path}`;
  const headers = {};
  if (body !== void 0) headers["Content-Type"] = "application/json";
  if (currentSessionId && method !== "GET") headers["X-CE-Session"] = currentSessionId;
  const res = await fetch(url, {
    method,
    headers,
    body: body !== void 0 ? JSON.stringify(body) : void 0
  });
  let payload = {};
  try {
    payload = await res.json();
  } catch {
  }
  if (method !== "GET") {
    const status = res.ok && !payload.error ? res.status : `ERR ${res.status}`;
    recordEvent("rest", `${method} ${path} → ${status}`);
  }
  if (!res.ok || payload.error) {
    throw new RestError(res.status, payload.error ?? res.statusText, url, method, body, payload);
  }
  return payload.data;
}
function getRootNodes(opts) {
  const q = new URLSearchParams();
  if (opts?.depth != null) q.set("depth", String(opts.depth));
  if (opts?.nested != null) q.set("nested", String(opts.nested));
  if (opts?.withEdges != null) q.set("withEdges", String(opts.withEdges));
  const qs = q.toString();
  return http("GET", `/nodes${qs ? "?" + qs : ""}`);
}
function getNodeByUid(uid, opts) {
  const q = new URLSearchParams();
  if (opts?.depth != null) q.set("depth", String(opts.depth));
  if (opts?.nested != null) q.set("nested", String(opts.nested));
  if (opts?.withEdges != null) q.set("withEdges", String(opts.withEdges));
  const qs = q.toString();
  return http("GET", `/nodes/uid/${uid}${qs ? "?" + qs : ""}`);
}
function addNode(req) {
  return http("POST", `/nodes`, req);
}
function updateNode(uid, req) {
  return http("PATCH", `/nodes/uid/${uid}`, req);
}
function patchOverrides(uid, req) {
  return http("PATCH", `/overrides/nodes/uid/${uid}`, req);
}
function callAction(uid, action, params) {
  return http("POST", `/call/nodes/uid/${uid}`, {
    action,
    params
  });
}
function removeNode(uid) {
  return http("DELETE", `/nodes/uid/${uid}`);
}
function getEdges(component) {
  const qs = component != null ? `?component=${encodeURIComponent(String(component))}` : "";
  return http("GET", `/edges${qs}`);
}
function addEdge(req) {
  return http("POST", `/edge`, req);
}
function bulkUpdate(updates) {
  return http("PATCH", `/bulknodes`, { updates });
}
function bulkDelete(req) {
  return http("DELETE", `/bulknodes`, req);
}
function copyNodes(req) {
  return http("POST", `/copy/nodes`, {
    includeInternalEdges: true,
    ...req
  });
}
function restoreItems(req) {
  return http("POST", `/restore`, req);
}
function bulkPartialError(res) {
  if (!res.result?.partialSuccess) return null;
  const errs = [
    ...res.result.errors?.components ?? res.result.componentErrors ?? [],
    ...res.result.errors?.edges ?? res.result.edgeErrors ?? []
  ];
  if (errs.length === 0) return "bulk op partially failed";
  const first = errs[0];
  return `bulk op: ${errs.length} item(s) failed — ${first.message}`;
}
function removeEdge(uid) {
  return http("DELETE", `/edge/uid/${uid}`);
}
function updateEdge(uid, req) {
  return http("PATCH", `/edge/uid/${uid}`, req);
}
function getExtensions() {
  return http("GET", `/extensions`);
}

const rest = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
  __proto__: null,
  RestError,
  addEdge,
  addNode,
  bulkDelete,
  bulkPartialError,
  bulkUpdate,
  callAction,
  copyNodes,
  getEdges,
  getExtensions,
  getNodeByUid,
  getRootNodes,
  patchOverrides,
  removeEdge,
  removeNode,
  restoreItems,
  setEngineBase,
  setRestSessionId,
  updateEdge,
  updateNode
}, Symbol.toStringTag, { value: 'Module' }));

function FindPanel({
  open,
  currentParentUid,
  onClose,
  onPick
}) {
  const [query, setQuery] = useState("");
  const [all, setAll] = useState(null);
  const [sel, setSel] = useState(0);
  const inputRef = useRef(null);
  const listRef = useRef(null);
  useEffect(() => {
    if (!open) return;
    setQuery("");
    setSel(0);
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        const flat = [];
        const walk = (c) => {
          if (c.uid !== 0) {
            const path = c.path.startsWith("root/") ? c.path.slice(5) : c.path;
            flat.push({
              uid: c.uid,
              name: c.name || c.type,
              type: c.type,
              path,
              parent: c.parent,
              here: c.parent === currentParentUid
            });
          }
          c.children?.forEach(walk);
        };
        resp.nodes.forEach(walk);
        setAll(flat);
      } catch {
        if (!cancelled) setAll([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, currentParentUid]);
  useEffect(() => {
    if (open) {
      const t = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(t);
    }
  }, [open]);
  const results = useMemo(() => {
    if (!all) return [];
    const q = query.trim().toLowerCase();
    if (!q) return all.slice(0, 50);
    const scored = all.map((h) => {
      const name = h.name.toLowerCase();
      let score = -1;
      if (name === q) score = 0;
      else if (name.startsWith(q)) score = 1;
      else if (name.includes(q)) score = 2;
      else if (h.path.toLowerCase().includes(q) || h.type.toLowerCase().includes(q)) score = 3;
      return { h, score };
    }).filter((x) => x.score >= 0).sort(
      (a, b) => Number(b.h.here) - Number(a.h.here) || a.score - b.score || a.h.name.localeCompare(b.h.name)
    ).slice(0, 50).map((x) => x.h);
    return scored;
  }, [all, query]);
  useEffect(() => {
    if (sel >= results.length) setSel(0);
  }, [results, sel]);
  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${sel}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [sel]);
  if (!open) return null;
  const pick = (h) => {
    if (!h) return;
    onPick(h.uid);
    onClose();
  };
  return /* @__PURE__ */ jsx(
    "div",
    {
      onMouseDown: onClose,
      style: {
        position: "fixed",
        inset: 0,
        zIndex: 200,
        background: "rgba(0,0,0,0.35)",
        display: "flex",
        justifyContent: "center",
        alignItems: "flex-start",
        paddingTop: "12vh"
      },
      children: /* @__PURE__ */ jsxs(
        "div",
        {
          onMouseDown: (e) => e.stopPropagation(),
          style: {
            width: 480,
            maxWidth: "90vw",
            background: "#1a1d24",
            border: "1px solid #2c313c",
            borderRadius: 8,
            boxShadow: "0 12px 40px rgba(0,0,0,0.6)",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
            fontFamily: "-apple-system, system-ui, sans-serif"
          },
          children: [
            /* @__PURE__ */ jsx(
              "input",
              {
                ref: inputRef,
                value: query,
                onChange: (e) => {
                  setQuery(e.target.value);
                  setSel(0);
                },
                onKeyDown: (e) => {
                  if (e.key === "Escape") {
                    e.preventDefault();
                    onClose();
                  } else if (e.key === "ArrowDown") {
                    e.preventDefault();
                    setSel((s) => Math.min(results.length - 1, s + 1));
                  } else if (e.key === "ArrowUp") {
                    e.preventDefault();
                    setSel((s) => Math.max(0, s - 1));
                  } else if (e.key === "Enter") {
                    e.preventDefault();
                    pick(results[sel]);
                  }
                  e.stopPropagation();
                },
                placeholder: "Find component by name, type, or path…",
                spellCheck: false,
                style: {
                  background: "#0f1115",
                  color: "#e6e8eb",
                  border: "none",
                  borderBottom: "1px solid #2c313c",
                  padding: "12px 14px",
                  fontSize: 14,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  outline: "none"
                }
              }
            ),
            /* @__PURE__ */ jsx("div", { ref: listRef, style: { maxHeight: "50vh", overflowY: "auto" }, children: all == null ? /* @__PURE__ */ jsx("div", { style: { padding: "12px 14px", color: "#5a6172", fontSize: 12 }, children: "loading…" }) : results.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "12px 14px", color: "#5a6172", fontSize: 12 }, children: "no matches" }) : results.map((h, i) => {
              const prev = i > 0 ? results[i - 1] : null;
              const showHereHeader = h.here && (prev === null || !prev.here);
              const showElsewhereHeader = !h.here && (prev === null || prev.here);
              return /* @__PURE__ */ jsxs("div", { children: [
                showHereHeader && /* @__PURE__ */ jsx(FindHeader, { label: "this folder" }),
                showElsewhereHeader && /* @__PURE__ */ jsx(FindHeader, { label: "elsewhere" }),
                /* @__PURE__ */ jsxs(
                  "button",
                  {
                    "data-idx": i,
                    onMouseEnter: () => setSel(i),
                    onClick: () => pick(h),
                    style: {
                      display: "flex",
                      width: "100%",
                      textAlign: "left",
                      alignItems: "baseline",
                      gap: 8,
                      padding: "8px 14px 8px 12px",
                      background: i === sel ? "#2c3a55" : "transparent",
                      border: "none",
                      // Left accent on same-folder rows so they read as "here"
                      // even mid-scroll, past the section header.
                      borderLeft: `2px solid ${h.here ? "#4a9eff" : "transparent"}`,
                      cursor: "pointer",
                      fontFamily: "ui-monospace, SFMono-Regular, monospace"
                    },
                    children: [
                      /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb", fontSize: 13, flexShrink: 0 }, children: h.name }),
                      /* @__PURE__ */ jsx(
                        "span",
                        {
                          style: {
                            color: "#5a6172",
                            fontSize: 11,
                            flex: 1,
                            minWidth: 0,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap"
                          },
                          title: `${h.path} · ${h.type}`,
                          children: h.here ? h.type : h.path
                        }
                      ),
                      !h.here && /* @__PURE__ */ jsx("span", { style: { color: "#8892a0", fontSize: 10, flexShrink: 0 }, children: h.type })
                    ]
                  }
                )
              ] }, h.uid);
            }) }),
            /* @__PURE__ */ jsx(
              "div",
              {
                style: {
                  padding: "6px 14px",
                  borderTop: "1px solid #2c313c",
                  color: "#5a6172",
                  fontSize: 10,
                  fontFamily: "ui-monospace, monospace"
                },
                children: "↑↓ navigate · ↵ go · esc close"
              }
            )
          ]
        }
      )
    }
  );
}
function FindHeader({ label }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        padding: "6px 14px 3px 14px",
        color: "#5a6172",
        fontSize: 9,
        textTransform: "uppercase",
        letterSpacing: 0.5,
        fontFamily: "ui-monospace, SFMono-Regular, monospace"
      },
      children: label
    }
  );
}

const PRESENCE_PALETTE = [
  "#f97316",
  // orange
  "#a855f7",
  // purple
  "#06b6d4",
  // cyan
  "#ec4899",
  // pink
  "#84cc16",
  // lime
  "#eab308",
  // yellow
  "#ef4444",
  // red
  "#14b8a6"
  // teal
];
let nextColor = 0;
const nowMs = () => performance.now();
const usePresence = create((set, get) => ({
  collaborators: /* @__PURE__ */ new Map(),
  version: 0,
  upsert: (sessionId, state) => set((s) => {
    const next = new Map(s.collaborators);
    const existing = next.get(sessionId);
    next.set(sessionId, {
      sessionId,
      state,
      colorIdx: existing ? existing.colorIdx : nextColor++ % PRESENCE_PALETTE.length,
      lastSeen: nowMs()
    });
    return { collaborators: next, version: s.version + 1 };
  }),
  sweep: (ttlMs) => set((s) => {
    const cutoff = nowMs() - ttlMs;
    let changed = false;
    const next = new Map(s.collaborators);
    for (const [id, c] of next) {
      if (c.lastSeen < cutoff) {
        next.delete(id);
        changed = true;
      }
    }
    return changed ? { collaborators: next, version: s.version + 1 } : s;
  }),
  remove: (sessionId) => set((s) => {
    if (!s.collaborators.has(sessionId)) return s;
    const next = new Map(s.collaborators);
    next.delete(sessionId);
    return { collaborators: next, version: s.version + 1 };
  }),
  replaceAll: (entries) => set((s) => {
    const next = /* @__PURE__ */ new Map();
    for (const e of entries) {
      const existing = get().collaborators.get(e.sessionId);
      next.set(e.sessionId, {
        sessionId: e.sessionId,
        state: e.state,
        colorIdx: existing ? existing.colorIdx : nextColor++ % PRESENCE_PALETTE.length,
        lastSeen: nowMs()
      });
    }
    return { collaborators: next, version: s.version + 1 };
  }),
  reset: () => set({ collaborators: /* @__PURE__ */ new Map(), version: 0 })
}));

function PresenceBar() {
  const collaborators = usePresence((s) => s.collaborators);
  if (collaborators.size === 0) return null;
  const list = [...collaborators.values()];
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        // Sit just above the breadcrumb (also bottom-centered at bottom:12) so
        // the two don't overlap.
        position: "fixed",
        bottom: 52,
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 25,
        display: "flex",
        alignItems: "center",
        gap: 8,
        background: "rgba(20,23,30,0.92)",
        border: "1px solid #2c313c",
        borderRadius: 20,
        padding: "5px 12px",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        color: "#cbd3e0"
      },
      children: [
        /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", fontSize: 10 }, children: [
          list.length,
          " other",
          list.length === 1 ? "" : "s"
        ] }),
        list.map((c) => {
          const color = PRESENCE_PALETTE[c.colorIdx];
          const name = c.state.userName ?? c.sessionId.slice(0, 6);
          const selCount = c.state.selectedComponents?.length ?? 0;
          return /* @__PURE__ */ jsxs(
            "span",
            {
              title: selCount > 0 ? `${name} — ${selCount} selected` : `${name} — viewing`,
              style: { display: "flex", alignItems: "center", gap: 4 },
              children: [
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      width: 8,
                      height: 8,
                      borderRadius: 4,
                      background: color,
                      display: "inline-block"
                    }
                  }
                ),
                /* @__PURE__ */ jsx("span", { children: name }),
                selCount > 0 && /* @__PURE__ */ jsxs("span", { style: { color: "#5a6172", fontSize: 9 }, children: [
                  "×",
                  selCount
                ] })
              ]
            },
            c.sessionId
          );
        })
      ]
    }
  );
}

const BUCKETS = [
  { minZoom: 0, hz: 1 },
  // far out — can't read anything
  { minZoom: 0.3, hz: 4 },
  // shapes legible, values not really
  { minZoom: 0.55, hz: 10 },
  // readable → full (ceiling) rate
  { minZoom: 1.3, hz: 15 }
  // deep zoom on a few nodes
];
function rateForZoom(zoom) {
  let hz = BUCKETS[0].hz;
  for (const b of BUCKETS) {
    if (zoom >= b.minZoom) hz = b.hz;
  }
  return hz;
}
const POLL_MS = 1e3;
const LOW_FPS = 30;
const GOOD_FPS = 50;
const MIN_HZ = 1;
const BACKOFF = 0.5;
const RECOVER = 0.25;
function ZoomRateController({
  enabled,
  setRate
}) {
  const store = useStoreApi();
  const scale = useRef(1);
  const lastSent = useRef(null);
  useEffect(() => {
    if (!enabled) {
      lastSent.current = null;
      scale.current = 1;
      return;
    }
    const evaluate = () => {
      const zoom = store.getState().transform[2];
      const ceiling = rateForZoom(zoom);
      const fps = metrics.fps;
      if (fps > 0) {
        if (fps < LOW_FPS) scale.current = Math.max(0.02, scale.current * BACKOFF);
        else if (fps > GOOD_FPS) scale.current = Math.min(1, scale.current + RECOVER);
      }
      const want = Math.max(MIN_HZ, Math.min(ceiling, Math.round(ceiling * scale.current)));
      if (want !== lastSent.current) {
        lastSent.current = want;
        setRate(want);
      }
    };
    evaluate();
    const id = window.setInterval(evaluate, POLL_MS);
    return () => window.clearInterval(id);
  }, [enabled, setRate, store]);
  return null;
}

function takenInputUids(edges) {
  const taken = /* @__PURE__ */ new Set();
  for (const e of edges) if (e.targetPropertyUid != null) taken.add(e.targetPropertyUid);
  return taken;
}
const connectTier = (g) => g.isParent ? 0 : g.sibling ? 1 : g.isChild ? 2 : 3;
function buildConnectGroups(components, opts) {
  const { sourceComponentUid, sourceParent, wantCategory, taken } = opts;
  const groups = [];
  for (const c of components) {
    if (c.uid === sourceComponentUid) continue;
    const props = [];
    for (const [name, p] of Object.entries(c.properties)) {
      if (p.category !== wantCategory) continue;
      if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
      if (taken.has(p.uid)) continue;
      props.push({ propUid: p.uid, propName: name });
    }
    if (props.length === 0) continue;
    props.sort((a, b) => a.propName.localeCompare(b.propName));
    groups.push({
      componentUid: c.uid,
      componentName: c.name || c.type,
      path: c.path,
      sibling: sourceParent !== void 0 && c.parent === sourceParent,
      isParent: sourceParent !== void 0 && c.uid === sourceParent,
      isChild: c.parent === sourceComponentUid,
      props
    });
  }
  groups.sort((a, b) => {
    const ta = connectTier(a);
    const tb = connectTier(b);
    if (ta !== tb) return ta - tb;
    return ta === 3 ? a.path.localeCompare(b.path) : a.componentName.localeCompare(b.componentName);
  });
  return groups;
}
function filterConnectGroups(groups, filter) {
  const f = filter.trim().toLowerCase();
  if (!f) return groups;
  const slash = f.lastIndexOf("/");
  const pathScope = slash >= 0 ? f.slice(0, slash) : "";
  const term = slash >= 0 ? f.slice(slash + 1) : f;
  const out = [];
  for (const g of groups) {
    const path = g.path.toLowerCase();
    if (pathScope && !path.includes(pathScope)) continue;
    if (!term) {
      out.push(g);
      continue;
    }
    const tail = pathScope ? path.slice(path.indexOf(pathScope) + pathScope.length) : path;
    if (g.componentName.toLowerCase().includes(term) || tail.includes(term)) {
      out.push(g);
      continue;
    }
    const props = g.props.filter((p) => p.propName.toLowerCase().includes(term));
    if (props.length > 0) out.push({ ...g, props });
  }
  return out;
}

const CeWiresheetContext = createContext(null);
const COLOR_NUMBER = "#4a9eff";
const COLOR_BOOL = "#4ade80";
const COLOR_STRING = "#f59e0b";
const TITLE_H = 40;
const ROW_H = 18;
const NODE_W = 220;
const GHOST_H = ROW_H;
const GHOST_W_MIN = 90;
const GHOST_W_MAX = 260;
function ghostWidthFor(path, propName) {
  const text = `${path || "root"} · ${propName}`;
  const w = 22 + Math.ceil(text.length * 6.2);
  return Math.max(GHOST_W_MIN, Math.min(GHOST_W_MAX, w));
}
function stripRoot(path) {
  if (path === "root" || path === "") return "root";
  if (path.startsWith("root/")) return path.slice(5);
  return path;
}
const DATATYPE_LABEL = {
  [DATATYPE_NUMBER]: "number",
  [DATATYPE_BOOL]: "bool",
  [DATATYPE_STRING]: "string"
};
const LOD_ZOOM = 0.12;
const EMPTY_VALUES = Object.freeze({});
const EMPTY_FLAGS = Object.freeze({});
function colorForType(dt) {
  if (dt === DATATYPE_BOOL) return COLOR_BOOL;
  if (dt === DATATYPE_STRING) return COLOR_STRING;
  return COLOR_NUMBER;
}
function inferDataType(v) {
  if (typeof v === "boolean") return DATATYPE_BOOL;
  if (typeof v === "string") return DATATYPE_STRING;
  return DATATYPE_NUMBER;
}
function fmtValue(v, dt) {
  if (v === void 0) return "—";
  if (typeof v === "bigint") return v.toString();
  if (typeof v === "boolean") return v ? "true" : "false";
  if (typeof v === "string") return JSON.stringify(v).slice(1, -1);
  if (dt === DATATYPE_BOOL) return v ? "true" : "false";
  if (Number.isInteger(v)) return v.toString();
  return v.toFixed(2);
}
function fmtValueFacet(v, dt, facet) {
  const al = aliasLabel(facet?.aliases, v);
  if (al != null) return al;
  let base;
  if (facet?.decimals != null && typeof v === "number") base = v.toFixed(facet.decimals);
  else base = fmtValue(v, dt);
  return facet?.unit && base !== "—" ? `${base} ${facet.unit}` : base;
}
function CopyUid({ label, value }) {
  const [copied, setCopied] = useState(false);
  return /* @__PURE__ */ jsxs(
    "span",
    {
      onClick: (e) => {
        e.stopPropagation();
        void navigator.clipboard?.writeText(String(value)).then(
          () => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 900);
          },
          () => {
          }
        );
      },
      title: "click to copy",
      style: {
        cursor: "pointer",
        textDecoration: "underline dotted",
        color: copied ? "#7ee787" : "inherit"
      },
      children: [
        label,
        " ",
        copied ? "copied" : value
      ]
    }
  );
}
function PropertyContextMenu({
  x,
  y,
  propName,
  propUid,
  category,
  dataType,
  currentValue,
  overridden,
  exposed,
  portOwner,
  componentUid,
  onClose
}) {
  const [promptOpen, setPromptOpen] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [draft, setDraft] = useState(
    currentValue == null ? "" : typeof currentValue === "string" ? currentValue : String(currentValue)
  );
  const [durationSec, setDurationSec] = useState(60);
  const ctx = useContext(CeWiresheetContext);
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-menu]")) return;
      onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  const canConnect = category === CATEGORY_INPUT || category === CATEGORY_OUTPUT;
  const overridable = !exposed && (category === CATEGORY_INPUT || category === CATEGORY_CONFIG || category === CATEGORY_OUTPUT);
  const parse = (raw) => {
    const t = raw.trim();
    if (t === "") return null;
    if (dataType === DATATYPE_BOOL) {
      const lower = t.toLowerCase();
      return lower === "true" || lower === "1" || lower === "yes";
    }
    if (dataType === DATATYPE_STRING) return t;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  };
  const optimisticSetBit = async (uid, bit, on) => {
    const { useStatusFlags: useStatusFlags2 } = await Promise.resolve().then(() => store);
    const s = useStatusFlags2.getState();
    const cur = s.flags.get(uid) ?? 0;
    const next = on ? cur | bit : cur & ~bit;
    s.applyStatus([uid], [next]);
  };
  const setOverride = async () => {
    const parsed = parse(draft);
    if (parsed == null) {
      onClose();
      return;
    }
    onClose();
    const { useStructural: useStructural2 } = await Promise.resolve().then(() => store);
    const cur = useStructural2.getState().components.get(componentUid);
    const uid = cur?.properties[propName]?.uid;
    if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, true);
    try {
      const { patchOverrides } = await Promise.resolve().then(() => rest);
      const updated = await patchOverrides(componentUid, {
        setOverrides: [
          { property: propName, value: parsed, duration: durationSec }
        ]
      });
      useStructural2.getState().upsertComponent(updated);
    } catch (e) {
      console.error("set override failed:", e.message);
      if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, false);
    }
  };
  const clearOverride = async () => {
    onClose();
    const { useStructural: useStructural2 } = await Promise.resolve().then(() => store);
    const cur = useStructural2.getState().components.get(componentUid);
    const uid = cur?.properties[propName]?.uid;
    if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, false);
    try {
      const { patchOverrides } = await Promise.resolve().then(() => rest);
      const updated = await patchOverrides(componentUid, { clearOverrides: [propName] });
      useStructural2.getState().upsertComponent(updated);
    } catch (e) {
      console.error("clear override failed:", e.message);
      if (uid != null) await optimisticSetBit(uid, STATUS_OVERRIDDEN, true);
    }
  };
  return createPortal(
    /* @__PURE__ */ jsxs(
      "div",
      {
        "data-ce-menu": true,
        onContextMenu: (e) => e.preventDefault(),
        style: {
          position: "fixed",
          left: x,
          top: y,
          zIndex: 100,
          background: "#1a1d24",
          border: "1px solid #2c313c",
          borderRadius: 4,
          padding: 4,
          minWidth: 180,
          boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
          fontSize: 11,
          color: "#e6e8eb",
          fontFamily: "-apple-system, system-ui, sans-serif"
        },
        children: [
          /* @__PURE__ */ jsxs(
            "div",
            {
              style: { padding: "4px 8px", color: "#8892a0", borderBottom: "1px solid #2c313c", marginBottom: 4 },
              children: [
                propName,
                " ",
                /* @__PURE__ */ jsxs("span", { style: { color: "#5a6172" }, children: [
                  "· ",
                  dataType
                ] }),
                /* @__PURE__ */ jsxs(
                  "div",
                  {
                    style: {
                      fontSize: 9,
                      color: "#5a6172",
                      fontFamily: "ui-monospace, SFMono-Regular, monospace",
                      marginTop: 2
                    },
                    children: [
                      /* @__PURE__ */ jsx(CopyUid, { label: "prop", value: propUid }),
                      " · ",
                      /* @__PURE__ */ jsx(CopyUid, { label: "comp", value: componentUid })
                    ]
                  }
                )
              ]
            }
          ),
          promptOpen ? /* @__PURE__ */ jsxs("div", { style: { padding: "4px 6px", display: "flex", flexDirection: "column", gap: 4 }, children: [
            dataType === DATATYPE_BOOL ? /* @__PURE__ */ jsxs(
              "select",
              {
                autoFocus: true,
                className: "nodrag",
                value: draft || "true",
                onChange: (e) => setDraft(e.target.value),
                onKeyDown: (e) => {
                  if (e.key === "Enter") setOverride();
                  else if (e.key === "Escape") onClose();
                  e.stopPropagation();
                },
                style: overrideInputStyle,
                children: [
                  /* @__PURE__ */ jsx("option", { value: "true", children: "true" }),
                  /* @__PURE__ */ jsx("option", { value: "false", children: "false" })
                ]
              }
            ) : /* @__PURE__ */ jsx(
              "input",
              {
                autoFocus: true,
                className: "nodrag",
                type: dataType === DATATYPE_NUMBER ? "number" : "text",
                inputMode: dataType === DATATYPE_NUMBER ? "decimal" : void 0,
                step: dataType === DATATYPE_NUMBER ? "any" : void 0,
                value: draft,
                onChange: (e) => setDraft(e.target.value),
                onKeyDown: (e) => {
                  if (e.key === "Enter") setOverride();
                  else if (e.key === "Escape") onClose();
                  e.stopPropagation();
                },
                style: overrideInputStyle,
                placeholder: "override value…"
              }
            ),
            /* @__PURE__ */ jsxs("label", { style: { display: "flex", alignItems: "center", gap: 4, color: "#8892a0", fontSize: 10 }, children: [
              /* @__PURE__ */ jsx("span", { style: { flex: 1 }, children: "duration" }),
              /* @__PURE__ */ jsxs(
                "select",
                {
                  className: "nodrag",
                  value: durationSec,
                  onChange: (e) => setDurationSec(Number(e.target.value)),
                  onClick: (e) => e.stopPropagation(),
                  style: {
                    background: "#0f1115",
                    color: "#e6e8eb",
                    border: "1px solid #2c313c",
                    borderRadius: 2,
                    padding: "2px 4px",
                    fontSize: 11,
                    fontFamily: "inherit"
                  },
                  children: [
                    /* @__PURE__ */ jsx("option", { value: 10, children: "10 sec" }),
                    /* @__PURE__ */ jsx("option", { value: 30, children: "30 sec" }),
                    /* @__PURE__ */ jsx("option", { value: 60, children: "1 min" }),
                    /* @__PURE__ */ jsx("option", { value: 300, children: "5 min" }),
                    /* @__PURE__ */ jsx("option", { value: 1200, children: "20 min" }),
                    /* @__PURE__ */ jsx("option", { value: 3600, children: "1 hr" }),
                    /* @__PURE__ */ jsx("option", { value: 7200, children: "2 hr" }),
                    /* @__PURE__ */ jsx("option", { value: 86400, children: "24 hr" }),
                    /* @__PURE__ */ jsx("option", { value: 0, children: "permanent" })
                  ]
                }
              )
            ] }),
            /* @__PURE__ */ jsx(
              "button",
              {
                onClick: setOverride,
                style: {
                  padding: "3px 6px",
                  background: "#3b6eff",
                  color: "#fff",
                  border: "1px solid #5a83ff",
                  borderRadius: 2,
                  cursor: "pointer",
                  fontSize: 11,
                  fontFamily: "inherit"
                },
                children: "Set override"
              }
            )
          ] }) : /* @__PURE__ */ jsxs(Fragment, { children: [
            overridable && /* @__PURE__ */ jsx(
              MenuItem,
              {
                onClick: () => setPromptOpen(true),
                label: overridden ? "Change override…" : "Set override…"
              }
            ),
            overridable && overridden && /* @__PURE__ */ jsx(MenuItem, { onClick: clearOverride, label: "Clear override", danger: true }),
            canConnect && /* @__PURE__ */ jsx(MenuItem, { onClick: () => setPickerOpen(true), label: "Connect to…" }),
            canConnect && ctx?.exposeProp && ctx.parentName && /* @__PURE__ */ jsx(
              MenuItem,
              {
                onClick: () => {
                  ctx.exposeProp?.(
                    propUid,
                    componentUid,
                    category === CATEGORY_OUTPUT ? "output" : "input",
                    propName
                  );
                  onClose();
                },
                label: `Expose on ${ctx.parentName}`
              }
            ),
            exposed && ctx?.openDetails && /* @__PURE__ */ jsx(
              MenuItem,
              {
                onClick: () => {
                  ctx.openDetails?.(componentUid);
                  onClose();
                },
                label: "Configure…"
              }
            ),
            exposed && ctx?.unexposeProp && portOwner != null && /* @__PURE__ */ jsx(
              MenuItem,
              {
                onClick: () => {
                  ctx.unexposeProp?.(portOwner, propUid);
                  onClose();
                },
                label: "Un-expose",
                danger: true
              }
            )
          ] }),
          pickerOpen && /* @__PURE__ */ jsx(
            ConnectPicker,
            {
              x,
              y,
              sourceComponentUid: componentUid,
              sourcePropUid: propUid,
              sourceCategory: category === CATEGORY_OUTPUT ? "output" : "input",
              onClose: () => {
                setPickerOpen(false);
                onClose();
              }
            }
          )
        ]
      }
    ),
    document.body
  );
}
function ConnectPicker({
  x,
  y,
  sourceComponentUid,
  sourcePropUid,
  sourceCategory,
  onClose
}) {
  const [filter, setFilter] = useState("");
  const [expanded, setExpanded] = useState(null);
  const ctx = useContext(CeWiresheetContext);
  const [creatingNew, setCreatingNew] = useState(false);
  const [pendingNew, setPendingNew] = useState(null);
  const [highlight, setHighlight] = useState(0);
  const hlRef = useRef(null);
  useEffect(() => {
    setHighlight(0);
  }, [filter, creatingNew, pendingNew]);
  useEffect(() => {
    hlRef.current?.scrollIntoView({ block: "nearest" });
  }, [highlight, creatingNew]);
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-menu]")) return;
      onClose();
    };
    const onEsc = (e) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
      document.removeEventListener("keydown", onEsc);
    };
  }, [onClose]);
  const wantCategory = sourceCategory === "output" ? CATEGORY_INPUT : CATEGORY_OUTPUT;
  const [allComponents, setAllComponents] = useState(null);
  const [allEdges, setAllEdges] = useState(null);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const { getRootNodes } = await Promise.resolve().then(() => rest);
      try {
        const resp = await getRootNodes({ depth: -1, nested: true, withEdges: true });
        if (cancelled) return;
        const flat = [];
        const walk = (c) => {
          flat.push(c);
          c.children?.forEach(walk);
        };
        const root = resp.nodes[0];
        root?.children?.forEach(walk);
        setAllComponents(flat);
        setAllEdges(resp.edges ?? []);
      } catch {
        if (cancelled) return;
        setAllComponents([...useStructural.getState().components.values()]);
        setAllEdges([...useStructural.getState().edges.values()]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
  const taken = sourceCategory === "output" && allEdges ? takenInputUids(allEdges) : /* @__PURE__ */ new Set();
  const sourceComp = useStructural.getState().components.get(sourceComponentUid);
  const sourceParent = sourceComp?.parent;
  const sourceName = sourceComp?.name || "component";
  const componentList = allComponents ?? [];
  const groups = buildConnectGroups(componentList, {
    sourceComponentUid,
    sourceParent,
    wantCategory,
    taken
  });
  const filteredGroups = filterConnectGroups(groups, filter);
  const f = filter.trim();
  const create = async (target) => {
    const payload = sourceCategory === "output" ? {
      sourceUid: sourceComponentUid,
      sourcePropUid,
      targetUid: target.componentUid,
      targetPropUid: target.propUid
    } : {
      sourceUid: target.componentUid,
      sourcePropUid: target.propUid,
      targetUid: sourceComponentUid,
      targetPropUid: sourcePropUid
    };
    try {
      if (ctx?.connectEdge) {
        await ctx.connectEdge(payload);
      } else {
        const { addEdge } = await Promise.resolve().then(() => rest);
        await addEdge(payload);
      }
    } catch (e) {
      console.error("add edge failed:", e.message);
    }
    onClose();
  };
  const allFilteredProps = filteredGroups.flatMap(
    (g) => g.props.map((p) => ({ componentUid: g.componentUid, propUid: p.propUid }))
  );
  const groupPropOffsets = [];
  {
    let acc = 0;
    for (const g of filteredGroups) {
      groupPropOffsets.push(acc);
      acc += g.props.length;
    }
  }
  const createNew = async (type) => {
    if (!ctx) return;
    const side = sourceCategory === "output" ? "right" : "left";
    const c = await ctx.createComponent(type, { nearUid: sourceComponentUid, side });
    if (!c) {
      onClose();
      return;
    }
    const matching = Object.entries(c.properties ?? {}).filter(
      ([, p]) => p.category === wantCategory && (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL
    ).map(([name, p]) => ({ uid: p.uid, name }));
    if (matching.length === 0) {
      onClose();
    } else if (matching.length === 1) {
      await create({ componentUid: c.uid, propUid: matching[0].uid });
    } else {
      setPendingNew(c);
      setFilter("");
    }
  };
  const nf = filter.trim().toLowerCase();
  const newTypes = (ctx?.componentTypes ?? []).filter(
    (t) => !nf || t.name.toLowerCase().includes(nf) || t.type.toLowerCase().includes(nf)
  );
  const newProps = pendingNew ? Object.entries(pendingNew.properties ?? {}).filter(
    ([, p]) => p.category === wantCategory && (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL
  ).map(([name, p]) => ({ uid: p.uid, name })) : [];
  const newPropsFiltered = nf ? newProps.filter((p) => p.name.toLowerCase().includes(nf)) : newProps;
  const PICKER_W = 240;
  const left = Math.min(x + 184, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 320);
  return createPortal(
    /* @__PURE__ */ jsxs(
      "div",
      {
        "data-ce-menu": true,
        onContextMenu: (e) => e.preventDefault(),
        style: {
          position: "fixed",
          left,
          top,
          zIndex: 101,
          background: "#1a1d24",
          border: "1px solid #2c313c",
          borderRadius: 4,
          width: PICKER_W,
          maxHeight: 320,
          boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
          fontSize: 11,
          color: "#e6e8eb",
          fontFamily: "-apple-system, system-ui, sans-serif",
          display: "flex",
          flexDirection: "column"
        },
        children: [
          /* @__PURE__ */ jsxs("div", { style: { padding: "6px 8px", borderBottom: "1px solid #2c313c" }, children: [
            /* @__PURE__ */ jsx("div", { style: { display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }, children: pendingNew ? /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setPendingNew(null),
                  title: "Back to component types",
                  style: {
                    background: "transparent",
                    border: "none",
                    color: "#9ecbff",
                    cursor: "pointer",
                    fontSize: 13,
                    padding: 0
                  },
                  children: "‹"
                }
              ),
              /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", fontSize: 10, flex: 1 }, children: [
                pendingNew.name,
                " → pick ",
                wantCategory === CATEGORY_INPUT ? "input" : "output"
              ] })
            ] }) : creatingNew ? /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setCreatingNew(false),
                  title: "Back to existing components",
                  style: {
                    background: "transparent",
                    border: "none",
                    color: "#9ecbff",
                    cursor: "pointer",
                    fontSize: 13,
                    padding: 0
                  },
                  children: "‹"
                }
              ),
              /* @__PURE__ */ jsx("span", { style: { color: "#8892a0", fontSize: 10, flex: 1 }, children: "New component" })
            ] }) : /* @__PURE__ */ jsxs(Fragment, { children: [
              /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", fontSize: 10, flex: 1 }, children: [
                "Existing component → ",
                wantCategory,
                "…"
              ] }),
              ctx && /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setCreatingNew(true),
                  title: "Create a new component and connect to it",
                  style: {
                    fontSize: 10,
                    padding: "1px 6px",
                    background: "#2c3a55",
                    color: "#9ecbff",
                    border: "1px solid #3b5388",
                    borderRadius: 3,
                    cursor: "pointer",
                    fontFamily: "inherit"
                  },
                  children: "+ New"
                }
              )
            ] }) }),
            /* @__PURE__ */ jsx(
              "input",
              {
                autoFocus: true,
                value: filter,
                onChange: (e) => setFilter(e.target.value),
                onKeyDown: (e) => {
                  if (e.key === "Escape") {
                    onClose();
                    return;
                  }
                  if (e.key === "Tab") {
                    e.preventDefault();
                    if (pendingNew) setPendingNew(null);
                    else if (ctx) setCreatingNew((v) => !v);
                    return;
                  }
                  const len = pendingNew ? newPropsFiltered.length : creatingNew ? newTypes.length : allFilteredProps.length;
                  if (e.key === "ArrowDown") {
                    e.preventDefault();
                    setHighlight((h) => Math.min(h + 1, Math.max(0, len - 1)));
                    return;
                  }
                  if (e.key === "ArrowUp") {
                    e.preventDefault();
                    setHighlight((h) => Math.max(0, h - 1));
                    return;
                  }
                  if (e.key === "Enter") {
                    e.preventDefault();
                    if (pendingNew) {
                      const p = newPropsFiltered[highlight];
                      if (p) void create({ componentUid: pendingNew.uid, propUid: p.uid });
                    } else if (creatingNew) {
                      const t = newTypes[highlight];
                      if (t) void createNew(t.type);
                    } else {
                      const p = allFilteredProps[highlight];
                      if (p) void create(p);
                    }
                    return;
                  }
                  e.stopPropagation();
                },
                placeholder: pendingNew ? "filter inputs…" : creatingNew ? "filter types…   ⇥ existing" : "filter…   ⇥ new",
                style: {
                  width: "100%",
                  background: "#0f1115",
                  color: "#e6e8eb",
                  border: "1px solid #2c313c",
                  borderRadius: 2,
                  padding: "3px 6px",
                  fontSize: 11,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  boxSizing: "border-box",
                  outline: "none"
                }
              }
            )
          ] }),
          /* @__PURE__ */ jsx("div", { style: { flex: 1, overflowY: "auto" }, children: pendingNew ? newPropsFiltered.length === 0 ? /* @__PURE__ */ jsxs("div", { style: { padding: "10px 8px", color: "#5a6172", fontSize: 11 }, children: [
            "no matching ",
            wantCategory === CATEGORY_INPUT ? "inputs" : "outputs"
          ] }) : newPropsFiltered.map((p, i) => /* @__PURE__ */ jsx(
            "button",
            {
              ref: i === highlight ? hlRef : void 0,
              onClick: () => create({ componentUid: pendingNew.uid, propUid: p.uid }),
              style: {
                display: "block",
                width: "100%",
                textAlign: "left",
                padding: "5px 8px",
                background: i === highlight ? "#2c3a55" : "transparent",
                color: "#e6e8eb",
                border: "none",
                cursor: "pointer",
                fontSize: 11,
                fontFamily: "ui-monospace, SFMono-Regular, monospace"
              },
              onMouseEnter: (e) => e.currentTarget.style.background = "#232733",
              onMouseLeave: (e) => e.currentTarget.style.background = i === highlight ? "#2c3a55" : "transparent",
              children: p.name
            },
            p.uid
          )) : creatingNew ? newTypes.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 8px", color: "#5a6172", fontSize: 11 }, children: ctx ? "no matching types" : "unavailable" }) : newTypes.map((t, i) => /* @__PURE__ */ jsxs(
            "button",
            {
              ref: i === highlight ? hlRef : void 0,
              onClick: () => createNew(t.type),
              style: {
                display: "flex",
                width: "100%",
                textAlign: "left",
                padding: "5px 8px",
                background: i === highlight ? "#2c3a55" : "transparent",
                color: "#e6e8eb",
                border: "none",
                cursor: "pointer",
                fontSize: 11,
                fontFamily: "ui-monospace, SFMono-Regular, monospace",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 6
              },
              onMouseEnter: (e) => e.currentTarget.style.background = "#232733",
              onMouseLeave: (e) => e.currentTarget.style.background = i === highlight ? "#2c3a55" : "transparent",
              children: [
                /* @__PURE__ */ jsx("span", { children: t.name }),
                /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 9 }, children: t.group })
              ]
            },
            t.type
          )) : filteredGroups.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 8px", color: "#5a6172", fontSize: 11 }, children: allComponents == null ? "loading…" : "no candidates" }) : filteredGroups.map((g, idx) => {
            const base = groupPropOffsets[idx];
            const containsHl = highlight >= base && highlight < base + g.props.length;
            const isOpen = f ? true : expanded === g.componentUid || containsHl;
            const prev = idx > 0 ? filteredGroups[idx - 1] : null;
            const tier = connectTier(g);
            const showSection = tier !== (prev ? connectTier(prev) : -1);
            const sectionLabel = tier === 0 ? "parent" : tier === 1 ? "same level" : tier === 2 ? `inside ${sourceName}` : "other folders";
            const folderPath = g.path.replace(/\/[^/]*$/, "").replace(/^root/, "");
            const showPath = tier === 3 && folderPath !== "";
            return /* @__PURE__ */ jsxs("div", { children: [
              showSection && /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    padding: "6px 8px 2px 8px",
                    color: "#5a6172",
                    fontSize: 9,
                    textTransform: "uppercase",
                    letterSpacing: 0.4,
                    borderTop: idx > 0 ? "1px solid #2c313c" : "none",
                    marginTop: idx > 0 ? 2 : 0
                  },
                  children: sectionLabel
                }
              ),
              /* @__PURE__ */ jsxs(
                "button",
                {
                  onClick: () => setExpanded((cur) => cur === g.componentUid ? null : g.componentUid),
                  style: {
                    display: "flex",
                    width: "100%",
                    textAlign: "left",
                    padding: "5px 8px",
                    background: "transparent",
                    color: "#e6e8eb",
                    border: "none",
                    cursor: "pointer",
                    fontSize: 11,
                    fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    alignItems: "center",
                    gap: 6
                  },
                  onMouseEnter: (e) => e.currentTarget.style.background = "#232733",
                  onMouseLeave: (e) => e.currentTarget.style.background = "transparent",
                  children: [
                    /* @__PURE__ */ jsx(
                      "span",
                      {
                        style: {
                          display: "flex",
                          alignItems: "center",
                          color: "#8892a0",
                          flexShrink: 0
                        },
                        children: isOpen ? /* @__PURE__ */ jsx(ChevronDown, { size: 13 }) : /* @__PURE__ */ jsx(ChevronRight, { size: 13 })
                      }
                    ),
                    /* @__PURE__ */ jsxs(
                      "span",
                      {
                        style: {
                          flex: 1,
                          minWidth: 0,
                          display: "flex",
                          flexDirection: "column",
                          overflow: "hidden"
                        },
                        children: [
                          /* @__PURE__ */ jsxs(
                            "span",
                            {
                              style: {
                                color: "#9ecbff",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap"
                              },
                              children: [
                                g.componentName,
                                g.isParent && /* @__PURE__ */ jsx(
                                  "span",
                                  {
                                    style: {
                                      marginLeft: 6,
                                      fontSize: 8,
                                      textTransform: "uppercase",
                                      letterSpacing: 0.4,
                                      color: "#ffd479",
                                      border: "1px solid #5a4a2a",
                                      background: "#2a2418",
                                      borderRadius: 3,
                                      padding: "0 4px"
                                    },
                                    children: "parent"
                                  }
                                )
                              ]
                            }
                          ),
                          showPath && /* @__PURE__ */ jsx(
                            "span",
                            {
                              style: {
                                color: "#5a6172",
                                fontSize: 9,
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap"
                              },
                              title: g.path,
                              children: folderPath
                            }
                          )
                        ]
                      }
                    ),
                    /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 10 }, children: g.props.length })
                  ]
                }
              ),
              isOpen && /* @__PURE__ */ jsx("div", { style: { paddingBottom: 2 }, children: g.props.map((p, pi) => {
                const isHl = base + pi === highlight;
                return /* @__PURE__ */ jsx(
                  "button",
                  {
                    ref: isHl ? hlRef : void 0,
                    onClick: () => create({ componentUid: g.componentUid, propUid: p.propUid }),
                    style: {
                      display: "block",
                      width: "100%",
                      textAlign: "left",
                      padding: "3px 8px 3px 28px",
                      background: isHl ? "#2c3a55" : "transparent",
                      color: "#e6e8eb",
                      border: "none",
                      cursor: "pointer",
                      fontSize: 11,
                      fontFamily: "ui-monospace, SFMono-Regular, monospace"
                    },
                    onMouseEnter: (e) => e.currentTarget.style.background = "#2c313c",
                    onMouseLeave: (e) => e.currentTarget.style.background = isHl ? "#2c3a55" : "transparent",
                    children: p.propName
                  },
                  p.propUid
                );
              }) })
            ] }, g.componentUid);
          }) })
        ]
      }
    ),
    document.body
  );
}
function MenuItem({ onClick, label, danger }) {
  return /* @__PURE__ */ jsx(
    "button",
    {
      onClick,
      style: {
        display: "block",
        width: "100%",
        textAlign: "left",
        padding: "5px 8px",
        background: "transparent",
        color: danger ? "#ffb8b8" : "#e6e8eb",
        border: "none",
        borderRadius: 2,
        cursor: "pointer",
        fontSize: 11,
        fontFamily: "inherit"
      },
      onMouseEnter: (e) => e.currentTarget.style.background = "#2c313c",
      onMouseLeave: (e) => e.currentTarget.style.background = "transparent",
      children: label
    }
  );
}
function PropertyValueEditor({
  componentUid,
  propName,
  value,
  dataType,
  facet
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const display = fmtValueFacet(value, dataType, facet);
  const start = () => {
    setDraft(value == null ? "" : typeof value === "string" ? value : String(value));
    setEditing(true);
  };
  const commit = async () => {
    setEditing(false);
    const raw = draft.trim();
    if (raw === "") return;
    let parsed;
    if (dataType === DATATYPE_BOOL) {
      const lower = raw.toLowerCase();
      parsed = lower === "true" || lower === "1" || lower === "yes";
    } else if (dataType === DATATYPE_STRING) {
      parsed = raw;
    } else {
      const n = Number(raw);
      if (!Number.isFinite(n)) return;
      parsed = n;
    }
    try {
      const { updateNode } = await Promise.resolve().then(() => rest);
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", e.message);
    }
  };
  if (editing) {
    const stop = (e) => e.stopPropagation();
    if (facet?.aliases && facet.aliases.length) {
      const cur = value === true ? 1 : value === false ? 0 : typeof value === "number" ? value : Number(value);
      return /* @__PURE__ */ jsx(
        "select",
        {
          autoFocus: true,
          className: "nodrag",
          value: String(cur),
          onChange: (e) => commitAlias(Number(e.target.value)),
          onKeyDown: (e) => {
            if (e.key === "Escape") setEditing(false);
            e.stopPropagation();
          },
          onBlur: () => setEditing(false),
          onClick: stop,
          onPointerDown: stop,
          style: editorInputStyle,
          children: facet.aliases.map((a) => /* @__PURE__ */ jsx("option", { value: String(a.code), children: a.label }, a.code))
        }
      );
    }
    if (dataType === DATATYPE_BOOL) {
      return /* @__PURE__ */ jsxs(
        "select",
        {
          autoFocus: true,
          className: "nodrag",
          value: draft,
          onChange: (e) => commitWith(e.target.value),
          onKeyDown: (e) => {
            if (e.key === "Escape") setEditing(false);
            e.stopPropagation();
          },
          onBlur: () => setEditing(false),
          onClick: stop,
          onPointerDown: stop,
          style: editorInputStyle,
          children: [
            /* @__PURE__ */ jsx("option", { value: "true", children: "true" }),
            /* @__PURE__ */ jsx("option", { value: "false", children: "false" })
          ]
        }
      );
    }
    return /* @__PURE__ */ jsx(
      "input",
      {
        autoFocus: true,
        className: "nodrag",
        type: dataType === DATATYPE_NUMBER ? "number" : "text",
        inputMode: dataType === DATATYPE_NUMBER ? "decimal" : void 0,
        step: dataType === DATATYPE_NUMBER ? "any" : void 0,
        value: draft,
        onChange: (e) => setDraft(e.target.value),
        onKeyDown: (e) => {
          if (e.key === "Enter") commit();
          else if (e.key === "Escape") setEditing(false);
          e.stopPropagation();
        },
        onBlur: commit,
        onClick: stop,
        onPointerDown: stop,
        style: editorInputStyle
      }
    );
  }
  return /* @__PURE__ */ jsx(
    "span",
    {
      onClick: (e) => {
        e.stopPropagation();
        start();
      },
      style: {
        color: dataType === DATATYPE_BOOL ? COLOR_BOOL : "#e6e8eb",
        fontVariantNumeric: "tabular-nums",
        cursor: "text",
        padding: "0 2px",
        borderRadius: 2
      },
      title: "click to edit",
      children: display
    }
  );
  async function commitWith(raw) {
    setDraft(raw);
    setEditing(false);
    let parsed;
    if (dataType === DATATYPE_BOOL) {
      parsed = raw === "true";
    } else if (dataType === DATATYPE_STRING) {
      parsed = raw;
    } else {
      const n = Number(raw);
      if (!Number.isFinite(n)) return;
      parsed = n;
    }
    try {
      const { updateNode } = await Promise.resolve().then(() => rest);
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", e.message);
    }
  }
  async function commitAlias(code) {
    setEditing(false);
    const parsed = dataType === DATATYPE_BOOL ? code === 1 : code;
    try {
      const { updateNode } = await Promise.resolve().then(() => rest);
      await updateNode(componentUid, { properties: { [propName]: { value: parsed } } });
    } catch (e) {
      console.error("update value failed:", e.message);
    }
  }
}
const editorInputStyle = {
  width: 90,
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #4a9eff",
  borderRadius: 2,
  padding: "0 4px",
  fontFamily: "inherit",
  fontSize: 11,
  textAlign: "right",
  outline: "none"
};
const overrideInputStyle = {
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #4a9eff",
  borderRadius: 2,
  padding: "3px 6px",
  fontFamily: "ui-monospace, SFMono-Regular, monospace",
  fontSize: 11,
  outline: "none"
};
function StatusDot({ color, text }) {
  const [hover, setHover] = useState(false);
  return /* @__PURE__ */ jsxs(
    "span",
    {
      onMouseEnter: () => setHover(true),
      onMouseLeave: () => setHover(false),
      style: {
        position: "relative",
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        flexShrink: 0
      },
      children: [
        /* @__PURE__ */ jsx(
          "span",
          {
            style: {
              width: 8,
              height: 8,
              borderRadius: 4,
              background: color,
              boxShadow: "0 0 0 1px rgba(0,0,0,0.4)",
              display: "block"
            }
          }
        ),
        hover && /* @__PURE__ */ jsx(
          "span",
          {
            style: {
              position: "absolute",
              top: "100%",
              right: 0,
              marginTop: 6,
              padding: "3px 7px",
              background: "#0f1115",
              border: "1px solid " + color,
              borderRadius: 3,
              color: "#e6e8eb",
              fontSize: 10,
              fontFamily: "ui-monospace, SFMono-Regular, monospace",
              whiteSpace: "nowrap",
              zIndex: 50,
              pointerEvents: "none",
              boxShadow: "0 2px 6px rgba(0,0,0,0.5)"
            },
            children: text || "—"
          }
        )
      ]
    }
  );
}
function parseStatus(raw) {
  if (raw == null) return "";
  if (typeof raw !== "string") {
    if (typeof raw === "object") return summarizeStatusObject(raw);
    return String(raw);
  }
  const t = raw.trim();
  if (t === "" || t === "{}") return "";
  try {
    const obj = JSON.parse(t);
    if (obj == null) return "";
    if (typeof obj === "string") return obj;
    if (typeof obj === "object") return summarizeStatusObject(obj);
  } catch {
  }
  return t;
}
function summarizeStatusObject(obj) {
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === "string" && v.trim() !== "") return `${k}: ${v}`;
    if (typeof v === "boolean" && v) return k;
    if (typeof v === "number" && v !== 0) return `${k}=${v}`;
  }
  return "";
}
function statusColorFor(s) {
  const v = s.toUpperCase();
  if (!v || v === "NONE" || v === "OK") return { bg: "#4ade80", label: v || "ok" };
  if (v === "STALE") return { bg: "#f59e0b", label: "stale" };
  if (v === "OVERRIDDEN") return { bg: "#9ecbff", label: "overridden" };
  if (v === "ERROR" || v === "FAULT" || v === "DOWN") return { bg: "#ef4444", label: v.toLowerCase() };
  return { bg: "#8892a0", label: s };
}
function FunctionBlockInner({ data, selected }) {
  const schemaV = useSchemaVersion((s) => s.version);
  const ctx = useContext(CeWiresheetContext);
  const restComp = useStructural((s) => s.components.get(data.componentUid));
  const ourUids = useMemo(() => {
    if (!restComp) return [];
    const own = Object.values(restComp.properties).map((p) => p.uid);
    for (const ep of exposedPorts(facetFor(restComp.uid, rawFacet(restComp.properties)))) {
      own.push(ep.childUid);
      if (ep.facet.facetProp != null) own.push(ep.facet.facetProp);
    }
    return own;
  }, [restComp]);
  const lod = useStore$1((s) => s.transform[2] < LOD_ZOOM);
  const valuesByUid = useValues(
    useShallow((s) => {
      if (lod) return EMPTY_VALUES;
      const out = {};
      for (const uid of ourUids) out[uid] = s.values.get(uid);
      return out;
    })
  );
  const flagsByUid = useStatusFlags(
    useShallow((s) => {
      if (lod) return EMPTY_FLAGS;
      const out = {};
      for (const uid of ourUids) out[uid] = s.flags.get(uid) ?? 0;
      return out;
    })
  );
  const [menu, setMenu] = useState(null);
  const otherSelectorKeys = usePresence(
    useShallow((s) => {
      const out = [];
      for (const c of s.collaborators.values()) {
        if (c.state.selectedComponents?.includes(data.componentUid)) {
          const name = c.state.userName ?? c.sessionId.slice(0, 6);
          out.push(`${PRESENCE_PALETTE[c.colorIdx]}	${name}`);
        }
      }
      return out;
    })
  );
  const otherSelectors = otherSelectorKeys.map((k) => {
    const [color, name] = k.split("	");
    return { color, name };
  });
  const ownFacetUid = restComp?.properties[FACET_PROP]?.uid;
  const liveFacetRaw = ownFacetUid != null && typeof valuesByUid[ownFacetUid] === "string" ? valuesByUid[ownFacetUid] : void 0;
  const prevFacetRaw = useRef(null);
  useEffect(() => {
    if (liveFacetRaw == null) return;
    if (prevFacetRaw.current === null) {
      prevFacetRaw.current = liveFacetRaw;
      return;
    }
    if (liveFacetRaw !== prevFacetRaw.current) {
      prevFacetRaw.current = liveFacetRaw;
      ctx?.requestReload?.();
    }
  }, [liveFacetRaw, ctx]);
  const structural = useMemo(() => {
    if (!restComp) return null;
    const isUserFacing = (p) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL;
    const entries = Object.entries(restComp.properties);
    const facet = facetFor(restComp.uid, rawFacet(restComp.properties));
    const mappedRows = entries.filter(([, p]) => isUserFacing(p)).map(([name, p]) => ({
      uid: p.uid,
      name,
      category: p.category,
      dataType: propertyDataType.get(p.uid) ?? inferDataType(p.value),
      systemRole: p.systemRole,
      facet: facet.get(p.uid)
    }));
    const hiddenCount2 = mappedRows.filter((r) => r.facet?.hidden).length;
    const userRows = mappedRows.filter((r) => !r.facet?.hidden);
    const portRows = exposedPorts(facet).map((ep) => ({
      uid: ep.childUid,
      name: ep.facet.label ?? `#${ep.childUid}`,
      category: ep.side === "input" ? CATEGORY_INPUT : CATEGORY_OUTPUT,
      dataType: propertyDataType.get(ep.childUid) ?? inferDataType(void 0),
      facet: ep.facet,
      exposed: true,
      exposedComponent: ep.facet.childComponent,
      facetPropUid: ep.facet.facetProp
    }));
    const allRows = [...userRows, ...portRows];
    const byOrder = (a, b) => (a.facet?.order ?? Number.MAX_SAFE_INTEGER) - (b.facet?.order ?? Number.MAX_SAFE_INTEGER);
    const rows2 = [
      ...allRows.filter((r) => r.category === CATEGORY_OUTPUT).sort(byOrder),
      ...allRows.filter((r) => r.category === CATEGORY_INPUT).sort(byOrder),
      ...allRows.filter((r) => r.category === CATEGORY_CONFIG).sort(byOrder)
    ];
    const statusEntry = entries.find(([, p]) => p.systemRole === ROLE_STATUS);
    const statusText2 = parseStatus(statusEntry?.[1].value);
    return {
      rows: rows2,
      // + ROW_H for the bottom lip (drill-in button + action marker).
      nodeH: TITLE_H + rows2.length * ROW_H + ROW_H,
      kind: restComp.type,
      statusText: statusText2,
      statusColor: statusColorFor(statusText2),
      statusPropExists: statusEntry != null,
      hiddenCount: hiddenCount2
    };
  }, [restComp, schemaV]);
  diagRecordRender("FunctionBlock");
  if (!restComp || !structural) {
    return /* @__PURE__ */ jsxs(
      "div",
      {
        style: {
          width: NODE_W,
          height: 40,
          background: "#1a1d24",
          border: "1px dashed #3b4350",
          borderRadius: 4,
          color: "#8892a0",
          fontSize: 11,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontFamily: "ui-monospace, SFMono-Regular, monospace"
        },
        children: [
          "uid ",
          data.componentUid
        ]
      }
    );
  }
  const values = valuesByUid;
  const statusFlagsMap = flagsByUid;
  const { rows, nodeH, kind, statusText, statusColor, statusPropExists, hiddenCount } = structural;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      onContextMenu: (e) => {
        let el = e.target;
        let uid = null;
        while (el && el !== e.currentTarget) {
          const v = el.dataset?.rowUid;
          if (v != null) {
            uid = Number(v);
            break;
          }
          el = el.parentElement;
        }
        if (uid == null) return;
        const p = rows.find((r) => r.uid === uid);
        if (!p) return;
        e.preventDefault();
        e.stopPropagation();
        const flags = statusFlagsMap[p.uid] ?? restComp.properties[p.name]?.statusFlags ?? 0;
        setMenu({
          x: e.clientX,
          y: e.clientY,
          propName: p.name,
          propUid: p.uid,
          category: p.category,
          dataType: p.dataType,
          currentValue: values[p.uid],
          overridden: (flags & STATUS_OVERRIDDEN) !== 0,
          exposed: !!p.exposed,
          exposedComponent: p.exposedComponent,
          portOwner: p.exposed ? data.componentUid : void 0
        });
      },
      style: {
        width: NODE_W,
        minHeight: nodeH,
        background: "#1a1d24",
        border: selected ? "1px solid #4a9eff" : otherSelectors.length > 0 ? `1px solid ${otherSelectors[0].color}` : "1px solid #2c313c",
        borderRadius: 4,
        color: "#e6e8eb",
        fontSize: 11,
        // Selection glow priority: our own selection (blue) wins; otherwise a
        // collaborator's selection paints a glow in their color. Both stack
        // their shadow over the default drop shadow.
        boxShadow: selected ? "0 0 0 1px #4a9eff, 0 0 12px rgba(74,158,255,0.45)" : otherSelectors.length > 0 ? `0 0 0 1px ${otherSelectors[0].color}, 0 0 10px ${otherSelectors[0].color}66` : "0 1px 2px rgba(0,0,0,0.4)",
        transition: "box-shadow 80ms ease, border-color 80ms ease",
        position: "relative",
        overflow: "visible"
      },
      children: [
        otherSelectors.length > 0 && /* @__PURE__ */ jsx(
          "div",
          {
            style: {
              position: "absolute",
              top: -9,
              left: 6,
              display: "flex",
              gap: 3,
              zIndex: 5,
              pointerEvents: "none"
            },
            children: otherSelectors.map((o) => /* @__PURE__ */ jsx(
              "span",
              {
                title: `${o.name} has this selected`,
                style: {
                  fontSize: 9,
                  lineHeight: "12px",
                  padding: "0 4px",
                  background: o.color,
                  color: "#0f1115",
                  borderRadius: 2,
                  fontWeight: 600,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  whiteSpace: "nowrap"
                },
                children: o.name
              },
              o.name
            ))
          }
        ),
        /* @__PURE__ */ jsxs(
          "div",
          {
            onContextMenu: (e) => {
              if (!data.onContextMenu) return;
              e.preventDefault();
              e.stopPropagation();
              data.onContextMenu(data.componentUid, e.clientX, e.clientY);
            },
            onDoubleClick: (e) => {
              e.stopPropagation();
              data.onEnter?.(data.componentUid);
            },
            style: {
              height: TITLE_H,
              padding: "4px 8px",
              background: "#232733",
              borderBottom: "1px solid #2c313c",
              display: "flex",
              flexDirection: "column",
              justifyContent: "center",
              boxSizing: "border-box"
            },
            children: [
              /* @__PURE__ */ jsxs("div", { style: { display: "flex", alignItems: "center", gap: 6 }, children: [
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      fontWeight: 600,
                      fontSize: 12,
                      flex: 1,
                      minWidth: 0,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap"
                    },
                    children: data.name ?? kind
                  }
                ),
                statusPropExists && /* @__PURE__ */ jsx(StatusDot, { color: statusColor.bg, text: statusText })
              ] }),
              /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    fontSize: 10,
                    lineHeight: 1.35,
                    color: "#8892a0",
                    fontFamily: "ui-monospace, monospace",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap"
                  },
                  title: kind,
                  children: kind
                }
              )
            ]
          }
        ),
        !lod && /* @__PURE__ */ jsxs(
          "div",
          {
            onDoubleClick: (e) => {
              e.stopPropagation();
              data.onEnter?.(data.componentUid);
            },
            title: "Double-click to enter this component's level",
            style: {
              position: "absolute",
              left: 0,
              right: 0,
              top: TITLE_H + rows.length * ROW_H,
              height: ROW_H,
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              padding: "0 8px",
              boxSizing: "border-box",
              borderTop: "1px solid #2c313c",
              background: "#1e222b",
              borderBottomLeftRadius: 4,
              borderBottomRightRadius: 4,
              cursor: "pointer"
            },
            children: [
              /* @__PURE__ */ jsxs("span", { style: { display: "flex", alignItems: "center", gap: 6 }, children: [
                data.hasActions && /* @__PURE__ */ jsx(
                  "span",
                  {
                    title: "This component has actions",
                    style: { display: "flex", alignItems: "center", color: "#ffd166" },
                    children: /* @__PURE__ */ jsx(Zap, { size: 12, strokeWidth: 2 })
                  }
                ),
                hiddenCount > 0 && /* @__PURE__ */ jsx(
                  "span",
                  {
                    title: `${hiddenCount} hidden propert${hiddenCount === 1 ? "y" : "ies"}`,
                    style: { display: "flex", alignItems: "center", color: "#5a6172" },
                    children: /* @__PURE__ */ jsx(EyeOff, { size: 12, strokeWidth: 2 })
                  }
                )
              ] }),
              data.hasChildren && /* @__PURE__ */ jsxs(
                "span",
                {
                  title: `Has ${data.childCount ?? ""} child${data.childCount === 1 ? "" : "ren"} — double-click to enter`,
                  style: {
                    display: "flex",
                    alignItems: "center",
                    gap: 3,
                    fontSize: 11,
                    color: "#9ecbff",
                    fontFamily: "ui-monospace, SFMono-Regular, monospace"
                  },
                  children: [
                    /* @__PURE__ */ jsx(Layers, { size: 12, strokeWidth: 2 }),
                    " ",
                    data.childCount ?? ""
                  ]
                }
              )
            ]
          }
        ),
        !lod && rows.map((p, i) => {
          const isInput = p.category === CATEGORY_INPUT;
          const isOutput = p.category === CATEGORY_OUTPUT;
          const v = values[p.uid];
          let rowFacet = p.facet;
          if (p.exposed && p.facetPropUid != null && p.exposedComponent != null) {
            const fv = values[p.facetPropUid];
            if (typeof fv === "string") {
              const live = facetFor(p.exposedComponent, fv).get(p.uid);
              if (live) rowFacet = { ...p.facet, ...live, label: live.label ?? p.facet?.label };
            }
          }
          const flags = statusFlagsMap[p.uid] ?? restComp.properties[p.name]?.statusFlags ?? 0;
          const overridden = (flags & STATUS_OVERRIDDEN) !== 0;
          const editable = !p.exposed && (isInput || p.category === CATEGORY_CONFIG);
          const rowTitle = `${p.name} — prop uid ${p.uid} · component uid ${p.exposed ? p.exposedComponent ?? "?" : data.componentUid}`;
          return /* @__PURE__ */ jsxs(
            "div",
            {
              "data-row-uid": p.uid,
              title: rowTitle,
              style: {
                position: "absolute",
                left: 0,
                right: 0,
                top: TITLE_H + i * ROW_H,
                height: ROW_H,
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "0 12px",
                fontSize: 11,
                fontFamily: "ui-monospace, SFMono-Regular, monospace",
                background: overridden ? "rgba(245,158,11,0.08)" : "transparent"
              },
              children: [
                /* @__PURE__ */ jsxs(
                  "span",
                  {
                    style: {
                      color: isInput ? "#8892a0" : isOutput ? "#cbd3e0" : "#9aa3b2",
                      display: "flex",
                      alignItems: "center",
                      gap: 4
                    },
                    children: [
                      p.exposed && /* @__PURE__ */ jsx(
                        "span",
                        {
                          style: { display: "flex", alignItems: "center", color: "#7a8a9f" },
                          title: "exposed from a child",
                          children: /* @__PURE__ */ jsx(CornerDownRight, { size: 11, strokeWidth: 2 })
                        }
                      ),
                      /* @__PURE__ */ jsx("span", { title: rowFacet?.label ? p.name : void 0, children: rowFacet?.label ?? p.name }),
                      p.category === CATEGORY_CONFIG ? " (cfg)" : "",
                      overridden && /* @__PURE__ */ jsx(
                        "span",
                        {
                          title: "overridden",
                          style: {
                            fontSize: 9,
                            padding: "0 4px",
                            background: "#f59e0b",
                            color: "#0f1115",
                            borderRadius: 2,
                            fontWeight: 600
                          },
                          children: "OVR"
                        }
                      )
                    ]
                  }
                ),
                editable ? /* @__PURE__ */ jsx(
                  PropertyValueEditor,
                  {
                    componentUid: data.componentUid,
                    propName: p.name,
                    value: v,
                    dataType: p.dataType,
                    facet: rowFacet
                  }
                ) : /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      color: p.dataType === DATATYPE_BOOL ? COLOR_BOOL : "#e6e8eb",
                      fontVariantNumeric: "tabular-nums",
                      // Same 2px horizontal padding as the inline editor's display
                      // span, so input and output values line up at the same right
                      // edge instead of outputs sitting 2px further right.
                      padding: "0 2px"
                    },
                    title: DATATYPE_LABEL[p.dataType],
                    children: fmtValueFacet(v, p.dataType, rowFacet)
                  }
                )
              ]
            },
            p.uid
          );
        }),
        rows.map((p, i) => {
          if (p.category === CATEGORY_CONFIG) return null;
          const isInput = p.category === CATEGORY_INPUT;
          const c = colorForType(p.dataType);
          const HANDLE_W = 35;
          const rowTop = TITLE_H + i * ROW_H;
          return /* @__PURE__ */ jsx(
            Handle,
            {
              id: String(p.uid),
              type: isInput ? "target" : "source",
              position: isInput ? Position.Left : Position.Right,
              "data-row-uid": p.uid,
              style: {
                top: rowTop,
                [isInput ? "left" : "right"]: 0,
                width: HANDLE_W,
                height: ROW_H,
                background: "transparent",
                border: "none",
                borderRadius: 0,
                // Cancel React Flow's default translate (which would push the handle
                // outside the node by 50% of its width). With translate(0,0) the box
                // sits exactly where `left:0` / `right:0` puts it — flush at the edge.
                transform: "none"
              },
              children: /* @__PURE__ */ jsx(
                "span",
                {
                  style: {
                    position: "absolute",
                    top: "50%",
                    // Center the visible marker ON the port-side edge of the hit box:
                    //   input → x=0 (box's left edge = node's left edge)
                    //   output → x=100% (box's right edge = node's right edge)
                    left: isInput ? 0 : "100%",
                    transform: "translate(-50%, -50%)",
                    width: 8,
                    height: 8,
                    background: c,
                    border: "1px solid #0f1115",
                    borderRadius: 1,
                    pointerEvents: "none"
                  }
                }
              )
            },
            `h-${p.uid}`
          );
        }),
        menu && /* @__PURE__ */ jsx(
          PropertyContextMenu,
          {
            x: menu.x,
            y: menu.y,
            propName: menu.propName,
            propUid: menu.propUid,
            category: menu.category,
            dataType: menu.dataType,
            currentValue: menu.currentValue,
            overridden: menu.overridden,
            exposed: menu.exposed,
            portOwner: menu.portOwner,
            componentUid: menu.exposedComponent ?? data.componentUid,
            onClose: () => setMenu(null)
          }
        )
      ]
    }
  );
}
const FunctionBlock = memo(FunctionBlockInner, (a, b) => {
  return a.selected === b.selected && a.data.componentUid === b.data.componentUid && a.data.name === b.data.name && a.data.hasChildren === b.data.hasChildren && a.data.childCount === b.data.childCount && a.data.onEnter === b.data.onEnter && a.data.onContextMenu === b.data.onContextMenu;
});
function GhostNodeInner({ data }) {
  const isInputSide = data.side === "input";
  const [popOpen, setPopOpen] = useState(false);
  const rootRef = useRef(null);
  const count = data.connections.length;
  const first = data.connections[0];
  const labelLeft = stripRoot(first?.externalPath ?? "");
  const labelRight = first?.externalPropName ?? "";
  const onClick = (e) => {
    e.stopPropagation();
    setPopOpen((v) => !v);
  };
  useEffect(() => {
    if (!popOpen) return;
    const dismiss = (ev) => {
      const el = ev.target;
      if (el && el.closest("[data-ce-ghost-pop]")) return;
      if (el && rootRef.current?.contains(el)) return;
      setPopOpen(false);
    };
    document.addEventListener("mousedown", dismiss);
    return () => document.removeEventListener("mousedown", dismiss);
  }, [popOpen]);
  return /* @__PURE__ */ jsxs(
    "div",
    {
      ref: rootRef,
      onClick,
      onDoubleClick: (e) => {
        e.stopPropagation();
        if (count === 1) {
          data.onNavigate?.(data.connections[0].externalComponentUid);
          setPopOpen(false);
        } else {
          setPopOpen(true);
        }
      },
      title: count === 1 ? `${first?.externalPath} · ${first?.externalPropName} — double-click to open` : `${count} cross-folder connections — click to expand`,
      style: {
        // Inner box fills whatever width the RF node was given. App.tsx sizes
        // each ghost to its content so this collapses tight around the text.
        width: "100%",
        height: GHOST_H,
        background: popOpen ? "#1a1d24" : "#0f1115",
        border: "1px dashed #5a6172",
        borderRadius: 3,
        display: "flex",
        alignItems: "center",
        padding: "0 8px",
        gap: 6,
        fontSize: 10,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        color: "#8892a0",
        whiteSpace: "nowrap",
        overflow: "hidden",
        cursor: "pointer",
        boxSizing: "border-box",
        // Make sure clicks reach us even though RF marks the node
        // non-selectable + non-draggable.
        pointerEvents: "all"
      },
      children: [
        /* @__PURE__ */ jsx(
          "span",
          {
            style: {
              color: "#9ecbff",
              overflow: "hidden",
              textOverflow: "ellipsis",
              minWidth: 0
            },
            children: labelLeft
          }
        ),
        /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", flexShrink: 0 }, children: "·" }),
        /* @__PURE__ */ jsx(
          "span",
          {
            style: {
              color: "#e6e8eb",
              overflow: "hidden",
              textOverflow: "ellipsis",
              minWidth: 0
            },
            children: labelRight
          }
        ),
        count > 1 && /* @__PURE__ */ jsxs(
          "span",
          {
            style: {
              flexShrink: 0,
              fontSize: 9,
              padding: "0 4px",
              background: "#3b6eff",
              color: "#fff",
              borderRadius: 2,
              fontWeight: 600
            },
            children: [
              "+",
              count - 1
            ]
          }
        ),
        /* @__PURE__ */ jsx(
          Handle,
          {
            id: data.handleId,
            type: isInputSide ? "target" : "source",
            position: isInputSide ? Position.Left : Position.Right,
            style: {
              width: 8,
              height: 8,
              background: "#5a6172",
              border: "1px solid #0f1115",
              borderRadius: 1,
              // Cancel React Flow's default 50% translate so the marker sits flush
              // at the ghost's edge, mirroring the real node's handle geometry.
              transform: "none",
              top: "50%",
              marginTop: -4,
              [isInputSide ? "left" : "right"]: -4
            }
          }
        ),
        popOpen && rootRef.current && /* @__PURE__ */ jsx(
          GhostPopover,
          {
            anchor: rootRef.current,
            isInputSide,
            connections: data.connections,
            onPick: (uid) => {
              setPopOpen(false);
              data.onNavigate?.(uid);
            },
            onDeleteEdge: data.onDeleteEdge
          }
        )
      ]
    }
  );
}
function GhostPopover({
  anchor,
  isInputSide,
  connections,
  onPick,
  onDeleteEdge
}) {
  const rect = anchor.getBoundingClientRect();
  const top = rect.bottom + 4;
  const left = isInputSide ? rect.left : rect.right - 220;
  return createPortal(
    /* @__PURE__ */ jsxs(
      "div",
      {
        "data-ce-ghost-pop": true,
        onClick: (e) => e.stopPropagation(),
        onContextMenu: (e) => e.preventDefault(),
        style: {
          position: "fixed",
          top,
          left,
          zIndex: 100,
          background: "#1a1d24",
          border: "1px solid #2c313c",
          borderRadius: 4,
          padding: 4,
          minWidth: 220,
          maxWidth: 360,
          maxHeight: 280,
          overflowY: "auto",
          boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
          fontSize: 11,
          color: "#e6e8eb",
          fontFamily: "ui-monospace, SFMono-Regular, monospace"
        },
        children: [
          /* @__PURE__ */ jsxs(
            "div",
            {
              style: {
                padding: "4px 8px 6px 8px",
                color: "#5a6172",
                fontSize: 9,
                textTransform: "uppercase",
                letterSpacing: 0.4,
                borderBottom: "1px solid #2c313c",
                marginBottom: 4
              },
              children: [
                connections.length,
                " connection",
                connections.length === 1 ? "" : "s"
              ]
            }
          ),
          connections.map((c) => {
            const pathLabel = stripRoot(c.externalPath);
            return /* @__PURE__ */ jsxs(
              "div",
              {
                style: {
                  display: "flex",
                  width: "100%",
                  alignItems: "center",
                  gap: 4,
                  borderRadius: 2
                },
                onMouseEnter: (e) => e.currentTarget.style.background = "#2c313c",
                onMouseLeave: (e) => e.currentTarget.style.background = "transparent",
                children: [
                  /* @__PURE__ */ jsxs(
                    "button",
                    {
                      onClick: () => onPick(c.externalComponentUid),
                      style: {
                        display: "flex",
                        flex: 1,
                        minWidth: 0,
                        alignItems: "baseline",
                        gap: 6,
                        padding: "4px 8px",
                        background: "transparent",
                        border: "none",
                        color: "#e6e8eb",
                        fontSize: 11,
                        fontFamily: "inherit",
                        cursor: "pointer",
                        textAlign: "left"
                      },
                      title: "open this component's folder",
                      children: [
                        /* @__PURE__ */ jsx("span", { style: { color: "#9ecbff", flexShrink: 0 }, children: pathLabel }),
                        /* @__PURE__ */ jsx("span", { style: { color: "#5a6172" }, children: "·" }),
                        /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb" }, children: c.externalPropName })
                      ]
                    }
                  ),
                  onDeleteEdge && /* @__PURE__ */ jsx(
                    "button",
                    {
                      onClick: () => void onDeleteEdge(c.edgeUid),
                      title: "delete this edge",
                      style: {
                        flexShrink: 0,
                        padding: "2px 6px",
                        marginRight: 4,
                        background: "transparent",
                        border: "1px solid transparent",
                        borderRadius: 2,
                        color: "#8892a0",
                        cursor: "pointer",
                        fontFamily: "inherit",
                        fontSize: 11
                      },
                      onMouseEnter: (e) => {
                        e.currentTarget.style.background = "#3a1a1a";
                        e.currentTarget.style.color = "#ffb8b8";
                        e.currentTarget.style.borderColor = "#6b2a2a";
                      },
                      onMouseLeave: (e) => {
                        e.currentTarget.style.background = "transparent";
                        e.currentTarget.style.color = "#8892a0";
                        e.currentTarget.style.borderColor = "transparent";
                      },
                      children: "✕"
                    }
                  )
                ]
              },
              c.edgeUid
            );
          })
        ]
      }
    ),
    document.body
  );
}
const GhostNode = memo(GhostNodeInner, (a, b) => {
  return a.data.connections === b.data.connections && a.data.handleId === b.data.handleId && a.data.side === b.data.side && a.data.width === b.data.width && a.data.onNavigate === b.data.onNavigate;
});
function userFacingRowIndex(comp, propName) {
  const entries = Object.entries(comp.properties).filter(
    ([, p]) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL
  );
  const order = [CATEGORY_OUTPUT, CATEGORY_INPUT, CATEGORY_CONFIG];
  const sorted = [];
  for (const cat of order) {
    for (const [n, p] of entries) {
      if (p.category === cat) sorted.push(n);
    }
  }
  return sorted.indexOf(propName);
}

const MARGIN = 400;
const EST_H = 240;
const DEBOUNCE_MS = 200;
function VisibilitySub({ onVisible }) {
  const rf = useReactFlow();
  const transform = useStore$1((s) => s.transform);
  const nodeCount = useStore$1((s) => s.nodes.length);
  const timer = useRef(null);
  useEffect(() => {
    if (timer.current != null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      const [tx, ty, zoom] = transform;
      if (!zoom) return;
      const W = window.innerWidth;
      const H = window.innerHeight;
      const vx0 = (0 - tx) / zoom - MARGIN;
      const vy0 = (0 - ty) / zoom - MARGIN;
      const vx1 = (W - tx) / zoom + MARGIN;
      const vy1 = (H - ty) / zoom + MARGIN;
      const visible = /* @__PURE__ */ new Set();
      for (const n of rf.getNodes()) {
        if (n.type === "ghost") continue;
        const px = n.position.x;
        const py = n.position.y;
        const w = n.width ?? NODE_W;
        const h = n.measured?.height ?? EST_H;
        if (px + w >= vx0 && px <= vx1 && py + h >= vy0 && py <= vy1) {
          const uid = Number(n.id);
          if (Number.isFinite(uid)) visible.add(uid);
        }
      }
      onVisible(visible);
    }, DEBOUNCE_MS);
    return () => {
      if (timer.current != null) window.clearTimeout(timer.current);
    };
  }, [transform, nodeCount, rf, onVisible]);
  return null;
}

const alignUp = (n, m) => n + (m - 1) & -8;
const STR_DECODER = new TextDecoder();
function decodeBinaryFrame(buf) {
  const view = new DataView(buf);
  const msgType = view.getUint8(0);
  const timestampMs = view.getUint32(4, true);
  const sectionCount = view.getUint8(8);
  const sections = [];
  let off = 16;
  for (let s = 0; s < sectionCount; s++) {
    off = alignUp(off, 8);
    const typeTag = view.getUint8(off);
    const count = view.getUint32(off + 4, true);
    const sectionBytes = view.getUint32(off + 8, true);
    const sectionStart = off;
    const uidsOff = off + 16;
    const uids = new Uint32Array(buf, uidsOff, count);
    let payloadOff = uidsOff + count * 4;
    const is64 = (typeTag & 240) === 32;
    if (is64) payloadOff = alignUp(payloadOff, 8);
    let values;
    switch (typeTag) {
      case TYPE_BOOL: {
        const bytes = new Uint8Array(buf, payloadOff, count);
        const out = new Array(count);
        for (let i = 0; i < count; i++) out[i] = bytes[i] !== 0;
        values = out;
        break;
      }
      case TYPE_U32:
      case TYPE_STATUS:
        values = new Uint32Array(buf, payloadOff, count);
        break;
      case TYPE_I32:
        values = new Int32Array(buf, payloadOff, count);
        break;
      case TYPE_F32:
        values = new Float32Array(buf, payloadOff, count);
        break;
      case TYPE_U64: {
        const u = new BigUint64Array(buf, payloadOff, count);
        const out = new Array(count);
        for (let i = 0; i < count; i++) {
          const v = u[i];
          out[i] = v <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(v) : v;
        }
        values = out;
        break;
      }
      case TYPE_I64: {
        const a = new BigInt64Array(buf, payloadOff, count);
        const out = new Array(count);
        for (let i = 0; i < count; i++) {
          const v = a[i];
          out[i] = v <= BigInt(Number.MAX_SAFE_INTEGER) && v >= BigInt(Number.MIN_SAFE_INTEGER) ? Number(v) : v;
        }
        values = out;
        break;
      }
      case TYPE_F64:
        values = new Float64Array(buf, payloadOff, count);
        break;
      case TYPE_STR: {
        const offsets = new Uint32Array(buf, payloadOff, count + 1);
        const blobOff = payloadOff + (count + 1) * 4;
        const blob = new Uint8Array(buf, blobOff, offsets[count]);
        const dec = STR_DECODER;
        const out = new Array(count);
        for (let i = 0; i < count; i++) {
          out[i] = dec.decode(blob.subarray(offsets[i], offsets[i + 1]));
        }
        values = out;
        break;
      }
      default:
        values = [];
        break;
    }
    sections.push({ typeTag, uids, values });
    off = sectionStart + sectionBytes;
  }
  return { msgType, timestampMs, sections };
}

function diffSets(current, desired) {
  const added = [];
  const removed = [];
  for (const uid of desired) if (!current.has(uid)) added.push(uid);
  for (const uid of current) if (!desired.has(uid)) removed.push(uid);
  return { added, removed };
}

const SESSION_STORAGE_KEY = "ce-ui.sessionId";
const TICKHZ_STORAGE_KEY = "ce-ui.tickHz";
const RECONNECT_MS = 500;
const RECONNECT_MAX_MS = 3e4;
const STABLE_MS = 5e3;
const SESSION_CHANNEL = "ce-ui.session-ownership";
const tabId = `${Math.floor(performance.now())}-${Math.trunc(performance.timeOrigin) % 1e5}`;
let sessionChannel = null;
let activeOwnedSessionId = null;
function ensureChannel() {
  if (sessionChannel) return sessionChannel;
  if (typeof BroadcastChannel === "undefined") return null;
  sessionChannel = new BroadcastChannel(SESSION_CHANNEL);
  sessionChannel.onmessage = (ev) => {
    const m = ev.data;
    if (m.kind === "own?" && m.from !== tabId && m.sessionId === activeOwnedSessionId) {
      sessionChannel?.postMessage({ kind: "owned", sessionId: m.sessionId, by: tabId });
    }
  };
  return sessionChannel;
}
function isSessionFree(candidate, timeoutMs = 180) {
  const ch = ensureChannel();
  if (!ch) return Promise.resolve(true);
  return new Promise((resolve) => {
    let settled = false;
    const onReply = (ev) => {
      const m = ev.data;
      if (m.kind === "owned" && m.sessionId === candidate && m.by !== tabId && !settled) {
        settled = true;
        ch.removeEventListener("message", onReply);
        resolve(false);
      }
    };
    ch.addEventListener("message", onReply);
    ch.postMessage({ kind: "own?", sessionId: candidate, from: tabId });
    window.setTimeout(() => {
      if (settled) return;
      settled = true;
      ch.removeEventListener("message", onReply);
      resolve(true);
    }, timeoutMs);
  });
}
class CeRestWs {
  ws = null;
  reconnectTimer = null;
  explicitlyClosed = false;
  subscribedComponents = /* @__PURE__ */ new Set();
  desiredSubscribed = /* @__PURE__ */ new Set();
  // Property-level subscription (alongside component-level) — used for exposed
  // ports, where we want a single off-canvas prop's value, not its whole
  // component. Diffed/sent the same way as components.
  subscribedProps = /* @__PURE__ */ new Set();
  desiredSubscribedProps = /* @__PURE__ */ new Set();
  sessionId = null;
  // Highest topology `seq` we've received. Sent on reconnect via `lastSeq` so the server
  // can replay missed topology events from its ring buffer instead of forcing a full
  // re-bootstrap.
  lastSeq = null;
  // Desired value/status push rate for this session. null = use engine
  // default. Persisted so a reconnect re-applies it via the configure
  // message. Live changes also go out as a `setRate` message.
  tickHz = null;
  // Reconnect backoff state.
  reconnectDelay = RECONNECT_MS;
  openedAt = null;
  url;
  h;
  constructor(url, h) {
    this.url = url;
    this.h = h;
    try {
      const savedHz = window.localStorage.getItem(TICKHZ_STORAGE_KEY);
      if (savedHz != null) {
        const n = Number(savedHz);
        if (Number.isFinite(n) && n >= 1 && n <= 1e3) this.tickHz = n;
      }
    } catch {
    }
    try {
      this.sessionId = window.sessionStorage.getItem(SESSION_STORAGE_KEY);
    } catch {
      this.sessionId = null;
    }
  }
  // Run the cross-tab ownership check once, before the first connect, so a
  // duplicated tab drops the copied sessionId and gets its own session. Only
  // gates the FIRST connect — reconnects keep the established session.
  resumeChecked = false;
  async ensureResumeAllowed() {
    if (this.resumeChecked) return;
    this.resumeChecked = true;
    if (!this.sessionId) return;
    const free = await isSessionFree(this.sessionId);
    if (!free) {
      recordEvent("ws-open", `session ${this.sessionId.slice(0, 8)} owned by another tab → fresh`);
      this.sessionId = null;
      try {
        window.sessionStorage.removeItem(SESSION_STORAGE_KEY);
      } catch {
      }
    }
  }
  async connect() {
    if (this.ws) return;
    await this.ensureResumeAllowed();
    if (this.ws) return;
    this.explicitlyClosed = false;
    const ws = new WebSocket(this.url);
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    ws.onopen = () => {
      metrics.wsConnected = true;
      this.openedAt = performance.now();
      this.h.onOpen();
      const msg = { type: "configure" };
      if (this.sessionId) msg.sessionId = this.sessionId;
      if (this.lastSeq != null) msg.lastSeq = this.lastSeq;
      if (this.tickHz != null) msg.tickHz = this.tickHz;
      ws.send(JSON.stringify(msg));
      recordEvent(
        "ws-open",
        `→ configure${this.sessionId ? ` sid=${this.sessionId.slice(0, 8)}` : ""}${this.lastSeq != null ? ` lastSeq=${this.lastSeq}` : ""}${this.tickHz != null ? ` tickHz=${this.tickHz}` : ""}`
      );
      this.subscribedComponents.clear();
      this.subscribedProps.clear();
    };
    ws.onclose = () => {
      metrics.wsConnected = false;
      metrics.reconnectCount++;
      this.ws = null;
      const lived = this.openedAt != null ? performance.now() - this.openedAt : 0;
      this.openedAt = null;
      if (lived >= STABLE_MS) {
        this.reconnectDelay = RECONNECT_MS;
      } else {
        this.reconnectDelay = Math.min(this.reconnectDelay * 2, RECONNECT_MAX_MS);
      }
      recordEvent("ws-close", `connection closed (lived ${lived.toFixed(0)}ms, next in ${this.reconnectDelay}ms)`);
      this.h.onClose();
      if (!this.explicitlyClosed) this.scheduleReconnect();
    };
    ws.onerror = () => {
    };
    ws.onmessage = (ev) => this.handleMessage(ev.data);
  }
  close() {
    this.explicitlyClosed = true;
    if (this.reconnectTimer != null) {
      window.clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
  }
  scheduleReconnect() {
    if (this.reconnectTimer != null) return;
    if (typeof document !== "undefined" && document.hidden) {
      const onVisible = () => {
        if (document.hidden) return;
        document.removeEventListener("visibilitychange", onVisible);
        if (!this.explicitlyClosed && !this.ws) void this.connect();
      };
      document.addEventListener("visibilitychange", onVisible);
      return;
    }
    const delay = Math.random() * this.reconnectDelay;
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      void this.connect();
    }, delay);
  }
  /**
   * Set this session's value/status push rate (Hz). Clamped [1,1000]; the
   * engine clamps too. Persisted so a reconnect re-applies it. Sends a live
   * `setRate` message if the socket is open; otherwise the next configure
   * carries it.
   */
  setRate(hz) {
    const clamped = Math.max(1, Math.min(1e3, Math.round(hz)));
    this.tickHz = clamped;
    try {
      window.localStorage.setItem(TICKHZ_STORAGE_KEY, String(clamped));
    } catch {
    }
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: "setRate", tickHz: clamped }));
      recordEvent("rest", `→ setRate ${clamped}Hz`);
    }
    diagGauges.tickHz = clamped;
  }
  getRate() {
    return this.tickHz;
  }
  /**
   * Publish opaque presence state for this session. The engine relays it
   * verbatim to other sessions (last-write-wins, ≤4 KB). No-op if the socket
   * isn't open — presence is ephemeral, a missed publish self-heals on the
   * next selection change.
   */
  publishPresence(state) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    this.ws.send(JSON.stringify({ type: "presence", state }));
  }
  /**
   * Diff-and-send subscribe / unsubscribe. Caller sets the desired component set; we
   * only emit the delta vs. what the server currently holds.
   */
  setDesiredSubscription(desired) {
    this.desiredSubscribed = desired;
    this.flushSubscriptions();
  }
  /** Property-level subscription (exposed ports). Diff-and-send like components. */
  setDesiredPropSubscription(desired) {
    this.desiredSubscribedProps = desired;
    this.flushSubscriptions();
  }
  flushSubscriptions() {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return;
    const { added, removed } = diffSets(this.subscribedComponents, this.desiredSubscribed);
    if (added.length > 0) {
      this.ws.send(JSON.stringify({ type: "subscribe", components: added }));
      for (const u of added) this.subscribedComponents.add(u);
      recordEvent("subscribe", `+[${added.join(",")}]`);
    }
    if (removed.length > 0) {
      this.ws.send(JSON.stringify({ type: "unsubscribe", components: removed }));
      for (const u of removed) this.subscribedComponents.delete(u);
      recordEvent("unsubscribe", `-[${removed.join(",")}]`);
    }
    const { added: addedP, removed: removedP } = diffSets(
      this.subscribedProps,
      this.desiredSubscribedProps
    );
    if (addedP.length > 0) {
      this.ws.send(JSON.stringify({ type: "subscribe", properties: addedP }));
      for (const u of addedP) this.subscribedProps.add(u);
      recordEvent("subscribe", `props +[${addedP.join(",")}]`);
    }
    if (removedP.length > 0) {
      this.ws.send(JSON.stringify({ type: "unsubscribe", properties: removedP }));
      for (const u of removedP) this.subscribedProps.delete(u);
      recordEvent("unsubscribe", `props -[${removedP.join(",")}]`);
    }
  }
  /** Most-recently observed sessionId; exposed so REST mutations can send the
   *  X-CE-Session header for change attribution. */
  getSessionId() {
    return this.sessionId;
  }
  handleMessage(data) {
    const isBinary = typeof data !== "string";
    const bytes = isBinary ? data.byteLength : data.length;
    recordMessage(bytes, isBinary);
    const t0 = performance.now();
    try {
      this.dispatchMessage(data);
    } finally {
      recordParse(performance.now() - t0);
    }
  }
  dispatchMessage(data) {
    if (typeof data === "string") {
      let msg;
      try {
        msg = JSON.parse(data);
      } catch {
        return;
      }
      diagRecordMessage(msg.type ?? "unknown", data.length);
      if (msg.type === "schema") {
        const s = msg;
        if (s.sessionId) {
          this.sessionId = s.sessionId;
          activeOwnedSessionId = s.sessionId;
          ensureChannel();
          try {
            window.sessionStorage.setItem(SESSION_STORAGE_KEY, s.sessionId);
          } catch {
          }
        }
        if (typeof s.currentSeq === "number") {
          this.lastSeq = s.currentSeq;
          metrics.lastSeq = s.currentSeq;
        }
        metrics.sessionId = s.sessionId ?? "";
        recordEvent(
          "schema",
          `sid=${s.sessionId?.slice(0, 8) ?? "—"} seq=${s.currentSeq} props=${s.properties?.length ?? 0}${s.resumed ? " (resumed)" : ""}`
        );
        this.h.onSchema(s);
        this.flushSubscriptions();
        if (this.tickHz != null) {
          this.ws?.send(JSON.stringify({ type: "setRate", tickHz: this.tickHz }));
          recordEvent("rest", `→ setRate ${this.tickHz}Hz (assert post-schema)`);
        }
        return;
      }
      if (msg.type === "topologyAdded" || msg.type === "topologyRemoved" || msg.type === "topologyChanged") {
        const t = msg;
        if (this.lastSeq != null && t.seq !== this.lastSeq + 1) {
          this.ws?.close();
          return;
        }
        this.lastSeq = t.seq;
        metrics.lastSeq = t.seq;
        if (t.type === "topologyAdded") recordTopology("added");
        else if (t.type === "topologyRemoved") recordTopology("removed");
        else recordTopology("changed");
        recordEvent("topology", summarizeTopology(t));
        this.h.onTopology(t);
        return;
      }
      if (msg.type === "presence") {
        const p = msg;
        recordEvent("rest", `presence ← ${p.sessionId?.slice(0, 8)}`);
        this.h.onPresence(p);
        return;
      }
      if (msg.type === "presenceSnapshot") {
        const p = msg;
        recordEvent("rest", `presenceSnapshot (${p.presences?.length ?? 0})`);
        this.h.onPresenceSnapshot(p);
        return;
      }
      if (msg.type === "presenceLeft") {
        const p = msg;
        recordEvent("rest", `presenceLeft ${p.sessionId?.slice(0, 8)}`);
        this.h.onPresenceLeft(p);
        return;
      }
      if (msg.type === "presenceError") {
        recordEvent("rest", `presenceError: ${msg.reason ?? "?"}`);
        return;
      }
      return;
    }
    const frame = decodeBinaryFrame(data);
    if (frame.msgType !== MSG_UPDATE && frame.msgType !== MSG_SNAPSHOT) return;
    let n = 0;
    let valUids = 0;
    let statusUids = 0;
    for (const s of frame.sections) {
      n += s.uids.length;
      if (s.typeTag === TYPE_STATUS) {
        statusUids += s.uids.length;
        diagRecordStatusUids(s.uids);
      } else {
        valUids += s.uids.length;
        diagRecordValueUids(s.uids);
      }
    }
    recordValueCount(n, frame.sections.length, data.byteLength);
    diagRecordMessage("binaryFrame", data.byteLength);
    diagRecordFrame(valUids, statusUids, data.byteLength);
    recordEvent(
      "frame",
      `${frame.msgType === MSG_SNAPSHOT ? "snapshot" : "update"} ${valUids}v +${statusUids}s · ${frame.sections.length} sec · ${data.byteLength}B`
    );
    this.h.onFrame(frame);
  }
}
function summarizeTopology(t) {
  if (t.type === "topologyAdded") {
    return `+ comp[${t.components.map((c) => c.uid).join(",")}] edge[${t.edges.map((e) => e.uid).join(",")}]`;
  }
  if (t.type === "topologyRemoved") {
    return `- comp[${t.componentUids.join(",")}] edge[${t.edgeUids.join(",")}]`;
  }
  return `~ comp[${t.components.map((c) => c.uid).join(",")}]`;
}
function wsUrlFromBase(origin) {
  return `${origin.replace(/^http/, "ws").replace(/\/+$/, "")}/ws`;
}

function sanitizeName(type) {
  const idx = type.lastIndexOf("::");
  const local = idx >= 0 ? type.slice(idx + 2) : type;
  const cleaned = local.replace(/[^A-Za-z0-9_]/g, "");
  return cleaned || "node";
}
function uniqueName(base, taken) {
  const set = taken instanceof Set ? taken : new Set(taken);
  let name = base;
  let n = 1;
  while (set.has(name)) {
    n += 1;
    name = `${base}${n}`;
  }
  return name;
}

const STACK_OFFSET = 16;
function layoutPositions(comps, nodeWidth) {
  const allZero = comps.every(
    (c) => (c.metadata?.position?.x ?? 0) === 0 && (c.metadata?.position?.y ?? 0) === 0
  );
  const cols = Math.max(1, Math.ceil(Math.sqrt(comps.length)));
  const GRID_X = nodeWidth + 60;
  const GRID_Y = 220;
  const stackSeen = /* @__PURE__ */ new Map();
  return comps.map((c, i) => {
    const px = c.metadata?.position?.x ?? 0;
    const py = c.metadata?.position?.y ?? 0;
    if (allZero) {
      return { x: i % cols * GRID_X, y: Math.floor(i / cols) * GRID_Y };
    }
    const key = `${px},${py}`;
    const dup = stackSeen.get(key) ?? 0;
    stackSeen.set(key, dup + 1);
    return { x: px + dup * STACK_OFFSET, y: py + dup * STACK_OFFSET };
  });
}

function groupBoundary(group, edges, comps) {
  const boundary = /* @__PURE__ */ new Map();
  for (const e of edges) {
    const srcIn = group.has(e.sourceUid);
    const dstIn = group.has(e.targetUid);
    if (srcIn === dstIn) continue;
    if (srcIn) {
      const child = comps.get(e.sourceUid);
      const propUid = e.sourcePropertyUid ?? child?.properties[e.sourceProperty]?.uid;
      if (propUid != null) {
        boundary.set(propUid, {
          childComponent: e.sourceUid,
          side: "output",
          label: e.sourceProperty,
          facetProp: child?.properties[FACET_PROP]?.uid
        });
      }
    } else {
      const child = comps.get(e.targetUid);
      const propUid = e.targetPropertyUid ?? child?.properties[e.targetProperty]?.uid;
      if (propUid != null) {
        boundary.set(propUid, {
          childComponent: e.targetUid,
          side: "input",
          label: e.targetProperty,
          facetProp: child?.properties[FACET_PROP]?.uid
        });
      }
    }
  }
  return boundary;
}

function buildSearchIndex(nodes, currentParentUid) {
  const flat = [];
  const walk = (c) => {
    if (c.uid !== 0) {
      const path = c.path.startsWith("root/") ? c.path.slice(5) : c.path;
      const here = c.parent === currentParentUid;
      const compName = c.name || c.type;
      flat.push({ compUid: c.uid, compName, type: c.type, path, here });
      const facet = parseFacet(rawFacet(c.properties) ?? "");
      for (const [propName, p] of Object.entries(c.properties)) {
        if ((p.systemRole ?? ROLE_NORMAL) !== ROLE_NORMAL) continue;
        const fc = facet.get(p.uid);
        const aliasText = fc?.aliases?.map((a) => a.label).join(" ") ?? "";
        if (!fc?.label && !aliasText) continue;
        flat.push({
          compUid: c.uid,
          compName,
          type: c.type,
          path,
          here,
          propName,
          label: fc?.label,
          aliasText
        });
      }
    }
    c.children?.forEach(walk);
  };
  nodes.forEach(walk);
  return flat;
}
function rankSearchHits(all, query) {
  const f = query.trim().toLowerCase();
  if (!f) return all.filter((h) => !h.propName).slice(0, 60);
  return all.map((h) => {
    let score = -1;
    if (h.propName) {
      const label = (h.label ?? "").toLowerCase();
      const al = (h.aliasText ?? "").toLowerCase();
      const pn = h.propName.toLowerCase();
      if (label === f || al.split(" ").includes(f)) score = 1;
      else if (label.startsWith(f) || pn.startsWith(f)) score = 2;
      else if (label.includes(f) || al.includes(f) || pn.includes(f)) score = 3;
    } else {
      const name = h.compName.toLowerCase();
      if (name === f) score = 0;
      else if (name.startsWith(f)) score = 1;
      else if (name.includes(f)) score = 2;
      else if (h.path.toLowerCase().includes(f) || h.type.toLowerCase().includes(f)) score = 3;
    }
    return { h, score };
  }).filter((x) => x.score >= 0).sort(
    (a, b) => Number(b.h.here) - Number(a.h.here) || a.score - b.score || a.h.compName.localeCompare(b.h.compName)
  ).slice(0, 80).map((x) => x.h);
}

function partitionEdges(edges, childUids) {
  const inEdges = [];
  const crossEdges = [];
  for (const e of edges) {
    const src = childUids.has(e.sourceUid);
    const dst = childUids.has(e.targetUid);
    if (src && dst) inEdges.push(e);
    else if (src !== dst) crossEdges.push(e);
  }
  return { inEdges, crossEdges };
}
function exposedPortIndex(children) {
  const index = /* @__PURE__ */ new Map();
  const remap = /* @__PURE__ */ new Map();
  const subProps = /* @__PURE__ */ new Set();
  for (const child of children) {
    for (const ep of exposedPorts(facetFor(child.uid, rawFacet(child.properties)))) {
      index.set(ep.childUid, { parentUid: child.uid });
      if (ep.facet.childComponent != null) remap.set(ep.childUid, ep.facet.childComponent);
      subProps.add(ep.childUid);
      if (ep.facet.facetProp != null) subProps.add(ep.facet.facetProp);
    }
  }
  return { index, remap, subProps };
}
function classifyCrossEdge(e, childUids, index) {
  const externalIsTarget = childUids.has(e.sourceUid);
  const externalPropUid = externalIsTarget ? e.targetPropertyUid : e.sourcePropertyUid;
  const exposed = externalPropUid != null ? index.get(externalPropUid) : void 0;
  if (exposed) {
    const visibleUid = externalIsTarget ? e.sourceUid : e.targetUid;
    const visiblePropUid = externalIsTarget ? e.sourcePropertyUid : e.targetPropertyUid;
    if (visiblePropUid != null) {
      return {
        kind: "port",
        edgeUid: e.uid,
        loopBack: e.loopBack === true,
        externalIsTarget,
        visibleUid,
        visiblePropUid,
        portParentUid: exposed.parentUid,
        portHandle: externalPropUid
      };
    }
  }
  return {
    kind: "ghost",
    edgeUid: e.uid,
    loopBack: e.loopBack === true,
    externalIsTarget,
    side: externalIsTarget ? "input" : "output",
    visibleUid: externalIsTarget ? e.sourceUid : e.targetUid,
    visiblePropName: externalIsTarget ? e.sourceProperty : e.targetProperty,
    externalUid: externalIsTarget ? e.targetUid : e.sourceUid,
    externalPropName: externalIsTarget ? e.targetProperty : e.sourceProperty,
    externalPath: (externalIsTarget ? e.targetPath : e.sourcePath) ?? ""
  };
}

function planPaste(clones, destParentUid, cursor, uidMap) {
  const all = [];
  const flatten = (c) => {
    all.push(c);
    c.children?.forEach(flatten);
  };
  clones.forEach(flatten);
  const topLevel = all.filter((c) => c.parent === destParentUid);
  const xs = topLevel.map((c) => c.metadata?.position?.x ?? 0);
  const ys = topLevel.map((c) => c.metadata?.position?.y ?? 0);
  const dx = topLevel.length ? cursor.x - (Math.min(...xs) + Math.max(...xs)) / 2 : 0;
  const dy = topLevel.length ? cursor.y - (Math.min(...ys) + Math.max(...ys)) / 2 : 0;
  const compMap = uidMap?.components ?? {};
  const propMap = uidMap?.properties ?? {};
  const topSet = new Set(topLevel.map((c) => c.uid));
  const updates = [];
  for (const c of all) {
    const entry = { uid: c.uid };
    if (topSet.has(c.uid)) {
      entry.position = {
        x: Math.round((c.metadata?.position?.x ?? 0) + dx),
        y: Math.round((c.metadata?.position?.y ?? 0) + dy)
      };
    }
    if (uidMap) {
      const raw = rawFacet(c.properties);
      if (raw) {
        const remapped = remapFacetUids(raw, compMap, propMap);
        if (remapped !== raw) entry.properties = { [FACET_PROP]: { value: remapped } };
      }
    }
    if (entry.position || entry.properties) updates.push(entry);
  }
  return { updates, newUids: topLevel.map((c) => c.uid) };
}

function moveCandidates(allComponents, movingUids) {
  const movingSet = new Set(movingUids);
  const movingPaths = allComponents.filter((c) => movingSet.has(c.uid)).map((c) => c.path);
  const isMovingOrDescendant = (path) => movingPaths.some((mp) => path === mp || path.startsWith(mp + "/"));
  const movingComp = allComponents.find((c) => movingSet.has(c.uid));
  const curFolderUid = movingComp?.parent;
  const curFolder = allComponents.find((c) => c.uid === curFolderUid);
  const upUid = curFolder?.parent;
  const curFolderPath = curFolder?.path;
  const tierOf = (c) => {
    if (upUid !== void 0 && c.uid === upUid) return 0;
    if (curFolderUid !== void 0 && c.parent === curFolderUid) return 1;
    if (curFolderPath && c.path.startsWith(curFolderPath + "/")) return 2;
    return 3;
  };
  const candidates = [];
  for (const c of allComponents) {
    if (movingSet.has(c.uid)) continue;
    if (isMovingOrDescendant(c.path)) continue;
    candidates.push({ uid: c.uid, name: c.name || c.type, kind: c.type, path: c.path, tier: tierOf(c) });
  }
  candidates.sort((a, b) => a.tier !== b.tier ? a.tier - b.tier : a.path.localeCompare(b.path));
  return candidates;
}
function filterMoveCandidates(candidates, filter) {
  const f = filter.trim().toLowerCase();
  if (!f) return candidates;
  return candidates.filter(
    (c) => c.name.toLowerCase().includes(f) || c.kind.toLowerCase().includes(f) || c.path.toLowerCase().includes(f)
  );
}

const EDGE_SELECTED_CSS = `
  .react-flow__edge.selected .react-flow__edge-path {
    stroke: #ffd166 !important;
    stroke-width: 2.5 !important;
    filter: drop-shadow(0 0 4px rgba(255,209,102,0.6));
  }
`;
const nodeTypes = { fb: FunctionBlock, ghost: GhostNode };
const FB_TITLE_H = 40;
const FB_ROW_H = 18;
const GHOST_GAP = 60;
let wsClient = null;
const DND_TYPE = "application/x-ce-component-type";
const ROOT_UID = 0;
const MARQUEE_DRAG_PX = 8;
const TAB_SUFFIX = Math.trunc(performance.now() * 1e3 + performance.timeOrigin).toString(36).slice(-4);
function CeEditor({ base }) {
  return /* @__PURE__ */ jsx(ReactFlowProvider, { children: /* @__PURE__ */ jsx(Inner, { base }) });
}
function Inner({ base }) {
  useMemo(() => setEngineBase(base), [base]);
  const [nodes, setNodes] = useState([]);
  const [edges, setEdges] = useState([]);
  const [pendingEdges, setPendingEdges] = useState(null);
  const exposedRemapRef = useRef(/* @__PURE__ */ new Map());
  const reloadGen = useRef(0);
  const lastAppliedReloadGen = useRef(0);
  const sessionIdRef = useRef(null);
  const POS_SETTLE_PX = 0.5;
  const posAnims = useRef(
    /* @__PURE__ */ new Map()
  );
  const posAnimRaf = useRef(null);
  const posAnimLastTick = useRef(null);
  const tickPosAnims = useCallback(() => {
    const now = performance.now();
    const last = posAnimLastTick.current;
    const dt = last != null ? Math.min(0.05, (now - last) / 1e3) : 1 / 60;
    posAnimLastTick.current = now;
    const anims = posAnims.current;
    if (anims.size === 0) {
      posAnimRaf.current = null;
      posAnimLastTick.current = null;
      return;
    }
    const alpha = 1 - Math.exp(-9 * dt);
    const patch = /* @__PURE__ */ new Map();
    for (const [id, a] of anims) {
      const nx = a.curPos.x + (a.endPos.x - a.curPos.x) * alpha;
      const ny = a.curPos.y + (a.endPos.y - a.curPos.y) * alpha;
      if (Math.abs(a.endPos.x - nx) < POS_SETTLE_PX && Math.abs(a.endPos.y - ny) < POS_SETTLE_PX) {
        patch.set(id, a.endPos);
        anims.delete(id);
      } else {
        a.curPos = { x: nx, y: ny };
        patch.set(id, { x: nx, y: ny });
      }
    }
    if (patch.size > 0) {
      setNodes(
        (ns) => ns.map((n) => {
          const p = patch.get(n.id);
          if (p) return { ...n, position: p };
          if (n.type === "ghost") {
            const g = n;
            const anchor = patch.get(String(g.data.anchorUid));
            if (!anchor) return n;
            const gx = g.data.side === "input" ? anchor.x + NODE_W + GHOST_GAP : anchor.x - g.data.width - GHOST_GAP;
            const gy = anchor.y + FB_TITLE_H + g.data.anchorRowIdx * FB_ROW_H;
            return { ...g, position: { x: gx, y: gy } };
          }
          return n;
        })
      );
    }
    posAnimRaf.current = anims.size > 0 ? requestAnimationFrame(tickPosAnims) : null;
    if (anims.size === 0) posAnimLastTick.current = null;
  }, []);
  const animateNodeTo = useCallback(
    (id, fromPos, toPos) => {
      const existing = posAnims.current.get(id);
      posAnims.current.set(id, {
        curPos: existing ? existing.curPos : fromPos,
        endPos: toPos
      });
      if (posAnimRaf.current == null) {
        posAnimRaf.current = requestAnimationFrame(tickPosAnims);
      }
    },
    [tickPosAnims]
  );
  useEffect(() => {
    return () => {
      if (posAnimRaf.current != null) {
        cancelAnimationFrame(posAnimRaf.current);
        posAnimRaf.current = null;
      }
      posAnims.current.clear();
      posAnimLastTick.current = null;
    };
  }, []);
  const [error, setError] = useState(null);
  const reportError = useCallback((e) => {
    if (e instanceof RestError) setError({ message: e.message, debug: e.debug });
    else setError({ message: e instanceof Error ? e.message : String(e) });
  }, []);
  const [palette, setPalette] = useState([]);
  const [actionsByType, setActionsByType] = useState(
    () => /* @__PURE__ */ new Map()
  );
  const actionTypesRef = useRef(/* @__PURE__ */ new Set());
  const [crumbs, setCrumbs] = useState([{ uid: ROOT_UID, name: "root" }]);
  const currentParentUid = crumbs[crumbs.length - 1].uid;
  const rf = useReactFlow();
  const marquee = useRef(null);
  const [marqueeRect, setMarqueeRect] = useState(null);
  const isPaneTarget = (target) => {
    let el = target;
    while (el) {
      if (el.classList?.contains("react-flow__node")) return false;
      if (el.classList?.contains("react-flow__edge")) return false;
      if (el.classList?.contains("react-flow__handle")) return false;
      if (el.classList?.contains("react-flow__pane")) return true;
      el = el.parentElement;
    }
    return false;
  };
  const onCanvasPointerDown = useCallback((e) => {
    if (e.button === 2 && isPaneTarget(e.target)) {
      marquee.current = { startX: e.clientX, startY: e.clientY, active: false };
    }
  }, []);
  const onCanvasPointerMove = useCallback((e) => {
    const m = marquee.current;
    if (!m) return;
    const dx = e.clientX - m.startX;
    const dy = e.clientY - m.startY;
    if (!m.active && Math.hypot(dx, dy) < MARQUEE_DRAG_PX) return;
    m.active = true;
    setMarqueeRect({
      x: Math.min(m.startX, e.clientX),
      y: Math.min(m.startY, e.clientY),
      w: Math.abs(dx),
      h: Math.abs(dy)
    });
  }, []);
  const onCanvasPointerUp = useCallback(
    (e) => {
      const m = marquee.current;
      marquee.current = null;
      if (!m) return;
      if (!m.active) {
        setMarqueeRect(null);
        setNodeMenu(null);
        setPaneMenu({ x: e.clientX, y: e.clientY });
        return;
      }
      const a = rf.screenToFlowPosition({ x: m.startX, y: m.startY });
      const b = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      const rect = {
        x: Math.min(a.x, b.x),
        y: Math.min(a.y, b.y),
        width: Math.abs(b.x - a.x),
        height: Math.abs(b.y - a.y)
      };
      const partially = e.clientX < m.startX;
      const hits = rf.getIntersectingNodes(rect, partially);
      const hitIds = new Set(hits.filter((n) => n.type !== "ghost").map((n) => n.id));
      const multi = e.shiftKey || e.metaKey || e.ctrlKey;
      setNodes(
        (ns) => ns.map((n) => {
          if (n.type === "ghost") return n;
          const want = multi ? n.selected || hitIds.has(n.id) : hitIds.has(n.id);
          return n.selected === want ? n : { ...n, selected: want };
        })
      );
      if (!multi) setEdges((es) => es.map((ed) => ed.selected ? { ...ed, selected: false } : ed));
      setMarqueeRect(null);
    },
    [rf]
  );
  useEffect(() => {
    const findHit = (target) => {
      let el = target;
      while (el) {
        if (el.classList?.contains("react-flow__node")) {
          const id = el.dataset.id ?? null;
          return id ? { kind: "node", id } : null;
        }
        if (el.classList?.contains("react-flow__edge")) {
          const id = el.dataset.id ?? null;
          return id ? { kind: "edge", id } : null;
        }
        if (el.classList?.contains("react-flow__pane")) return null;
        el = el.parentElement;
      }
      return null;
    };
    const isPane = (target) => {
      let el = target;
      while (el) {
        if (el.classList?.contains("react-flow__pane")) return true;
        if (el.classList?.contains("react-flow__node")) return false;
        if (el.classList?.contains("react-flow__edge")) return false;
        el = el.parentElement;
      }
      return false;
    };
    let downAt = null;
    const onDown = (e) => {
      if (e.button !== 0) {
        downAt = null;
        return;
      }
      downAt = { x: e.clientX, y: e.clientY, hit: findHit(e.target) };
    };
    const onUp = (e) => {
      const d = downAt;
      downAt = null;
      if (!d) return;
      const dist = Math.hypot(e.clientX - d.x, e.clientY - d.y);
      if (dist > 4) return;
      const upHit = findHit(e.target);
      const multi = e.shiftKey || e.metaKey || e.ctrlKey;
      if (d.hit?.kind === "node" && upHit?.kind === "node" && upHit.id === d.hit.id) {
        const id = d.hit.id;
        if (id.startsWith("ghost:")) return;
        metrics.lastSelChange = `click→${useStructural.getState().components.get(Number(id))?.name ?? id} (capture)`;
        metrics.lastSelChangeAt = performance.now();
        setNodes(
          (ns) => ns.map((n) => {
            if (multi) return n.id === id ? { ...n, selected: !n.selected } : n;
            const want = n.id === id;
            return n.selected === want ? n : { ...n, selected: want };
          })
        );
        if (!multi) {
          setEdges((es) => es.map((edge) => edge.selected ? { ...edge, selected: false } : edge));
        }
        return;
      }
      if (d.hit?.kind === "edge" && upHit?.kind === "edge" && upHit.id === d.hit.id) {
        const id = d.hit.id;
        metrics.lastSelChange = `edge→${id}`;
        metrics.lastSelChangeAt = performance.now();
        setEdges(
          (es) => es.map((edge) => {
            if (multi) return edge.id === id ? { ...edge, selected: !edge.selected } : edge;
            const want = edge.id === id;
            return edge.selected === want ? edge : { ...edge, selected: want };
          })
        );
        if (!multi) {
          setNodes((ns) => ns.map((n) => n.selected ? { ...n, selected: false } : n));
        }
        return;
      }
      if (!d.hit && !upHit && isPane(e.target)) {
        metrics.lastSelChange = "pane→clear (capture)";
        metrics.lastSelChangeAt = performance.now();
        setNodes((ns) => ns.map((n) => n.selected ? { ...n, selected: false } : n));
        setEdges((es) => es.map((edge) => edge.selected ? { ...edge, selected: false } : edge));
      }
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("pointerup", onUp, true);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("pointerup", onUp, true);
    };
  }, []);
  useEffect(() => {
    const onKey = (e) => {
      if (e.key !== "Escape") return;
      const ae = document.activeElement;
      if (ae && (ae.tagName === "INPUT" || ae.tagName === "TEXTAREA" || ae.isContentEditable)) {
        return;
      }
      setNodes((ns) => ns.map((n) => n.selected ? { ...n, selected: false } : n));
      setEdges((es) => es.map((edge) => edge.selected ? { ...edge, selected: false } : edge));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  const enter = useCallback((uid) => {
    const c = useStructural.getState().components.get(uid);
    if (!c) return;
    setCrumbs((cur) => [...cur, { uid: c.uid, name: c.name || c.type }]);
  }, []);
  const [focusAfterLoad, setFocusAfterLoad] = useState(null);
  const [findOpen, setFindOpen] = useState(false);
  const [clickDebugOpen, setClickDebugOpen] = useState(() => {
    try {
      return window.localStorage.getItem("ce-ui.clickDebug") === "1";
    } catch {
      return false;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.clickDebug", clickDebugOpen ? "1" : "0");
    } catch {
    }
  }, [clickDebugOpen]);
  const [pendingPasteSelection, setPendingPasteSelection] = useState(null);
  const goToComponent = useCallback(async (uid) => {
    try {
      const targetResp = await getNodeByUid(uid, { depth: 0 });
      const target = targetResp.nodes[0];
      if (!target) return;
      const chain = [];
      let cursor = target.parent;
      while (cursor !== ROOT_UID) {
        const r = await getNodeByUid(cursor, { depth: 0 });
        const c = r.nodes[0];
        if (!c) break;
        chain.unshift({ uid: c.uid, name: c.name || c.type });
        if (c.parent === c.uid) break;
        cursor = c.parent;
      }
      setFocusAfterLoad(uid);
      setCrumbs([{ uid: ROOT_UID, name: "root" }, ...chain]);
    } catch (e) {
      reportError(e);
    }
  }, []);
  const [nodeMenu, setNodeMenu] = useState(
    null
  );
  const [movePickerOpen, setMovePickerOpen] = useState(false);
  const [actionPickerOpen, setActionPickerOpen] = useState(false);
  const [detailsUid, setDetailsUid] = useState(null);
  const [paneMenu, setPaneMenu] = useState(null);
  const openNodeContextMenu = useCallback(
    (uid, x, y) => {
      setNodeMenu({ x, y, uid });
      setPaneMenu(null);
      setMovePickerOpen(false);
      setActionPickerOpen(false);
      setNodes((ns) => {
        const target = ns.find((n) => n.id === String(uid));
        if (target?.selected) return ns;
        return ns.map((n) => {
          const want = n.id === String(uid);
          return n.selected === want ? n : { ...n, selected: want };
        });
      });
    },
    []
  );
  const getActionsFor = useCallback(
    (uids) => {
      const comps = useStructural.getState().components;
      const lists = uids.map((u) => comps.get(u)?.type).filter((t) => !!t).map((t) => actionsByType.get(t) ?? []);
      if (lists.length === 0) return [];
      const [first, ...rest] = lists;
      return first.filter((a) => rest.every((l) => l.some((b) => b.name === a.name)));
    },
    [actionsByType]
  );
  const invokeAction = useCallback(
    (uids, action, params) => Promise.all(uids.map((u) => callAction(u, action, params))),
    []
  );
  const goToCrumb = useCallback((idx) => {
    setCrumbs((cur) => cur.slice(0, idx + 1));
  }, []);
  const copySelectionToClipboard = useCallback(() => {
    const selectedReal = nodes.filter((n) => n.selected && n.type !== "ghost");
    if (selectedReal.length === 0) return;
    const uids = selectedReal.map((n) => Number(n.id));
    const xs = selectedReal.map((n) => n.position.x);
    const ys = selectedReal.map((n) => n.position.y);
    const centroid = {
      x: (Math.min(...xs) + Math.max(...xs)) / 2,
      y: (Math.min(...ys) + Math.max(...ys)) / 2
    };
    clipboardRef.current = { uids, centroid };
    metrics.lastSelChange = `copied ${uids.length}c`;
    metrics.lastSelChangeAt = performance.now();
  }, [nodes]);
  const deleteGhostEdge = useCallback(async (edgeUid) => {
    try {
      await removeEdge(edgeUid);
    } catch (e) {
      reportError(e);
      return;
    }
    useStructural.getState().removeEdge(edgeUid);
    setEdges((es) => es.filter((e) => e.id !== String(edgeUid)));
    setNodes(
      (ns) => ns.flatMap((n) => {
        if (n.type !== "ghost") return [n];
        const g = n;
        const idx = g.data.connections.findIndex((c) => c.edgeUid === edgeUid);
        if (idx < 0) return [n];
        const next = g.data.connections.filter((_, i) => i !== idx);
        if (next.length === 0) return [];
        return [{ ...g, data: { ...g.data, connections: next } }];
      })
    );
  }, []);
  const reload = useCallback(async () => {
    const gen = ++reloadGen.current;
    try {
      let resp;
      if (currentParentUid === ROOT_UID) {
        resp = await getRootNodes({ depth: 1, nested: true, withEdges: true });
      } else {
        resp = await getNodeByUid(currentParentUid, {
          depth: 1,
          nested: true,
          withEdges: true
        });
      }
      if (gen < lastAppliedReloadGen.current) return;
      lastAppliedReloadGen.current = gen;
      const parent = resp.nodes[0];
      const children = parent?.children ?? [];
      const scopedEdges = resp.edges ?? [];
      const childUids = new Set(children.map((c) => c.uid));
      const childByUid = new Map(children.map((c) => [c.uid, c]));
      const { inEdges, crossEdges } = partitionEdges(scopedEdges, childUids);
      useStructural.getState().setNodes(children, inEdges);
      const { index: exposedIndex, remap: exposedRemap, subProps } = exposedPortIndex(children);
      exposedRemapRef.current = exposedRemap;
      wsClient?.setDesiredPropSubscription(subProps);
      const portEdges = [];
      const ghostGroups = /* @__PURE__ */ new Map();
      for (const e of crossEdges) {
        const route = classifyCrossEdge(e, childUids, exposedIndex);
        const style = route.loopBack ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } : { stroke: "#4a9eff", strokeWidth: 1.5 };
        if (route.kind === "port") {
          portEdges.push(
            route.externalIsTarget ? {
              id: String(route.edgeUid),
              source: String(route.visibleUid),
              sourceHandle: String(route.visiblePropUid),
              target: String(route.portParentUid),
              targetHandle: String(route.portHandle),
              style,
              animated: false
            } : {
              id: String(route.edgeUid),
              source: String(route.portParentUid),
              sourceHandle: String(route.portHandle),
              target: String(route.visibleUid),
              targetHandle: String(route.visiblePropUid),
              style,
              animated: false
            }
          );
          continue;
        }
        const visibleComp = childByUid.get(route.visibleUid);
        if (!visibleComp) continue;
        const visibleProp = visibleComp.properties[route.visiblePropName];
        if (!visibleProp) continue;
        const rowIdx = userFacingRowIndex(visibleComp, route.visiblePropName);
        if (rowIdx < 0) continue;
        const key = `${route.visibleUid}:${visibleProp.uid}`;
        let group = ghostGroups.get(key);
        if (!group) {
          group = {
            visibleUid: route.visibleUid,
            visiblePropUid: visibleProp.uid,
            rowIdx,
            side: route.side,
            connections: [],
            edgeUids: [],
            visibleX: visibleComp.metadata?.position?.x ?? 0,
            visibleY: visibleComp.metadata?.position?.y ?? 0
          };
          ghostGroups.set(key, group);
        }
        group.connections.push({
          externalComponentUid: route.externalUid,
          externalPath: route.externalPath,
          externalPropName: route.externalPropName,
          edgeUid: route.edgeUid
        });
        group.edgeUids.push(route.edgeUid);
      }
      const ghostNodes = [];
      const ghostEdges = [];
      for (const g of ghostGroups.values()) {
        const first = g.connections[0];
        const labelPath = stripRoot(first.externalPath);
        const gw = ghostWidthFor(labelPath, first.externalPropName) + (g.connections.length > 1 ? 26 : 0);
        const gx = g.side === "input" ? g.visibleX + NODE_W + GHOST_GAP : g.visibleX - gw - GHOST_GAP;
        const gy = g.visibleY + FB_TITLE_H + g.rowIdx * FB_ROW_H + (FB_ROW_H - GHOST_H) / 2;
        const ghostId = `ghost:${g.visibleUid}:${g.visiblePropUid}`;
        const handleId = `gh:${g.visibleUid}:${g.visiblePropUid}`;
        ghostNodes.push({
          id: ghostId,
          type: "ghost",
          position: { x: gx, y: gy },
          width: gw,
          // selectable: false would strip pointer events on the wrapper in
          // some RF configs, defeating the popover. Keep selectable + harmless;
          // the doc-level click handler skips ghost ids so it doesn't latch
          // selection visually. Still non-draggable.
          draggable: false,
          data: {
            connections: g.connections,
            handleId,
            side: g.side,
            anchorUid: g.visibleUid,
            anchorRowIdx: g.rowIdx,
            width: gw,
            onNavigate: goToComponent,
            onDeleteEdge: deleteGhostEdge
          }
        });
        const visibleHandleId = String(g.visiblePropUid);
        for (const edgeUid of g.edgeUids) {
          const e = crossEdges.find((x) => x.uid === edgeUid);
          const externalIsTarget = g.side === "input";
          ghostEdges.push({
            id: String(edgeUid),
            source: externalIsTarget ? String(g.visibleUid) : ghostId,
            sourceHandle: externalIsTarget ? visibleHandleId : handleId,
            target: externalIsTarget ? ghostId : String(g.visibleUid),
            targetHandle: externalIsTarget ? handleId : visibleHandleId,
            style: e.loopBack === true ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } : { stroke: "#4a9eff", strokeWidth: 1.5 },
            animated: false
          });
        }
      }
      setNodes((prev) => {
        const selectedIds = /* @__PURE__ */ new Set();
        for (const n of prev) if (n.selected) selectedIds.add(n.id);
        const real = buildRfNodes(
          children,
          enter,
          openNodeContextMenu,
          selectedIds,
          actionTypesRef.current
        );
        return [...real, ...ghostNodes];
      });
      setEdges([]);
      setPendingEdges([...buildRfEdges(inEdges, children), ...ghostEdges, ...portEdges]);
    } catch (e) {
      reportError(e);
    }
  }, [currentParentUid, enter, openNodeContextMenu, goToComponent, deleteGhostEdge]);
  const reloadRef = useRef(reload);
  reloadRef.current = reload;
  const pasteFromClipboard = useCallback(async () => {
    const cb = clipboardRef.current;
    if (!cb || cb.uids.length === 0) return;
    try {
      const res = await copyNodes({
        componentUids: cb.uids,
        destParentUid: currentParentUid,
        includeInternalEdges: true
      });
      const clones = res.nodes ?? [];
      if (clones.length === 0) {
        setError({ message: "paste: nothing cloned (sources may have been deleted)" });
        return;
      }
      const cursor = rf.screenToFlowPosition(mouseScreenPos.current);
      const { updates, newUids } = planPaste(clones, currentParentUid, cursor, res.uidMap);
      try {
        if (updates.length > 0) await bulkUpdate(updates);
      } catch (e) {
        console.error("paste: reposition/facet-remap failed:", e.message);
      }
      setPendingPasteSelection(newUids);
      pushUndo({ kind: "delete", componentUids: newUids });
      await reload();
    } catch (e) {
      reportError(e);
    }
  }, [currentParentUid, reload, rf]);
  const undo = useCallback(async () => {
    const pid = currentParentUidRef.current;
    const stack = undoStacksByParent.current.get(pid);
    const entry = stack?.pop();
    if (!entry) return;
    try {
      if (entry.kind === "move") {
        if (entry.updates.length === 1) {
          const u = entry.updates[0];
          await updateNode(u.uid, { position: u.position });
        } else if (entry.updates.length > 1) {
          await bulkUpdate(entry.updates);
        }
      } else if (entry.kind === "delete") {
        await bulkDelete({
          componentUids: entry.componentUids,
          edgeUids: entry.edgeUids
        });
      } else if (entry.kind === "restore") {
        await restoreItems({
          componentUids: entry.componentUids,
          edgeUids: entry.edgeUids
        });
      }
      await reload();
    } catch (e) {
      reportError(e);
    }
  }, [reload]);
  useEffect(() => {
    const onKey = (e) => {
      const cmd = e.metaKey || e.ctrlKey;
      if (cmd && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setFindOpen(true);
        return;
      }
      const ae = document.activeElement;
      if (ae && (ae.tagName === "INPUT" || ae.tagName === "TEXTAREA" || ae.isContentEditable)) {
        return;
      }
      if (!cmd) return;
      const key = e.key.toLowerCase();
      if (key === "c") {
        e.preventDefault();
        copySelectionToClipboard();
      } else if (key === "v") {
        e.preventDefault();
        void pasteFromClipboard();
      } else if (key === "z" && !e.shiftKey) {
        e.preventDefault();
        void undo();
      } else if (key === "d" && e.shiftKey) {
        e.preventDefault();
        setClickDebugOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [copySelectionToClipboard, pasteFromClipboard, undo]);
  useEffect(() => {
    reload();
  }, [reload]);
  useEffect(() => {
    if (focusAfterLoad == null) return;
    const targetId = String(focusAfterLoad);
    const target = nodes.find((n) => n.id === targetId);
    if (!target || target.type === "ghost") return;
    const restComp = useStructural.getState().components.get(focusAfterLoad);
    const userPropCount = restComp ? Object.values(restComp.properties).filter(
      (p) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL
    ).length : 4;
    const FB_TITLE = 40;
    const FB_ROW = 18;
    const estH = FB_TITLE + userPropCount * FB_ROW + 4;
    const cx = target.position.x + NODE_W / 2;
    const cy = target.position.y + estH / 2;
    rf.setCenter(cx, cy, { duration: 400, zoom: rf.getZoom() });
    setNodes(
      (ns) => ns.map((n) => {
        const want = n.id === targetId;
        return n.selected === want ? n : { ...n, selected: want };
      })
    );
    setFocusAfterLoad(null);
  }, [nodes, focusAfterLoad, rf]);
  useEffect(() => {
    if (pendingPasteSelection == null) return;
    const wantedIds = new Set(pendingPasteSelection.map(String));
    let foundCount = 0;
    for (const n of nodes) if (wantedIds.has(n.id)) foundCount++;
    if (foundCount < wantedIds.size) return;
    setNodes(
      (ns) => ns.map((n) => {
        if (n.type === "ghost") return n;
        const want = wantedIds.has(n.id);
        return n.selected === want ? n : { ...n, selected: want };
      })
    );
    setEdges((es) => es.map((e) => e.selected ? { ...e, selected: false } : e));
    setPendingPasteSelection(null);
  }, [nodes, pendingPasteSelection]);
  const readyKey = useStore$1((s) => {
    if (!pendingEdges) return "";
    const lookup = s.nodeLookup;
    if (!lookup) return "";
    const ids = [];
    for (const e of pendingEdges) {
      const src = lookup.get(e.source);
      const dst = lookup.get(e.target);
      const srcBounds = src?.internals?.handleBounds?.source;
      const dstBounds = dst?.internals?.handleBounds?.target;
      if (!srcBounds || !dstBounds) continue;
      if (!srcBounds.some((h) => h.id === e.sourceHandle)) continue;
      if (!dstBounds.some((h) => h.id === e.targetHandle)) continue;
      ids.push(e.id);
    }
    return ids.join(",");
  });
  useEffect(() => {
    if (!pendingEdges) return;
    const ready = new Set(readyKey ? readyKey.split(",") : []);
    setEdges(pendingEdges.filter((e) => ready.has(e.id)));
    if (ready.size === pendingEdges.length) setPendingEdges(null);
  }, [readyKey, pendingEdges]);
  useEffect(() => {
    if (!pendingEdges) return;
    const t = window.setTimeout(() => setPendingEdges(null), 1500);
    return () => window.clearTimeout(t);
  }, [pendingEdges]);
  useEffect(() => {
    fetch(`${base}/api/v0/schema`).then((r) => r.json()).then((j) => {
      const exts = j.data;
      const seen = /* @__PURE__ */ new Map();
      const actions = /* @__PURE__ */ new Map();
      for (const e of exts) {
        const id = `${e.vendor}-${e.name}`;
        let group = seen.get(id);
        if (!group) {
          group = { id, vendor: e.vendor, name: e.name, version: e.version, components: [] };
          seen.set(id, group);
        }
        const have = new Set(group.components.map((c) => c.type));
        for (const c of e.components ?? []) {
          const type = `${id}::${c.name}`;
          if (c.actions && c.actions.length > 0 && !actions.has(type)) {
            actions.set(type, c.actions);
          }
          if (have.has(type)) continue;
          have.add(type);
          group.components.push({ name: c.name, type, icon: c.icon });
        }
      }
      setActionsByType(actions);
      actionTypesRef.current = new Set(actions.keys());
      const list = [...seen.values()].filter((g) => g.components.length > 0);
      list.sort((a, b) => a.id.localeCompare(b.id));
      for (const g of list) g.components.sort((a, b) => a.name.localeCompare(b.name));
      setPalette(list);
    }).catch(() => {
    });
  }, []);
  useEffect(() => {
    const comps = useStructural.getState().components;
    setNodes((ns) => {
      let changed = false;
      const next = ns.map((n) => {
        if (n.type !== "fb") return n;
        const fb = n;
        const t = comps.get(Number(fb.id))?.type;
        const has = t ? actionsByType.has(t) : false;
        if (fb.data.hasActions === has) return n;
        changed = true;
        return { ...fb, data: { ...fb.data, hasActions: has } };
      });
      return changed ? next : ns;
    });
  }, [actionsByType]);
  useEffect(() => {
    startDiagnostics();
    startDiagReporter(1e3);
    return () => {
      stopDiagReporter();
      stopDiagnostics();
    };
  }, []);
  const graphSubsRef = useRef(/* @__PURE__ */ new Set());
  const tableSubsRef = useRef(/* @__PURE__ */ new Set());
  const flushSubs = useCallback(() => {
    const union = /* @__PURE__ */ new Set([...graphSubsRef.current, ...tableSubsRef.current]);
    wsClient?.setDesiredSubscription(union);
    diagGauges.subscribedComponents = union.size;
  }, []);
  const onVisibleSubscription = useCallback(
    (uids) => {
      graphSubsRef.current = uids;
      flushSubs();
    },
    [flushSubs]
  );
  const onTableRows = useCallback(
    (uids) => {
      tableSubsRef.current = new Set(uids);
      flushSubs();
    },
    [flushSubs]
  );
  const rateThrottle = useRef({
    timer: null,
    pending: null
  });
  const wsAdapter = useMemo(
    () => ({
      setRate: (hz) => {
        const t = rateThrottle.current;
        if (t.timer != null) {
          t.pending = hz;
          return;
        }
        wsClient?.setRate(hz);
        t.pending = null;
        t.timer = window.setTimeout(() => {
          t.timer = null;
          if (t.pending != null) {
            wsClient?.setRate(t.pending);
            t.pending = null;
          }
        }, 200);
      },
      getRate: () => wsClient?.getRate() ?? null
    }),
    []
  );
  const [autoRate, setAutoRate] = useState(() => {
    try {
      return window.localStorage.getItem("ce-ui.autoRate") === "1";
    } catch {
      return false;
    }
  });
  const [manualRate, setManualRate] = useState(() => {
    try {
      const v = Number(window.localStorage.getItem("ce-ui.manualRate"));
      return Number.isFinite(v) && v >= 1 ? v : 10;
    } catch {
      return 10;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.autoRate", autoRate ? "1" : "0");
      window.localStorage.setItem("ce-ui.manualRate", String(manualRate));
    } catch {
    }
  }, [autoRate, manualRate]);
  const onSetManualRate = useCallback((hz) => {
    setManualRate(hz);
    wsClient?.setRate(hz);
  }, []);
  useEffect(() => {
    if (!autoRate) wsClient?.setRate(manualRate);
  }, [autoRate, manualRate]);
  const userName = useMemo(() => {
    let base2 = "user";
    try {
      base2 = window.localStorage.getItem("ce-ui.userName") || "user";
    } catch {
    }
    return `${base2}-${TAB_SUFFIX}`;
  }, []);
  const selectedUidsKey = nodes.filter((n) => n.selected && n.type !== "ghost").map((n) => n.id).join(",");
  const selKeyRef = useRef(selectedUidsKey);
  selKeyRef.current = selectedUidsKey;
  const publishPresence = useCallback(() => {
    const key = selKeyRef.current;
    wsClient?.publishPresence({
      userName,
      selectedComponents: key ? key.split(",").map(Number) : [],
      parentUid: currentParentUid
    });
  }, [userName, currentParentUid]);
  useEffect(() => {
    const t = window.setTimeout(publishPresence, 150);
    return () => window.clearTimeout(t);
  }, [selectedUidsKey, currentParentUid, userName, publishPresence]);
  useEffect(() => {
    const HEARTBEAT_MS = 2e4;
    const SWEEP_MS = 8e3;
    const PRESENCE_TTL_MS = 5e4;
    const hb = window.setInterval(publishPresence, HEARTBEAT_MS);
    const sw = window.setInterval(() => usePresence.getState().sweep(PRESENCE_TTL_MS), SWEEP_MS);
    return () => {
      window.clearInterval(hb);
      window.clearInterval(sw);
    };
  }, [publishPresence]);
  const totalComponentCount = useStructural((s) => s.components.size);
  diagGauges.visibleNodes = nodes.filter((n) => n.type !== "ghost").length;
  diagGauges.ghostNodes = nodes.filter((n) => n.type === "ghost").length;
  diagGauges.edges = edges.length;
  diagGauges.totalComponents = totalComponentCount;
  diagGauges.wsConnected = metrics.wsConnected;
  diagGauges.reconnects = metrics.reconnectCount;
  diagGauges.lastSeq = metrics.lastSeq;
  useEffect(() => {
    if (wsClient) return;
    const ws = new CeRestWs(wsUrlFromBase(base), {
      onSchema: (msg) => {
        loadSchemaIndices(msg.properties);
        setRestSessionId(msg.sessionId);
        sessionIdRef.current = msg.sessionId;
      },
      onFrame: (frame) => {
        for (const s of frame.sections) {
          if (s.typeTag === TYPE_STATUS) {
            useStatusFlags.getState().applyStatus(s.uids, s.values);
          } else {
            useValues.getState().apply(s.uids, s.values);
          }
        }
      },
      onTopology: (msg) => {
        if (msg.type === "topologyAdded") {
          const st = useStructural.getState();
          const haveAll = msg.components.every((c) => st.components.has(c.uid)) && msg.edges.every((e) => st.edges.has(e.uid));
          if (haveAll) return;
          scheduleTopologyReload();
        } else if (msg.type === "topologyRemoved") {
          const dropC = new Set(msg.componentUids.map(String));
          const dropE = new Set(msg.edgeUids.map(String));
          setNodes((ns) => ns.filter((n) => !dropC.has(n.id)));
          setEdges((es) => es.filter((e) => !dropE.has(e.id)));
          for (const uid of msg.componentUids) {
            useStructural.getState().removeComponent(uid);
          }
          for (const uid of msg.edgeUids) {
            useStructural.getState().removeEdge(uid);
          }
        } else if (msg.type === "topologyChanged") {
          const shapeChanged = msg.components.some(
            (c) => c.addedProperties && c.addedProperties.length > 0 || c.removedProperties && c.removedProperties.length > 0 || c.parent !== void 0
          );
          if (shapeChanged) {
            scheduleTopologyReload();
            return;
          }
          const isOwnEcho = msg.originSessionId === sessionIdRef.current;
          const patches = /* @__PURE__ */ new Map();
          for (const p of msg.components) {
            const id = String(p.uid);
            if (isOwnEcho && draggingNodes.current.has(id) && p.position && !p.name) {
              continue;
            }
            patches.set(id, { position: p.position, name: p.name });
          }
          if (patches.size === 0) return;
          setNodes(
            (ns) => ns.map((n) => {
              if (n.type === "ghost") return n;
              const fb = n;
              const p = patches.get(fb.id);
              if (!p) return n;
              const newPos = p.position ?? fb.position;
              const newName = p.name ?? fb.data.name;
              const samePos = newPos === fb.position;
              const sameName = newName === fb.data.name;
              if (samePos && sameName) return n;
              if (!samePos && !isOwnEcho && p.position) {
                animateNodeTo(fb.id, fb.position, p.position);
                return sameName ? n : { ...fb, data: { ...fb.data, name: newName } };
              }
              posAnims.current.delete(fb.id);
              return {
                ...fb,
                position: samePos ? fb.position : newPos,
                data: sameName ? fb.data : { ...fb.data, name: newName }
              };
            })
          );
        }
      },
      onPresence: (m) => {
        usePresence.getState().upsert(m.sessionId, m.state ?? {});
      },
      onPresenceSnapshot: (m) => {
        usePresence.getState().replaceAll(
          (m.presences ?? []).map((p) => ({
            sessionId: p.sessionId,
            state: p.state ?? {}
          }))
        );
      },
      onPresenceLeft: (m) => {
        usePresence.getState().remove(m.sessionId);
      },
      onOpen: () => {
      },
      onClose: () => {
        usePresence.getState().reset();
      }
    });
    ws.connect();
    wsClient = ws;
  }, []);
  const topoTimer = useRef(null);
  const scheduleTopologyReload = useCallback(() => {
    if (topoTimer.current != null) return;
    topoTimer.current = window.setTimeout(() => {
      topoTimer.current = null;
      reloadRef.current();
    }, 0);
  }, []);
  const mouseScreenPos = useRef({
    x: window.innerWidth / 2,
    y: window.innerHeight / 2
  });
  useEffect(() => {
    const onMove = (e) => {
      mouseScreenPos.current = { x: e.clientX, y: e.clientY };
    };
    window.addEventListener("mousemove", onMove);
    return () => window.removeEventListener("mousemove", onMove);
  }, []);
  const clipboardRef = useRef(null);
  const undoStacksByParent = useRef(/* @__PURE__ */ new Map());
  const UNDO_MAX = 50;
  const currentParentUidRef = useRef(currentParentUid);
  useEffect(() => {
    currentParentUidRef.current = currentParentUid;
  }, [currentParentUid]);
  const pushUndo = useCallback((entry) => {
    const pid = currentParentUidRef.current;
    const m = undoStacksByParent.current;
    let stack = m.get(pid);
    if (!stack) {
      stack = [];
      m.set(pid, stack);
    }
    stack.push(entry);
    if (stack.length > UNDO_MAX) stack.shift();
  }, []);
  const draggingNodes = useRef(/* @__PURE__ */ new Set());
  const DRAG_PATCH_MS = 100;
  const dragPatchState = useRef({ lastSent: 0, pending: /* @__PURE__ */ new Map(), timer: null });
  const flushDragPatch = useCallback(() => {
    const s = dragPatchState.current;
    s.timer = null;
    if (s.pending.size === 0) return;
    s.lastSent = performance.now();
    const updates = [...s.pending.entries()].map(([uid, p]) => ({
      uid,
      position: { x: Math.round(p.x), y: Math.round(p.y) }
    }));
    s.pending.clear();
    if (updates.length === 1) {
      const u = updates[0];
      updateNode(u.uid, { position: u.position }).catch(() => {
      });
    } else {
      bulkUpdate(updates).catch(() => {
      });
    }
  }, []);
  const sendDragPatch = useCallback(
    (uid, pos) => {
      const s = dragPatchState.current;
      s.pending.set(uid, pos);
      const now = performance.now();
      if (now - s.lastSent >= DRAG_PATCH_MS) {
        flushDragPatch();
        return;
      }
      if (s.timer == null) {
        s.timer = window.setTimeout(flushDragPatch, DRAG_PATCH_MS - (now - s.lastSent));
      }
    },
    [flushDragPatch]
  );
  const cancelDragPatch = useCallback((id) => {
    const s = dragPatchState.current;
    s.pending.delete(Number(id));
    if (s.timer != null && s.pending.size === 0) {
      window.clearTimeout(s.timer);
      s.timer = null;
    }
  }, []);
  const onEdgesChange = useCallback((changes) => {
    setEdges((es) => applyEdgeChanges(changes.filter((c) => c.type !== "select"), es));
  }, []);
  const [edgeMenu, setEdgeMenu] = useState(
    null
  );
  const onEdgeContextMenu = useCallback((e, edge) => {
    e.preventDefault();
    e.stopPropagation();
    setEdgeMenu({ x: e.clientX, y: e.clientY, edgeId: edge.id });
    setEdges(
      (es) => es.map((ed) => ed.id === edge.id ? ed.selected ? ed : { ...ed, selected: true } : ed)
    );
  }, []);
  const reEvaluateEdges = useCallback(async (ids) => {
    const results = await Promise.allSettled(
      ids.map((uid) => updateEdge(uid, { reEvaluate: true }))
    );
    const failed = results.find((r) => r.status === "rejected");
    if (failed) reportError(failed.reason);
  }, []);
  const setEdgesLoopBack = useCallback(async (ids) => {
    if (ids.length === 0) return;
    const results = await Promise.allSettled(
      ids.map((uid) => updateEdge(uid, { loopBack: true }))
    );
    const failed = results.find((r) => r.status === "rejected");
    if (failed) reportError(failed.reason);
    const ok = ids.filter((_, i) => results[i].status === "fulfilled");
    if (ok.length === 0) return;
    const okSet = new Set(ok.map(String));
    const st = useStructural.getState();
    for (const uid of ok) {
      const e = st.edges.get(uid);
      if (e) st.upsertEdge({ ...e, loopBack: true });
    }
    setEdges(
      (es) => es.map(
        (e) => okSet.has(e.id) ? { ...e, style: { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } } : e
      )
    );
  }, []);
  const onNodesChange = useCallback((changes) => {
    setNodes((ns) => {
      const next = applyNodeChanges(changes.filter((c) => c.type !== "select"), ns);
      const movedAnchors = /* @__PURE__ */ new Map();
      for (const ch of changes) {
        if (ch.type !== "position" || !ch.position) continue;
        const n = next.find((m) => m.id === ch.id);
        if (!n || n.type === "ghost") continue;
        movedAnchors.set(ch.id, ch.position);
      }
      if (movedAnchors.size === 0) return next;
      return next.map((n) => {
        if (n.type !== "ghost") return n;
        const g = n;
        const anchor = movedAnchors.get(String(g.data.anchorUid));
        if (!anchor) return n;
        const gx = g.data.side === "input" ? anchor.x + NODE_W + GHOST_GAP : anchor.x - g.data.width - GHOST_GAP;
        const gy = anchor.y + FB_TITLE_H + g.data.anchorRowIdx * FB_ROW_H;
        return { ...g, position: { x: gx, y: gy } };
      });
    });
    const selChanges = changes.filter((c) => c.type === "select");
    if (selChanges.length > 0) {
      const comps = useStructural.getState().components;
      const compact = selChanges.map((c) => {
        const id = c.id;
        const sel = c.selected;
        const name = comps.get(Number(id))?.name ?? id;
        return `${name}=${sel ? "+" : "-"}`;
      }).join(" ");
      metrics.lastSelChange = compact;
      metrics.lastSelChangeAt = performance.now();
    }
    for (const ch of changes) {
      if (ch.type === "position" && ch.dragging) {
        posAnims.current.delete(ch.id);
      }
    }
  }, []);
  const dragStartPositions = useRef(
    []
  );
  const onNodeDragStart = useCallback(
    (_e, _node, ns) => {
      const real = ns.filter((n) => n.type !== "ghost");
      for (const n of real) draggingNodes.current.add(n.id);
      dragStartPositions.current = real.map((n) => ({
        uid: Number(n.id),
        position: { x: Math.round(n.position.x), y: Math.round(n.position.y) }
      }));
    },
    []
  );
  const onNodeDrag = useCallback(
    (_e, _node, ns) => {
      for (const n of ns) {
        if (n.type === "ghost") continue;
        sendDragPatch(Number(n.id), n.position);
      }
    },
    [sendDragPatch]
  );
  const onNodeDragStop = useCallback(
    (_e, _node, ns) => {
      const real = ns.filter((n) => n.type !== "ghost");
      for (const n of real) {
        draggingNodes.current.delete(n.id);
        cancelDragPatch(n.id);
      }
      if (real.length === 0) return;
      const updates = real.map((n) => ({
        uid: Number(n.id),
        position: { x: Math.round(n.position.x), y: Math.round(n.position.y) }
      }));
      const starts = dragStartPositions.current;
      if (starts.length > 0) {
        const moved = starts.filter((s) => {
          const u = updates.find((x) => x.uid === s.uid);
          return u && (u.position.x !== s.position.x || u.position.y !== s.position.y);
        });
        if (moved.length > 0) pushUndo({ kind: "move", updates: moved });
      }
      dragStartPositions.current = [];
      if (updates.length === 1) {
        const u = updates[0];
        updateNode(u.uid, { position: u.position }).catch(
          (e) => reportError(e)
        );
      } else {
        bulkUpdate(updates).catch((e) => reportError(e));
      }
    },
    [cancelDragPatch, pushUndo]
  );
  const onConnect = useCallback(async (c) => {
    if (!c.source || !c.target || !c.sourceHandle || !c.targetHandle) return;
    try {
      const remap = exposedRemapRef.current;
      const srcUid = remap.get(Number(c.sourceHandle)) ?? Number(c.source);
      const tgtUid = remap.get(Number(c.targetHandle)) ?? Number(c.target);
      const created = await addEdge({
        sourceUid: srcUid,
        sourcePropUid: Number(c.sourceHandle),
        targetUid: tgtUid,
        targetPropUid: Number(c.targetHandle)
      });
      if (created?.uid != null) {
        useStructural.getState().upsertEdge({
          ...created,
          sourceUid: srcUid,
          sourcePropertyUid: Number(c.sourceHandle),
          targetUid: tgtUid,
          targetPropertyUid: Number(c.targetHandle)
        });
        const isLoop = created.loopBack === true;
        const rfEdge = {
          id: String(created.uid),
          source: c.source,
          sourceHandle: c.sourceHandle,
          target: c.target,
          targetHandle: c.targetHandle,
          style: isLoop ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } : { stroke: "#4a9eff", strokeWidth: 1.5 },
          animated: false
        };
        setEdges((es) => es.some((e) => e.id === rfEdge.id) ? es : [...es, rfEdge]);
      } else {
        await reload();
      }
    } catch (e) {
      reportError(e);
    }
  }, [reload]);
  const onNodesDelete = useCallback(
    async (ns) => {
      const real = ns.filter((n) => n.type !== "ghost");
      if (real.length === 0) return;
      const uids = real.map((n) => Number(n.id));
      try {
        if (uids.length === 1) {
          await removeNode(uids[0]);
        } else {
          await bulkDelete({ componentUids: uids });
        }
        for (const uid of uids) useStructural.getState().removeComponent(uid);
        pushUndo({ kind: "restore", componentUids: uids });
      } catch (e) {
        reportError(e);
      }
      await reload();
    },
    [reload, pushUndo]
  );
  const onEdgesDelete = useCallback(
    async (es) => {
      if (es.length === 0) return;
      const uids = es.map((e) => Number(e.id));
      try {
        if (uids.length === 1) {
          await removeEdge(uids[0]);
        } else {
          await bulkDelete({ edgeUids: uids });
        }
        for (const uid of uids) useStructural.getState().removeEdge(uid);
        pushUndo({ kind: "restore", edgeUids: uids });
      } catch (err) {
        reportError(err);
      }
      setEdges((cur) => cur.filter((e) => !es.find((d) => d.id === e.id)));
    },
    [pushUndo]
  );
  const onAddNode = useCallback(
    async (type, worldPos) => {
      const vp = rf.getViewport();
      const pos = worldPos ?? {
        x: Math.round((window.innerWidth / 2 - vp.x) / vp.zoom),
        y: Math.round((window.innerHeight / 2 - vp.y) / vp.zoom)
      };
      {
        const STACK_OFFSET = 16;
        const occupied = (x, y) => rf.getNodes().some(
          (n) => n.type === "fb" && Math.round(n.position.x) === x && Math.round(n.position.y) === y
        );
        let guard = 0;
        while (occupied(Math.round(pos.x), Math.round(pos.y)) && guard < 200) {
          pos.x += STACK_OFFSET;
          pos.y += STACK_OFFSET;
          guard += 1;
        }
        pos.x = Math.round(pos.x);
        pos.y = Math.round(pos.y);
      }
      const base2 = sanitizeName(type);
      const siblings = new Set(
        Array.from(useStructural.getState().components.values()).filter((c) => c.parent === currentParentUid).map((c) => c.name)
      );
      const name = uniqueName(base2, siblings);
      try {
        const created = await addNode({
          type,
          name,
          parentUid: currentParentUid,
          defaultValues: { position: { x: Math.round(pos.x), y: Math.round(pos.y) } }
        });
        if (created?.uid != null) {
          pushUndo({ kind: "delete", componentUids: [created.uid] });
          useStructural.getState().upsertComponent(created);
          const [rfNode] = buildRfNodes(
            [created],
            enter,
            openNodeContextMenu,
            void 0,
            actionTypesRef.current
          );
          if (rfNode) {
            setNodes((ns) => ns.some((n) => n.id === rfNode.id) ? ns : [...ns, rfNode]);
          }
        } else {
          await reload();
        }
      } catch (e) {
        reportError(e);
      }
    },
    [rf, reload, currentParentUid, pushUndo, enter, openNodeContextMenu]
  );
  const componentTypes = useMemo(
    () => palette.flatMap(
      (g) => g.components.map((c) => ({ name: c.name, type: c.type, group: g.id }))
    ),
    [palette]
  );
  const createComponent = useCallback(
    async (type, opts) => {
      const baseName = sanitizeName(type);
      const siblings = new Set(
        Array.from(useStructural.getState().components.values()).filter((c) => c.parent === currentParentUid).map((c) => c.name)
      );
      const name = uniqueName(baseName, siblings);
      const near = opts?.nearUid != null ? useStructural.getState().components.get(opts.nearUid) : void 0;
      let pos;
      if (near?.metadata?.position) {
        const GAP = 80;
        const dx = (NODE_W + GAP) * (opts?.side === "left" ? -1 : 1);
        pos = { x: (near.metadata.position.x ?? 0) + dx, y: near.metadata.position.y ?? 0 };
      } else {
        const vp = rf.getViewport();
        pos = {
          x: Math.round((window.innerWidth / 2 - vp.x) / vp.zoom),
          y: Math.round((window.innerHeight / 2 - vp.y) / vp.zoom)
        };
      }
      try {
        const created = await addNode({
          type,
          name,
          parentUid: currentParentUid,
          defaultValues: { position: { x: Math.round(pos.x), y: Math.round(pos.y) } }
        });
        if (created?.uid != null) {
          pushUndo({ kind: "delete", componentUids: [created.uid] });
          useStructural.getState().upsertComponent(created);
          const [rfNode] = buildRfNodes(
            [created],
            enter,
            openNodeContextMenu,
            void 0,
            actionTypesRef.current
          );
          if (rfNode) setNodes((ns) => ns.some((n) => n.id === rfNode.id) ? ns : [...ns, rfNode]);
        }
        return created ?? null;
      } catch (e) {
        reportError(e);
        return null;
      }
    },
    [rf, currentParentUid, pushUndo, enter, openNodeContextMenu]
  );
  const connectEdge = useCallback(
    async (payload) => {
      const created = await addEdge(payload);
      if (created?.uid == null) return;
      useStructural.getState().upsertEdge({
        ...created,
        sourceUid: payload.sourceUid,
        sourcePropertyUid: payload.sourcePropUid,
        targetUid: payload.targetUid,
        targetPropertyUid: payload.targetPropUid
      });
      const st = useStructural.getState();
      const inView = st.components.has(payload.sourceUid) && st.components.has(payload.targetUid);
      if (inView) {
        const isLoop = created.loopBack === true;
        const rfEdge = {
          id: String(created.uid),
          source: String(payload.sourceUid),
          sourceHandle: String(payload.sourcePropUid),
          target: String(payload.targetUid),
          targetHandle: String(payload.targetPropUid),
          style: isLoop ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } : { stroke: "#4a9eff", strokeWidth: 1.5 },
          animated: false
        };
        setEdges((es) => es.some((e) => e.id === rfEdge.id) ? es : [...es, rfEdge]);
      } else {
        await reload();
      }
    },
    [reload]
  );
  const exposeProp = useCallback(
    async (childPropUid, childComponentUid, side, defaultLabel) => {
      const parentUid = currentParentUid;
      try {
        const resp = await getNodeByUid(parentUid, { depth: 0 });
        const parent = resp.nodes[0];
        const facet = parseFacet(rawFacet(parent?.properties) ?? "");
        const child = useStructural.getState().components.get(childComponentUid);
        const facetPropUid = child?.properties?.[FACET_PROP]?.uid;
        const existing = facet.get(childPropUid) ?? {};
        facet.set(childPropUid, {
          ...existing,
          expose: side,
          childComponent: childComponentUid,
          facetProp: facetPropUid,
          // Fallback display name only — live label/unit/aliases come from the
          // child's streamed __facets.
          label: existing.label ?? defaultLabel
        });
        await updateNode(parentUid, {
          properties: { [FACET_PROP]: { value: serializeFacet(facet) } }
        });
        await reload();
      } catch (e) {
        reportError(e);
      }
    },
    [currentParentUid, reload]
  );
  const unexposeProp = useCallback(
    async (folderUid, childPropUid) => {
      try {
        const resp = await getNodeByUid(folderUid, { depth: 0 });
        const folder = resp.nodes[0];
        const facet = parseFacet(rawFacet(folder?.properties) ?? "");
        facet.delete(childPropUid);
        await updateNode(folderUid, {
          properties: { [FACET_PROP]: { value: serializeFacet(facet) } }
        });
        await reload();
      } catch (e) {
        reportError(e);
      }
    },
    [reload, reportError]
  );
  const groupSelected = useCallback(
    async (uids) => {
      if (uids.length < 2) return;
      const group = new Set(uids);
      const comps = useStructural.getState().components;
      const edges2 = useStructural.getState().edges;
      const boundary = groupBoundary(group, edges2.values(), comps);
      const xs = [];
      const ys = [];
      for (const node of rf.getNodes()) {
        if (group.has(Number(node.id))) {
          xs.push(node.position.x);
          ys.push(node.position.y);
        }
      }
      const position = xs.length ? {
        x: Math.round((Math.min(...xs) + Math.max(...xs)) / 2),
        y: Math.round((Math.min(...ys) + Math.max(...ys)) / 2)
      } : { x: 0, y: 0 };
      const siblings = new Set(
        Array.from(comps.values()).filter((c) => c.parent === currentParentUid).map((c) => c.name)
      );
      const name = uniqueName("group", siblings);
      try {
        const folder = await addNode({
          type: "core-extRoot::Folder",
          name,
          parentUid: currentParentUid,
          defaultValues: { position }
        });
        if (folder?.uid == null) return;
        await bulkUpdate(uids.map((uid) => ({ uid, parentUid: folder.uid })));
        if (boundary.size > 0) {
          const facet = /* @__PURE__ */ new Map();
          for (const [propUid, b] of boundary) {
            facet.set(propUid, {
              expose: b.side,
              childComponent: b.childComponent,
              facetProp: b.facetProp,
              label: b.label
            });
          }
          await updateNode(folder.uid, {
            properties: { [FACET_PROP]: { value: serializeFacet(facet) } }
          });
        }
        await reload();
      } catch (e) {
        reportError(e);
      }
    },
    [currentParentUid, reload, reportError, rf]
  );
  const openDetails = useCallback(async (componentUid) => {
    if (!useStructural.getState().components.has(componentUid)) {
      try {
        const resp = await getNodeByUid(componentUid, { depth: 0 });
        const c = resp.nodes[0];
        if (c) useStructural.getState().upsertComponent(c);
      } catch {
      }
    }
    setDetailsUid(componentUid);
  }, []);
  const ceCtx = useMemo(
    () => ({
      componentTypes,
      createComponent,
      connectEdge,
      exposeProp,
      unexposeProp,
      openDetails,
      requestReload: scheduleTopologyReload,
      parentName: crumbs.length > 1 ? crumbs[crumbs.length - 1]?.name : void 0
    }),
    [
      componentTypes,
      createComponent,
      connectEdge,
      exposeProp,
      unexposeProp,
      openDetails,
      scheduleTopologyReload,
      crumbs
    ]
  );
  const onDragOver = useCallback((e) => {
    if (e.dataTransfer.types.includes(DND_TYPE)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }, []);
  const onDrop = useCallback(
    (e) => {
      const type = e.dataTransfer.getData(DND_TYPE);
      if (!type) return;
      e.preventDefault();
      const worldPos = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
      onAddNode(type, worldPos);
    },
    [rf, onAddNode]
  );
  const [tableOpen, setTableOpen] = useState(false);
  const [splitPct, setSplitPct] = useState(55);
  const splitRestore = useRef(55);
  const tableMaxed = splitPct <= 12;
  const onTableSelect = useCallback((uid, additive) => {
    setNodes(
      (ns) => ns.map((n) => {
        if (n.id === String(uid)) return n.selected ? n : { ...n, selected: true };
        if (additive) return n;
        return n.selected ? { ...n, selected: false } : n;
      })
    );
  }, []);
  const startSplitDrag = useCallback((e) => {
    e.preventDefault();
    const move = (ev) => {
      const pct = ev.clientX / window.innerWidth * 100;
      setSplitPct(Math.min(90, Math.max(10, pct)));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }, []);
  const tableSelected = nodes.filter((n) => n.selected).map((n) => Number(n.id));
  return /* @__PURE__ */ jsxs(CeWiresheetContext.Provider, { value: ceCtx, children: [
    /* @__PURE__ */ jsx("style", { children: EDGE_SELECTED_CSS }),
    /* @__PURE__ */ jsxs("div", { style: { position: "absolute", inset: 0, display: "flex", flexDirection: "column" }, children: [
      /* @__PURE__ */ jsxs("div", { style: { flex: 1, minHeight: 0, display: "flex" }, children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              position: "relative",
              height: "100%",
              width: tableOpen ? `${splitPct}%` : "100%",
              flexShrink: 0,
              transform: "translateZ(0)",
              overflow: "hidden"
            },
            children: [
              /* @__PURE__ */ jsx(
                "div",
                {
                  style: { position: "absolute", inset: 0 },
                  onDragOver,
                  onDrop,
                  onPointerDown: onCanvasPointerDown,
                  onPointerMove: onCanvasPointerMove,
                  onPointerUp: onCanvasPointerUp,
                  onContextMenu: (e) => {
                    e.preventDefault();
                  },
                  children: /* @__PURE__ */ jsxs(
                    index,
                    {
                      nodes,
                      edges,
                      nodeTypes,
                      onNodesChange,
                      onEdgesChange,
                      onEdgeContextMenu,
                      onNodeDragStart,
                      onNodeDrag,
                      onNodeDragStop,
                      onConnect,
                      onNodesDelete,
                      onEdgesDelete,
                      defaultViewport: { x: 80, y: 80, zoom: 1 },
                      minZoom: 0.1,
                      maxZoom: 2,
                      onlyRenderVisibleElements: true,
                      nodeOrigin: [0, 0],
                      deleteKeyCode: ["Delete", "Backspace"],
                      panOnDrag: [0],
                      selectionMode: SelectionMode.Partial,
                      multiSelectionKeyCode: ["Shift", "Meta", "Control"],
                      selectionKeyCode: null,
                      nodeDragThreshold: 4,
                      panOnScroll: false,
                      panOnScrollMode: PanOnScrollMode.Free,
                      proOptions: { hideAttribution: true },
                      children: [
                        /* @__PURE__ */ jsx(Background, { color: "#1f242e", gap: 20 }),
                        /* @__PURE__ */ jsx(
                          MiniMap,
                          {
                            position: "bottom-right",
                            pannable: true,
                            zoomable: true,
                            ariaLabel: "Graph overview",
                            style: {
                              backgroundColor: "#2b313c",
                              border: "1px solid #3d444d",
                              borderRadius: 6
                            },
                            maskColor: "rgba(10,12,16,0.45)",
                            nodeStrokeWidth: 2,
                            nodeColor: miniMapNodeColor,
                            nodeStrokeColor: miniMapNodeStroke
                          }
                        ),
                        /* @__PURE__ */ jsx(ZoomRateController, { enabled: autoRate, setRate: wsAdapter.setRate }),
                        /* @__PURE__ */ jsx(VisibilitySub, { onVisible: onVisibleSubscription })
                      ]
                    }
                  )
                }
              ),
              /* @__PURE__ */ jsx(
                LeftDock,
                {
                  palette,
                  onAdd: (t) => onAddNode(t),
                  currentParentUid,
                  onPick: (uid) => void goToComponent(uid)
                }
              ),
              clickDebugOpen && /* @__PURE__ */ jsx(ClickDebugger, {}),
              /* @__PURE__ */ jsx(EventsPanel, {}),
              /* @__PURE__ */ jsx(
                DiagPanel,
                {
                  wsRef: wsAdapter,
                  autoRate,
                  manualRate,
                  onSetManualRate,
                  onToggleAutoRate: () => setAutoRate((v) => !v)
                }
              ),
              /* @__PURE__ */ jsx(PresenceBar, {}),
              /* @__PURE__ */ jsx(
                FindPanel,
                {
                  open: findOpen,
                  currentParentUid,
                  onClose: () => setFindOpen(false),
                  onPick: (uid) => void goToComponent(uid)
                }
              ),
              marqueeRect && /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    position: "fixed",
                    left: marqueeRect.x,
                    top: marqueeRect.y,
                    width: marqueeRect.w,
                    height: marqueeRect.h,
                    border: "1px solid #4a9eff",
                    background: "rgba(74,158,255,0.12)",
                    zIndex: 40,
                    pointerEvents: "none"
                  }
                }
              ),
              nodeMenu && !movePickerOpen && !actionPickerOpen && detailsUid === null && /* @__PURE__ */ jsx(
                NodeContextMenu,
                {
                  x: nodeMenu.x,
                  y: nodeMenu.y,
                  hasActions: getActionsFor(nodes.filter((n) => n.selected).map((n) => Number(n.id))).length > 0,
                  canRename: nodes.filter((n) => n.selected).length === 1,
                  count: nodes.filter((n) => n.selected).length,
                  uid: nodes.filter((n) => n.selected).length === 1 ? Number(nodes.filter((n) => n.selected)[0].id) : void 0,
                  name: nodes.filter((n) => n.selected).length === 1 ? useStructural.getState().components.get(Number(nodes.filter((n) => n.selected)[0].id))?.name : void 0,
                  onRename: async () => {
                    const sel = nodes.filter((n) => n.selected).map((n) => Number(n.id));
                    setNodeMenu(null);
                    if (sel.length !== 1) return;
                    const uid = sel[0];
                    const cur = useStructural.getState().components.get(uid);
                    const next = window.prompt("Rename component", cur?.name ?? "");
                    if (next == null) return;
                    const trimmed = next.trim();
                    if (!trimmed || trimmed === cur?.name) return;
                    try {
                      await updateNode(uid, { name: trimmed });
                      await reload();
                    } catch (e) {
                      reportError(e);
                    }
                  },
                  onDetails: () => {
                    const sel = nodes.filter((n) => n.selected).map((n) => Number(n.id));
                    if (sel.length === 1) setDetailsUid(sel[0]);
                  },
                  onGroup: () => {
                    void groupSelected(nodes.filter((n) => n.selected).map((n) => Number(n.id)));
                    setNodeMenu(null);
                  },
                  onMoveInto: () => setMovePickerOpen(true),
                  onAction: () => setActionPickerOpen(true),
                  onClose: () => setNodeMenu(null)
                }
              ),
              nodeMenu && actionPickerOpen && /* @__PURE__ */ jsx(
                ActionPicker,
                {
                  x: nodeMenu.x,
                  y: nodeMenu.y,
                  targetUids: nodes.filter((n) => n.selected).map((n) => Number(n.id)),
                  actions: getActionsFor(nodes.filter((n) => n.selected).map((n) => Number(n.id))),
                  onInvoke: invokeAction,
                  onClose: () => {
                    setActionPickerOpen(false);
                    setNodeMenu(null);
                  }
                }
              ),
              nodeMenu && movePickerOpen && /* @__PURE__ */ jsx(
                MoveIntoPicker,
                {
                  x: nodeMenu.x,
                  y: nodeMenu.y,
                  movingUids: nodes.filter((n) => n.selected).map((n) => Number(n.id)),
                  onMove: async (newParent) => {
                    const moving = nodes.filter((n) => n.selected).map((n) => Number(n.id));
                    for (const uid of moving) {
                      try {
                        await updateNode(uid, { parentUid: newParent });
                      } catch (e) {
                        reportError(e);
                      }
                    }
                    setMovePickerOpen(false);
                    setNodeMenu(null);
                    await reload();
                  },
                  onClose: () => {
                    setMovePickerOpen(false);
                    setNodeMenu(null);
                  }
                }
              ),
              detailsUid != null && /* @__PURE__ */ jsx(
                ConfigurePanel,
                {
                  componentUid: detailsUid,
                  currentParentUid,
                  exposeProp,
                  unexposeProp,
                  onSave: async (facetString) => {
                    try {
                      await updateNode(detailsUid, {
                        properties: { [FACET_PROP]: { value: facetString } }
                      });
                      await reload();
                    } catch (e) {
                      reportError(e);
                    }
                  },
                  onClose: () => {
                    setDetailsUid(null);
                    setNodeMenu(null);
                  }
                }
              ),
              paneMenu && /* @__PURE__ */ jsx(
                PaneContextMenu,
                {
                  x: paneMenu.x,
                  y: paneMenu.y,
                  canGoUp: crumbs.length > 1,
                  parentName: crumbs.length > 1 ? crumbs[crumbs.length - 2].name : "",
                  palette,
                  canPaste: (clipboardRef.current?.uids.length ?? 0) > 0,
                  onUp: () => goToCrumb(crumbs.length - 2),
                  onAdd: (type) => void onAddNode(type, rf.screenToFlowPosition({ x: paneMenu.x, y: paneMenu.y })),
                  onPaste: () => {
                    mouseScreenPos.current = { x: paneMenu.x, y: paneMenu.y };
                    void pasteFromClipboard();
                  },
                  onClose: () => setPaneMenu(null)
                }
              ),
              edgeMenu && (() => {
                const rest = useStructural.getState().edges.get(Number(edgeMenu.edgeId));
                if (!rest) return null;
                const isLoop = rest.loopBack === true;
                return /* @__PURE__ */ jsx(
                  EdgeContextMenu,
                  {
                    x: edgeMenu.x,
                    y: edgeMenu.y,
                    isLoopBack: isLoop,
                    onPrimary: () => {
                      const ids = selectedEdgeIds(edges, edgeMenu.edgeId);
                      const filtered = ids.filter((id) => {
                        const e = useStructural.getState().edges.get(Number(id));
                        return e ? e.loopBack === true === isLoop : false;
                      });
                      if (isLoop) void reEvaluateEdges(filtered.map(Number));
                      else void setEdgesLoopBack(filtered.map(Number));
                      setEdgeMenu(null);
                    },
                    onDelete: () => {
                      const ids = selectedEdgeIds(edges, edgeMenu.edgeId);
                      const drop = edges.filter((e) => ids.includes(e.id));
                      void onEdgesDelete(drop);
                      setEdgeMenu(null);
                    },
                    onClose: () => setEdgeMenu(null)
                  }
                );
              })(),
              error && /* @__PURE__ */ jsx(ErrorBanner, { error, onClose: () => setError(null) }),
              !tableOpen && /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setTableOpen(true),
                  title: "Open table view",
                  style: {
                    position: "absolute",
                    top: "50%",
                    right: 0,
                    transform: "translateY(-50%)",
                    zIndex: 20,
                    width: 26,
                    height: 70,
                    background: "rgba(20,23,30,0.92)",
                    border: "1px solid #2c313c",
                    borderRight: "none",
                    borderRadius: "6px 0 0 6px",
                    color: "#cbd3e0",
                    cursor: "pointer",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center"
                  },
                  children: /* @__PURE__ */ jsx(Table2, { size: 16 })
                }
              )
            ]
          }
        ),
        tableOpen && /* @__PURE__ */ jsx(
          "div",
          {
            onPointerDown: startSplitDrag,
            title: "Drag to resize",
            style: { width: 5, flexShrink: 0, cursor: "col-resize", background: "#2c313c" }
          }
        ),
        tableOpen && /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              flex: 1,
              minWidth: 0,
              height: "100%",
              display: "flex",
              flexDirection: "column"
            },
            children: [
              /* @__PURE__ */ jsxs(
                "div",
                {
                  style: {
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                    padding: "3px 6px",
                    background: "#1a1d24",
                    borderBottom: "1px solid #2c313c",
                    flexShrink: 0
                  },
                  children: [
                    /* @__PURE__ */ jsx("span", { style: { fontSize: 10, color: "#5a6172", marginRight: "auto" }, children: "table view" }),
                    /* @__PURE__ */ jsx(
                      "button",
                      {
                        title: tableMaxed ? "Restore split" : "Maximize table",
                        onClick: () => {
                          if (tableMaxed) setSplitPct(splitRestore.current || 55);
                          else {
                            splitRestore.current = splitPct;
                            setSplitPct(8);
                          }
                        },
                        style: tableChromeBtn,
                        children: tableMaxed ? /* @__PURE__ */ jsx(Minimize2, { size: 14 }) : /* @__PURE__ */ jsx(Maximize2, { size: 14 })
                      }
                    ),
                    /* @__PURE__ */ jsx("button", { title: "Close table", onClick: () => setTableOpen(false), style: tableChromeBtn, children: /* @__PURE__ */ jsx(X, { size: 14 }) })
                  ]
                }
              ),
              /* @__PURE__ */ jsx("div", { style: { flex: 1, minHeight: 0 }, children: /* @__PURE__ */ jsx(
                ComponentTable,
                {
                  currentParentUid,
                  selectedUids: tableSelected,
                  onSelectRow: onTableSelect,
                  onDrillIn: enter,
                  onRowsChange: onTableRows
                }
              ) })
            ]
          }
        )
      ] }),
      /* @__PURE__ */ jsx(
        "div",
        {
          style: {
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "4px 10px",
            background: "#1a1d24",
            borderTop: "1px solid #2c313c",
            flexShrink: 0,
            zIndex: 30,
            color: "#e6e8eb"
          },
          children: /* @__PURE__ */ jsx(Breadcrumb, { crumbs, onGoTo: goToCrumb })
        }
      )
    ] })
  ] });
}
function selectedEdgeIds(edges, rightClickedId) {
  const sel = edges.filter((e) => e.selected).map((e) => e.id);
  return sel.length > 1 && sel.includes(rightClickedId) ? sel : [rightClickedId];
}
function EdgeContextMenu({
  x,
  y,
  isLoopBack,
  onPrimary,
  onDelete,
  onClose
}) {
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-edge-menu]")) return;
      onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  const primaryLabel = isLoopBack ? "Reevaluate" : "Set as loopback";
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ce-edge-menu": true,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        left: x,
        top: y,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        padding: 4,
        minWidth: 160,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif"
      },
      children: [
        /* @__PURE__ */ jsx(EdgeMenuItem, { label: primaryLabel, onClick: onPrimary }),
        /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Delete", onClick: onDelete, danger: true })
      ]
    }
  );
}
function EdgeMenuItem({
  label,
  onClick,
  danger
}) {
  const [hover, setHover] = useState(false);
  return /* @__PURE__ */ jsx(
    "button",
    {
      onClick,
      onMouseEnter: () => setHover(true),
      onMouseLeave: () => setHover(false),
      style: {
        width: "100%",
        textAlign: "left",
        background: hover ? danger ? "#3a1a1a" : "#232733" : "transparent",
        color: danger ? "#ffb8b8" : "#e6e8eb",
        border: "none",
        padding: "6px 10px",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: 12,
        borderRadius: 3
      },
      children: label
    }
  );
}
const tableChromeBtn = {
  display: "flex",
  alignItems: "center",
  background: "transparent",
  border: "none",
  color: "#8892a0",
  cursor: "pointer",
  padding: "2px 4px"
};
const detailsField = {
  background: "#0f1115",
  color: "#e6e8eb",
  border: "1px solid #2c313c",
  borderRadius: 2,
  padding: "2px 5px",
  fontSize: 11,
  fontFamily: "ui-monospace, SFMono-Regular, monospace",
  boxSizing: "border-box",
  outline: "none",
  minWidth: 0
};
function ConfigurePanel({
  componentUid,
  currentParentUid,
  exposeProp,
  unexposeProp,
  onSave,
  onClose
}) {
  const comp = useStructural((s) => s.components.get(componentUid));
  const props = useMemo(() => {
    if (!comp) return [];
    return Object.entries(comp.properties).filter(([, p]) => (p.systemRole ?? ROLE_NORMAL) === ROLE_NORMAL).map(([name, p]) => ({ uid: p.uid, name, category: p.category }));
  }, [comp]);
  const initial = useMemo(
    () => facetFor(componentUid, rawFacet(comp?.properties)),
    [comp, componentUid]
  );
  const portRows = useMemo(() => {
    const own = new Set(props.map((p) => p.uid));
    const out = [];
    for (const [uid, f] of initial) {
      if (f.expose && !own.has(uid)) {
        out.push({ uid, name: f.label ?? `port ${uid}`, side: f.expose });
      }
    }
    return out;
  }, [initial, props]);
  const empty = { label: "", unit: "", decimals: "", hidden: false, aliases: "" };
  const seed = (uid) => {
    const f = initial.get(uid);
    return {
      label: f?.label ?? "",
      unit: f?.unit ?? "",
      decimals: f?.decimals != null ? String(f.decimals) : "",
      hidden: f?.hidden ?? false,
      aliases: f?.aliases?.map((a) => `${a.code}=${a.label}`).join(", ") ?? ""
    };
  };
  const [draft, setDraft] = useState(() => {
    const d = {};
    for (const p of props) d[p.uid] = seed(p.uid);
    for (const pr of portRows) d[pr.uid] = seed(pr.uid);
    return d;
  });
  const set = (uid, patch) => setDraft((d) => ({ ...d, [uid]: { ...d[uid] ?? empty, ...patch } }));
  const canExposeHere = comp != null && comp.parent === currentParentUid && currentParentUid !== ROOT_UID;
  const [exposedOnParent, setExposedOnParent] = useState(() => /* @__PURE__ */ new Set());
  useEffect(() => {
    if (!canExposeHere) return;
    let cancelled = false;
    void getNodeByUid(currentParentUid, { depth: 0 }).then((resp) => {
      if (cancelled) return;
      const pf = parseFacet(rawFacet(resp.nodes[0]?.properties) ?? "");
      const s = /* @__PURE__ */ new Set();
      for (const [uid, f] of pf) if (f.expose != null) s.add(uid);
      setExposedOnParent(s);
    }).catch(() => {
    });
    return () => {
      cancelled = true;
    };
  }, [canExposeHere, currentParentUid, componentUid]);
  const [portInfo, setPortInfo] = useState(
    () => /* @__PURE__ */ new Map()
  );
  useEffect(() => {
    if (portRows.length === 0) return;
    let cancelled = false;
    void getNodeByUid(componentUid, { depth: 1, nested: true }).then((resp) => {
      if (cancelled) return;
      const children = resp.nodes[0]?.children ?? [];
      const byComp = new Map(children.map((c) => [c.uid, c]));
      const m = /* @__PURE__ */ new Map();
      for (const [uid, f] of initial) {
        if (!f.expose || f.childComponent == null) continue;
        const child = byComp.get(f.childComponent);
        if (!child) continue;
        const propName = Object.entries(child.properties).find(([, p]) => p.uid === uid)?.[0];
        m.set(uid, { comp: child.name, prop: propName ?? String(uid) });
      }
      setPortInfo(m);
    }).catch(() => {
    });
    return () => {
      cancelled = true;
    };
  }, [componentUid, initial, portRows.length]);
  const toggleExpose = (p) => {
    const side = p.category === CATEGORY_INPUT ? "input" : "output";
    const next = new Set(exposedOnParent);
    if (next.has(p.uid)) {
      next.delete(p.uid);
      setExposedOnParent(next);
      void unexposeProp(currentParentUid, p.uid);
    } else {
      next.add(p.uid);
      setExposedOnParent(next);
      void exposeProp(p.uid, componentUid, side, draft[p.uid]?.label || p.name);
    }
  };
  const applyCosmetic = (f, d) => {
    if (d.label.trim()) f.label = d.label.trim();
    else delete f.label;
    if (d.unit.trim()) f.unit = d.unit.trim();
    else delete f.unit;
    const dec = Number(d.decimals);
    if (d.decimals.trim() !== "" && Number.isFinite(dec)) f.decimals = dec;
    else delete f.decimals;
    if (d.hidden) f.hidden = true;
    else delete f.hidden;
    const aliases = parseAliasInput(d.aliases);
    if (aliases.length) f.aliases = aliases;
    else delete f.aliases;
  };
  const save = () => {
    const facet = /* @__PURE__ */ new Map();
    for (const [uid, f] of initial) facet.set(uid, { ...f });
    for (const p of props) {
      const f = { ...facet.get(p.uid) ?? {} };
      applyCosmetic(f, draft[p.uid] ?? empty);
      if (Object.keys(f).length > 0) facet.set(p.uid, f);
      else facet.delete(p.uid);
    }
    for (const pr of portRows) {
      const f = { ...facet.get(pr.uid) ?? {} };
      applyCosmetic(f, draft[pr.uid] ?? empty);
      facet.set(pr.uid, f);
    }
    onSave(serializeFacet(facet));
    onClose();
  };
  const onFieldKey = (e) => {
    e.stopPropagation();
    if (e.key === "Enter") {
      e.preventDefault();
      save();
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };
  useEffect(() => {
    const onEsc = (e) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onEsc);
    return () => document.removeEventListener("keydown", onEsc);
  }, [onClose]);
  const cosmeticFields = (uid) => {
    const d = draft[uid] ?? empty;
    return /* @__PURE__ */ jsxs(Fragment, { children: [
      /* @__PURE__ */ jsxs(
        "div",
        {
          style: {
            display: "grid",
            gridTemplateColumns: "1fr 64px 46px auto",
            gap: 6,
            alignItems: "center"
          },
          children: [
            /* @__PURE__ */ jsx(
              "input",
              {
                placeholder: "label",
                value: d.label,
                onChange: (e) => set(uid, { label: e.target.value }),
                onKeyDown: onFieldKey,
                style: detailsField
              }
            ),
            /* @__PURE__ */ jsx(
              "input",
              {
                placeholder: "unit",
                value: d.unit,
                onChange: (e) => set(uid, { unit: e.target.value }),
                onKeyDown: onFieldKey,
                style: detailsField
              }
            ),
            /* @__PURE__ */ jsx(
              "input",
              {
                placeholder: "dec",
                value: d.decimals,
                onChange: (e) => set(uid, { decimals: e.target.value }),
                onKeyDown: onFieldKey,
                style: detailsField
              }
            ),
            /* @__PURE__ */ jsxs("label", { style: { display: "flex", alignItems: "center", gap: 4, color: "#8892a0" }, children: [
              /* @__PURE__ */ jsx(
                "input",
                {
                  type: "checkbox",
                  checked: d.hidden,
                  onChange: (e) => set(uid, { hidden: e.target.checked })
                }
              ),
              "hide"
            ] })
          ]
        }
      ),
      /* @__PURE__ */ jsx(
        "input",
        {
          placeholder: "aliases   e.g.  0=off, 1=auto, 2=manual",
          value: d.aliases,
          onChange: (e) => set(uid, { aliases: e.target.value }),
          onKeyDown: onFieldKey,
          style: { ...detailsField, width: "100%", marginTop: 6 }
        }
      )
    ] });
  };
  return /* @__PURE__ */ jsx(
    "div",
    {
      onClick: onClose,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        inset: 0,
        zIndex: 200,
        background: "rgba(0,0,0,0.45)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center"
      },
      children: /* @__PURE__ */ jsxs(
        "div",
        {
          onClick: (e) => e.stopPropagation(),
          style: {
            width: 480,
            maxHeight: "80vh",
            background: "#1a1d24",
            border: "1px solid #2c313c",
            borderRadius: 6,
            boxShadow: "0 8px 28px rgba(0,0,0,0.6)",
            display: "flex",
            flexDirection: "column",
            color: "#e6e8eb",
            fontFamily: "-apple-system, system-ui, sans-serif",
            fontSize: 12
          },
          children: [
            /* @__PURE__ */ jsxs(
              "div",
              {
                style: {
                  padding: "8px 12px",
                  borderBottom: "1px solid #2c313c",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between"
                },
                children: [
                  /* @__PURE__ */ jsxs("span", { style: { fontWeight: 600 }, children: [
                    "Configure — ",
                    /* @__PURE__ */ jsx("span", { style: { color: "#9ecbff" }, children: comp?.name ?? componentUid })
                  ] }),
                  /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 10 }, children: "label · unit · decimals · aliases" })
                ]
              }
            ),
            /* @__PURE__ */ jsxs("div", { style: { overflowY: "auto" }, children: [
              props.length === 0 && portRows.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "12px", color: "#5a6172" }, children: "no editable properties" }) : props.map((p) => {
                const canExpose = canExposeHere && p.category !== CATEGORY_CONFIG;
                return /* @__PURE__ */ jsxs("div", { style: { borderBottom: "1px solid #232733", padding: "8px 12px" }, children: [
                  /* @__PURE__ */ jsxs(
                    "div",
                    {
                      style: {
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        marginBottom: 5
                      },
                      children: [
                        /* @__PURE__ */ jsx(
                          "span",
                          {
                            style: {
                              color: "#9ecbff",
                              fontFamily: "ui-monospace, SFMono-Regular, monospace"
                            },
                            children: p.name
                          }
                        ),
                        canExpose && /* @__PURE__ */ jsxs(
                          "label",
                          {
                            title: `Expose this ${p.category === CATEGORY_INPUT ? "input" : "output"} as a port on the parent folder`,
                            style: { display: "flex", alignItems: "center", gap: 4, color: "#8892a0" },
                            children: [
                              /* @__PURE__ */ jsx(
                                "input",
                                {
                                  type: "checkbox",
                                  checked: exposedOnParent.has(p.uid),
                                  onChange: () => toggleExpose(p)
                                }
                              ),
                              "expose"
                            ]
                          }
                        )
                      ]
                    }
                  ),
                  cosmeticFields(p.uid)
                ] }, p.uid);
              }),
              portRows.length > 0 && /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    padding: "6px 12px",
                    color: "#5a6172",
                    fontSize: 10,
                    textTransform: "uppercase",
                    letterSpacing: 0.5,
                    borderBottom: "1px solid #232733",
                    background: "#15181e"
                  },
                  children: "exposed ports"
                }
              ),
              portRows.map((pr) => {
                const info = portInfo.get(pr.uid);
                return /* @__PURE__ */ jsxs("div", { style: { borderBottom: "1px solid #232733", padding: "8px 12px" }, children: [
                  /* @__PURE__ */ jsxs(
                    "div",
                    {
                      style: {
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "space-between",
                        marginBottom: 5
                      },
                      children: [
                        /* @__PURE__ */ jsxs(
                          "span",
                          {
                            style: {
                              display: "flex",
                              alignItems: "center",
                              gap: 4,
                              color: "#9ecbff",
                              fontFamily: "ui-monospace, SFMono-Regular, monospace"
                            },
                            children: [
                              /* @__PURE__ */ jsx(CornerDownRight, { size: 12, strokeWidth: 2 }),
                              info ? `${info.comp} · ${info.prop}` : pr.name,
                              " ",
                              /* @__PURE__ */ jsxs("span", { style: { color: "#5a6172" }, children: [
                                "(",
                                pr.side,
                                ")"
                              ] })
                            ]
                          }
                        ),
                        /* @__PURE__ */ jsxs(
                          "label",
                          {
                            title: "Un-expose this port",
                            style: { display: "flex", alignItems: "center", gap: 4, color: "#8892a0" },
                            children: [
                              /* @__PURE__ */ jsx(
                                "input",
                                {
                                  type: "checkbox",
                                  checked: true,
                                  onChange: () => void unexposeProp(componentUid, pr.uid)
                                }
                              ),
                              "exposed"
                            ]
                          }
                        )
                      ]
                    }
                  ),
                  cosmeticFields(pr.uid)
                ] }, pr.uid);
              })
            ] }),
            /* @__PURE__ */ jsxs(
              "div",
              {
                style: {
                  padding: "8px 12px",
                  borderTop: "1px solid #2c313c",
                  display: "flex",
                  justifyContent: "flex-end",
                  gap: 8
                },
                children: [
                  /* @__PURE__ */ jsx(
                    "button",
                    {
                      onClick: onClose,
                      style: {
                        background: "transparent",
                        color: "#9aa3b2",
                        border: "1px solid #2c313c",
                        borderRadius: 3,
                        padding: "4px 12px",
                        cursor: "pointer",
                        fontSize: 12
                      },
                      children: "Cancel"
                    }
                  ),
                  /* @__PURE__ */ jsx(
                    "button",
                    {
                      onClick: save,
                      style: {
                        background: "#2c3a55",
                        color: "#9ecbff",
                        border: "1px solid #3b5388",
                        borderRadius: 3,
                        padding: "4px 14px",
                        cursor: "pointer",
                        fontSize: 12
                      },
                      children: "Save"
                    }
                  )
                ]
              }
            )
          ]
        }
      )
    }
  );
}
function PaneContextMenu({
  x,
  y,
  canGoUp,
  parentName,
  palette,
  canPaste,
  onUp,
  onAdd,
  onPaste,
  onClose
}) {
  const [adding, setAdding] = useState(false);
  const [filter, setFilter] = useState("");
  const [highlight, setHighlight] = useState(0);
  const hlRef = useRef(null);
  useEffect(() => {
    setHighlight(0);
  }, [filter, adding]);
  useEffect(() => {
    hlRef.current?.scrollIntoView({ block: "nearest" });
  }, [highlight]);
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    const onEsc = (e) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
      document.removeEventListener("keydown", onEsc);
    };
  }, [onClose]);
  const W = adding ? 240 : 180;
  const left = Math.min(x, window.innerWidth - W - 8);
  const top = Math.min(y, window.innerHeight - (adding ? 320 : 140));
  const all = palette.flatMap(
    (g) => g.components.map((c) => ({ name: c.name, type: c.type, group: g.id }))
  );
  const f = filter.trim().toLowerCase();
  const filtered = f ? all.filter((c) => c.name.toLowerCase().includes(f) || c.type.toLowerCase().includes(f)) : all;
  return /* @__PURE__ */ jsx(
    "div",
    {
      "data-ce-node-menu": true,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        left,
        top,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        width: W,
        maxHeight: adding ? 320 : void 0,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden"
      },
      children: adding ? /* @__PURE__ */ jsxs(Fragment, { children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              padding: "6px 8px",
              borderBottom: "1px solid #2c313c",
              display: "flex",
              alignItems: "center",
              gap: 6
            },
            children: [
              /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => setAdding(false),
                  title: "Back",
                  style: {
                    background: "transparent",
                    border: "none",
                    color: "#9ecbff",
                    cursor: "pointer",
                    fontSize: 14,
                    padding: 0
                  },
                  children: "‹"
                }
              ),
              /* @__PURE__ */ jsx(
                "input",
                {
                  autoFocus: true,
                  value: filter,
                  onChange: (e) => setFilter(e.target.value),
                  onKeyDown: (e) => {
                    if (e.key === "Escape") {
                      onClose();
                      return;
                    }
                    if (e.key === "ArrowDown") {
                      e.preventDefault();
                      setHighlight((h) => Math.min(h + 1, Math.max(0, filtered.length - 1)));
                      return;
                    }
                    if (e.key === "ArrowUp") {
                      e.preventDefault();
                      setHighlight((h) => Math.max(0, h - 1));
                      return;
                    }
                    if (e.key === "Enter") {
                      e.preventDefault();
                      const c = filtered[highlight];
                      if (c) {
                        onAdd(c.type);
                        onClose();
                      }
                      return;
                    }
                    e.stopPropagation();
                  },
                  placeholder: "Filter components…",
                  style: { ...acInput, flex: 1 }
                }
              )
            ]
          }
        ),
        /* @__PURE__ */ jsx("div", { style: { overflowY: "auto", padding: 4 }, children: filtered.length === 0 ? /* @__PURE__ */ jsx("div", { style: { color: "#5a6172", padding: "6px 8px" }, children: "no matches" }) : filtered.map((c, i) => /* @__PURE__ */ jsxs(
          "button",
          {
            ref: i === highlight ? hlRef : void 0,
            onMouseEnter: () => setHighlight(i),
            onClick: () => {
              onAdd(c.type);
              onClose();
            },
            style: { ...acBtn, background: i === highlight ? "#2c3a55" : "transparent" },
            children: [
              /* @__PURE__ */ jsx("span", { children: c.name }),
              /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 10 }, children: c.group })
            ]
          },
          c.type
        )) })
      ] }) : /* @__PURE__ */ jsxs("div", { style: { padding: 4 }, children: [
        canGoUp && /* @__PURE__ */ jsx(
          EdgeMenuItem,
          {
            label: `‹ Up to ${parentName}`,
            onClick: () => {
              onUp();
              onClose();
            }
          }
        ),
        /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Add component…", onClick: () => setAdding(true) }),
        canPaste && /* @__PURE__ */ jsx(
          EdgeMenuItem,
          {
            label: "Paste",
            onClick: () => {
              onPaste();
              onClose();
            }
          }
        )
      ] })
    }
  );
}
function NodeContextMenu({
  x,
  y,
  hasActions,
  canRename,
  name,
  uid,
  count,
  onRename,
  onDetails,
  onGroup,
  onMoveInto,
  onAction,
  onClose
}) {
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ce-node-menu": true,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        left: x,
        top: y,
        zIndex: 100,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        padding: 4,
        minWidth: 160,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: { padding: "4px 8px", color: "#8892a0", borderBottom: "1px solid #2c313c", marginBottom: 4 },
            children: [
              uid != null ? name || "component" : `${count} components`,
              /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    fontSize: 9,
                    color: "#5a6172",
                    fontFamily: "ui-monospace, SFMono-Regular, monospace",
                    marginTop: 2
                  },
                  children: uid != null ? /* @__PURE__ */ jsx(CopyUid, { label: "comp", value: uid }) : `${count} selected`
                }
              )
            ]
          }
        ),
        canRename && /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Rename…", onClick: onRename }),
        canRename && /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Configure…", onClick: onDetails }),
        count >= 2 && /* @__PURE__ */ jsx(EdgeMenuItem, { label: `Group ${count} into folder`, onClick: onGroup }),
        /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Move into…", onClick: onMoveInto }),
        hasActions && /* @__PURE__ */ jsx(EdgeMenuItem, { label: "Action…", onClick: onAction })
      ]
    }
  );
}
function MoveIntoPicker({
  x,
  y,
  movingUids,
  onMove,
  onClose
}) {
  const [filter, setFilter] = useState("");
  const [allComponents, setAllComponents] = useState(null);
  const movingSet = new Set(movingUids);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        const flat = [];
        const walk = (c) => {
          flat.push(c);
          c.children?.forEach(walk);
        };
        resp.nodes.forEach(walk);
        setAllComponents(flat);
      } catch {
        if (cancelled) return;
        setAllComponents([...useStructural.getState().components.values()]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
  const candidates = moveCandidates(allComponents ?? [], movingSet);
  const visible = filterMoveCandidates(candidates, filter);
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
    };
  }, [onClose]);
  const PICKER_W = 260;
  const left = Math.min(x, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 320);
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ce-node-menu": true,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        left,
        top,
        zIndex: 101,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        width: PICKER_W,
        maxHeight: 320,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column"
      },
      children: [
        /* @__PURE__ */ jsxs("div", { style: { padding: "6px 8px", borderBottom: "1px solid #2c313c" }, children: [
          /* @__PURE__ */ jsxs("div", { style: { color: "#8892a0", fontSize: 10, marginBottom: 4 }, children: [
            "Move ",
            movingUids.length === 1 ? "1 component" : `${movingUids.length} components`,
            " into…"
          ] }),
          /* @__PURE__ */ jsx(
            "input",
            {
              autoFocus: true,
              value: filter,
              onChange: (e) => setFilter(e.target.value),
              onKeyDown: (e) => {
                if (e.key === "Escape") onClose();
                else if (e.key === "Enter" && visible.length === 1) onMove(visible[0].uid);
                e.stopPropagation();
              },
              placeholder: "filter…",
              style: {
                width: "100%",
                background: "#0f1115",
                color: "#e6e8eb",
                border: "1px solid #2c313c",
                borderRadius: 2,
                padding: "3px 6px",
                fontSize: 12,
                fontFamily: "ui-monospace, SFMono-Regular, monospace",
                boxSizing: "border-box",
                outline: "none"
              }
            }
          )
        ] }),
        /* @__PURE__ */ jsx("div", { style: { flex: 1, overflowY: "auto" }, children: visible.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 8px", color: "#5a6172", fontSize: 12 }, children: allComponents == null ? "loading…" : "no destinations" }) : visible.map((c, idx) => {
          const pathLabel = c.path === "root" ? "root" : c.path.startsWith("root/") ? c.path.slice(5) : c.path;
          const showSection = c.tier !== (idx > 0 ? visible[idx - 1].tier : -1);
          const sectionLabel = c.tier === 0 ? "up one level" : c.tier === 1 ? "same level" : c.tier === 2 ? "inside this folder" : "other";
          return /* @__PURE__ */ jsxs("div", { children: [
            showSection && /* @__PURE__ */ jsx(
              "div",
              {
                style: {
                  padding: "6px 8px 2px 8px",
                  color: "#5a6172",
                  fontSize: 9,
                  textTransform: "uppercase",
                  letterSpacing: 0.4,
                  borderTop: idx > 0 ? "1px solid #2c313c" : "none",
                  marginTop: idx > 0 ? 2 : 0
                },
                children: sectionLabel
              }
            ),
            /* @__PURE__ */ jsxs(
              "button",
              {
                onClick: () => onMove(c.uid),
                style: {
                  display: "flex",
                  width: "100%",
                  textAlign: "left",
                  padding: "5px 8px",
                  background: "transparent",
                  color: "#e6e8eb",
                  border: "none",
                  cursor: "pointer",
                  fontSize: 12,
                  fontFamily: "ui-monospace, SFMono-Regular, monospace",
                  alignItems: "baseline",
                  gap: 6
                },
                onMouseEnter: (e) => e.currentTarget.style.background = "#2c313c",
                onMouseLeave: (e) => e.currentTarget.style.background = "transparent",
                children: [
                  /* @__PURE__ */ jsx(
                    "span",
                    {
                      style: {
                        color: "#9ecbff",
                        flex: 1,
                        minWidth: 0,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap"
                      },
                      title: c.path,
                      children: pathLabel
                    }
                  ),
                  /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 11, flexShrink: 0 }, children: c.kind })
                ]
              }
            )
          ] }, c.uid);
        }) })
      ]
    }
  );
}
const acInput = {
  background: "#0f1115",
  border: "1px solid #2c313c",
  borderRadius: 4,
  color: "#e6e8eb",
  fontSize: 12,
  padding: "4px 6px",
  fontFamily: "inherit"
};
const acBtn = {
  width: "100%",
  textAlign: "left",
  background: "transparent",
  color: "#e6e8eb",
  border: "none",
  borderRadius: 4,
  padding: "6px 8px",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: 12,
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  gap: 6
};
const acBtnPrimary = {
  width: "100%",
  background: "#2d6cdf",
  color: "#fff",
  border: "none",
  borderRadius: 4,
  padding: "7px 8px",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: 12,
  marginTop: 6
};
const acRow = { display: "flex", justifyContent: "space-between", padding: "2px 0" };
function actionKind(type) {
  const t = type.toLowerCase();
  if (t === "bool" || t === "boolean") return "bool";
  if (/^(u?int\d*|[iuf]\d+|float|double|number)$/.test(t)) return "num";
  return "str";
}
function defaultForType(type) {
  const k = actionKind(type);
  return k === "bool" ? false : k === "num" ? 0 : "";
}
function coerceParam(type, raw) {
  const k = actionKind(type);
  if (k === "num") {
    const n = Number(raw);
    return Number.isFinite(n) ? n : 0;
  }
  if (k === "bool") return raw === "true" || raw === "1";
  return raw;
}
function ParamField({
  def,
  value,
  onChange
}) {
  const kind = actionKind(def.type);
  return /* @__PURE__ */ jsxs("label", { style: { display: "flex", flexDirection: "column", gap: 3, margin: "0 0 8px" }, children: [
    /* @__PURE__ */ jsxs("span", { style: { color: "#8892a0", fontSize: 10 }, children: [
      def.label ?? def.name,
      /* @__PURE__ */ jsxs("span", { style: { color: "#5a6172" }, children: [
        " · ",
        def.type
      ] })
    ] }),
    def.enum ? /* @__PURE__ */ jsx(
      "select",
      {
        value: String(value ?? ""),
        onChange: (e) => onChange(coerceParam(def.type, e.target.value)),
        style: acInput,
        children: def.enum.map((opt) => /* @__PURE__ */ jsx("option", { value: String(opt), children: String(opt) }, String(opt)))
      }
    ) : kind === "bool" ? /* @__PURE__ */ jsx(
      "input",
      {
        type: "checkbox",
        checked: Boolean(value),
        onChange: (e) => onChange(e.target.checked),
        style: { width: 14, height: 14 }
      }
    ) : /* @__PURE__ */ jsx(
      "input",
      {
        type: kind === "num" ? "number" : "text",
        value: value === null || value === void 0 ? "" : String(value),
        onChange: (e) => onChange(coerceParam(def.type, e.target.value)),
        style: acInput
      }
    )
  ] });
}
function ActionPicker({
  x,
  y,
  actions,
  targetUids,
  onInvoke,
  onClose
}) {
  const [selected, setSelected] = useState(null);
  const [values, setValues] = useState({});
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState(null);
  const [error, setError] = useState(null);
  useEffect(() => {
    const dismiss = (e) => {
      const el = e.target;
      if (el && el.closest("[data-ce-node-menu]")) return;
      onClose();
    };
    const onEsc = (e) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("pointerdown", dismiss, true);
    document.addEventListener("contextmenu", dismiss, true);
    document.addEventListener("keydown", onEsc);
    return () => {
      document.removeEventListener("pointerdown", dismiss, true);
      document.removeEventListener("contextmenu", dismiss, true);
      document.removeEventListener("keydown", onEsc);
    };
  }, [onClose]);
  const run = async (a, params) => {
    setBusy(true);
    setError(null);
    try {
      const res = await onInvoke(targetUids, a.name, params);
      setResult(res[0]?.returns ?? {});
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  const choose = (a) => {
    setError(null);
    setResult(null);
    if (!a.params || a.params.length === 0) {
      void run(a, {});
      return;
    }
    const init = {};
    for (const p of a.params) init[p.name] = p.default ?? defaultForType(p.type);
    setValues(init);
    setSelected(a);
  };
  const PICKER_W = 280;
  const left = Math.min(x, window.innerWidth - PICKER_W - 8);
  const top = Math.min(y, window.innerHeight - 360);
  const count = targetUids.length;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-ce-node-menu": true,
      onContextMenu: (e) => e.preventDefault(),
      style: {
        position: "fixed",
        left,
        top,
        zIndex: 101,
        background: "#1a1d24",
        border: "1px solid #2c313c",
        borderRadius: 4,
        width: PICKER_W,
        maxHeight: 360,
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)",
        fontSize: 12,
        color: "#e6e8eb",
        fontFamily: "-apple-system, system-ui, sans-serif",
        display: "flex",
        flexDirection: "column"
      },
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              padding: "6px 8px",
              borderBottom: "1px solid #2c313c",
              display: "flex",
              alignItems: "center",
              gap: 6
            },
            children: [
              selected && !result && /* @__PURE__ */ jsx(
                "button",
                {
                  onClick: () => {
                    setSelected(null);
                    setError(null);
                  },
                  title: "Back",
                  style: {
                    background: "transparent",
                    border: "none",
                    color: "#9ecbff",
                    cursor: "pointer",
                    fontSize: 14,
                    padding: 0
                  },
                  children: "‹"
                }
              ),
              /* @__PURE__ */ jsx("div", { style: { color: "#8892a0", fontSize: 10, flex: 1 }, children: result ? "Result" : selected ? selected.label ?? selected.name : `Action on ${count === 1 ? "1 component" : `${count} components`}` })
            ]
          }
        ),
        /* @__PURE__ */ jsx("div", { style: { flex: 1, overflowY: "auto", padding: 8 }, children: result ? /* @__PURE__ */ jsxs("div", { children: [
          Object.keys(result).length === 0 ? /* @__PURE__ */ jsx("div", { style: { color: "#5a6172" }, children: "done — no return values" }) : Object.entries(result).map(([k, v]) => /* @__PURE__ */ jsxs("div", { style: acRow, children: [
            /* @__PURE__ */ jsx("span", { style: { color: "#8892a0" }, children: k }),
            /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb", fontVariantNumeric: "tabular-nums" }, children: String(v) })
          ] }, k)),
          /* @__PURE__ */ jsx("button", { onClick: onClose, style: acBtnPrimary, children: "Close" })
        ] }) : selected ? /* @__PURE__ */ jsxs(
          "form",
          {
            onSubmit: (e) => {
              e.preventDefault();
              void run(selected, values);
            },
            children: [
              (selected.params ?? []).map((p) => /* @__PURE__ */ jsx(
                ParamField,
                {
                  def: p,
                  value: values[p.name],
                  onChange: (v) => setValues((cur) => ({ ...cur, [p.name]: v }))
                },
                p.name
              )),
              error && /* @__PURE__ */ jsx("div", { style: { color: "#ffb8b8", margin: "6px 0" }, children: error }),
              /* @__PURE__ */ jsx("button", { type: "submit", disabled: busy, style: acBtnPrimary, children: busy ? "Running…" : `Run on ${count === 1 ? "1 component" : `${count} components`}` })
            ]
          }
        ) : actions.length === 0 ? /* @__PURE__ */ jsx("div", { style: { color: "#5a6172" }, children: "no actions for this component" }) : /* @__PURE__ */ jsxs("div", { style: { display: "flex", flexDirection: "column", gap: 2 }, children: [
          error && /* @__PURE__ */ jsx("div", { style: { color: "#ffb8b8", margin: "2px 0 6px" }, children: error }),
          actions.map((a) => /* @__PURE__ */ jsxs(
            "button",
            {
              onClick: () => choose(a),
              disabled: busy,
              title: a.description,
              style: acBtn,
              children: [
                /* @__PURE__ */ jsx("span", { children: a.label ?? a.name }),
                a.params && a.params.length > 0 ? /* @__PURE__ */ jsx("span", { style: { color: "#5a6172", fontSize: 10 }, children: "…" }) : null
              ]
            },
            a.name
          ))
        ] }) })
      ]
    }
  );
}
function buildRfNodes(comps, onEnter, onContextMenu, selectedIds, actionTypes) {
  const positions = layoutPositions(comps, NODE_W);
  return comps.map((c, i) => {
    const pos = positions[i];
    const id = String(c.uid);
    return {
      id,
      type: "fb",
      position: pos,
      width: NODE_W,
      data: {
        componentUid: c.uid,
        name: c.name,
        hasChildren: (c.childrenCount ?? 0) > 0,
        childCount: c.childrenCount ?? 0,
        hasActions: actionTypes?.has(c.type) ?? false,
        onEnter,
        onContextMenu
      },
      draggable: true,
      selected: selectedIds?.has(id) ?? false
    };
  });
}
function miniMapNodeColor(n) {
  if (n.selected) return "#6cb1ff";
  if (n.type === "ghost") return "#5a626e";
  return n.data?.hasChildren ? "#4f80c4" : "#7b8593";
}
function miniMapNodeStroke(n) {
  if (n.selected) return "#8cc4ff";
  return n.data?.hasChildren ? "#6cb1ff" : "#9aa3b2";
}
function buildRfEdges(edges, comps) {
  const cByUid = /* @__PURE__ */ new Map();
  for (const c of comps) cByUid.set(c.uid, c);
  const out = [];
  for (const e of edges) {
    const src = cByUid.get(e.sourceUid);
    const dst = cByUid.get(e.targetUid);
    const srcProp = src?.properties[e.sourceProperty];
    const dstProp = dst?.properties[e.targetProperty];
    if (!srcProp || !dstProp) continue;
    const isLoop = e.loopBack === true;
    out.push({
      id: String(e.uid),
      source: String(e.sourceUid),
      sourceHandle: String(srcProp.uid),
      target: String(e.targetUid),
      targetHandle: String(dstProp.uid),
      style: isLoop ? { stroke: "#7a8a9f", strokeWidth: 1.5, strokeDasharray: "6 4" } : { stroke: "#4a9eff", strokeWidth: 1.5 },
      animated: false
    });
  }
  return out;
}
function Breadcrumb({ crumbs, onGoTo }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        display: "flex",
        alignItems: "center",
        gap: 6,
        fontSize: 12,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        minWidth: 0,
        overflowX: "auto"
      },
      children: crumbs.map((c, i) => {
        const last = i === crumbs.length - 1;
        return /* @__PURE__ */ jsxs("span", { style: { display: "flex", alignItems: "center", gap: 6 }, children: [
          /* @__PURE__ */ jsx(
            "button",
            {
              onClick: () => onGoTo(i),
              disabled: last,
              style: {
                background: "transparent",
                color: last ? "#e6e8eb" : "#9ecbff",
                border: "none",
                padding: 0,
                cursor: last ? "default" : "pointer",
                fontFamily: "inherit",
                fontSize: 12,
                fontWeight: last ? 600 : 400
              },
              children: c.name
            }
          ),
          !last && /* @__PURE__ */ jsx("span", { style: { color: "#5a6172" }, children: "/" })
        ] }, c.uid);
      })
    }
  );
}
function LeftDock({
  palette,
  onAdd,
  currentParentUid,
  onPick
}) {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem("ce-ui.palette.collapsed") === "1";
    } catch {
      return false;
    }
  });
  useEffect(() => {
    try {
      window.localStorage.setItem("ce-ui.palette.collapsed", collapsed ? "1" : "0");
    } catch {
    }
  }, [collapsed]);
  const [tab, setTab] = useState("add");
  const [filter, setFilter] = useState("");
  const [sel, setSel] = useState(0);
  const inputRef = useRef(null);
  const listRef = useRef(null);
  const f = filter.trim().toLowerCase();
  useEffect(() => setSel(0), [filter, tab]);
  useEffect(() => {
    if (!collapsed) {
      const t = window.setTimeout(() => inputRef.current?.focus(), 0);
      return () => window.clearTimeout(t);
    }
  }, [collapsed, tab]);
  const addRows = useMemo(() => {
    const rows = [];
    for (const g of palette) {
      for (const c of g.components) {
        if (!f || c.name.toLowerCase().includes(f) || c.type.toLowerCase().includes(f)) {
          rows.push({ type: c.type, name: c.name, group: g.id });
        }
      }
    }
    return rows;
  }, [palette, f]);
  const [all, setAll] = useState(null);
  useEffect(() => {
    if (collapsed || tab !== "search") return;
    let cancelled = false;
    (async () => {
      try {
        const resp = await getRootNodes({ depth: -1, nested: true });
        if (cancelled) return;
        setAll(buildSearchIndex(resp.nodes, currentParentUid));
      } catch {
        if (!cancelled) setAll([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [collapsed, tab, currentParentUid]);
  const searchHits = useMemo(() => all ? rankSearchHits(all, f) : [], [all, f]);
  const count = tab === "add" ? addRows.length : searchHits.length;
  useEffect(() => {
    if (sel >= count) setSel(0);
  }, [count, sel]);
  useEffect(() => {
    listRef.current?.querySelector(`[data-idx="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  }, [sel, tab]);
  const activate = (i = sel) => {
    if (tab === "add") {
      const r = addRows[i];
      if (r) onAdd(r.type);
    } else {
      const h = searchHits[i];
      if (h) onPick(h.compUid);
    }
  };
  const onKey = (e) => {
    if (e.key === "Tab") {
      e.preventDefault();
      setTab((t) => t === "add" ? "search" : "add");
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSel((s) => Math.min(count - 1, s + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSel((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      activate();
    } else if (e.key === "Escape") {
      e.preventDefault();
      setCollapsed(true);
    }
    e.stopPropagation();
  };
  if (collapsed) {
    return /* @__PURE__ */ jsx(
      "button",
      {
        onClick: () => setCollapsed(false),
        title: "Add / search components",
        style: {
          position: "fixed",
          top: 12,
          left: 12,
          zIndex: 20,
          width: 28,
          height: 80,
          background: "rgba(20,23,30,0.92)",
          border: "1px solid #2c313c",
          borderRadius: 6,
          color: "#cbd3e0",
          cursor: "pointer",
          fontFamily: "inherit",
          fontSize: 14,
          display: "flex",
          alignItems: "center",
          justifyContent: "center"
        },
        children: /* @__PURE__ */ jsx(ChevronRight, { size: 16 })
      }
    );
  }
  const tabBtn = (id, label) => /* @__PURE__ */ jsx(
    "button",
    {
      onClick: () => setTab(id),
      style: {
        flex: 1,
        background: tab === id ? "#2c3a55" : "transparent",
        color: tab === id ? "#cfe0ff" : "#8892a0",
        border: "none",
        borderBottom: `2px solid ${tab === id ? "#4a9eff" : "transparent"}`,
        padding: "7px 8px",
        cursor: "pointer",
        fontFamily: "inherit",
        fontSize: 12,
        fontWeight: 600
      },
      children: label
    }
  );
  return /* @__PURE__ */ jsxs(
    "div",
    {
      onPointerDown: (e) => e.stopPropagation(),
      style: {
        position: "fixed",
        top: 12,
        left: 12,
        bottom: 12,
        zIndex: 20,
        background: "rgba(20, 23, 30, 0.96)",
        border: "1px solid #2c313c",
        borderRadius: 6,
        color: "#e6e8eb",
        fontSize: 12,
        width: 260,
        display: "flex",
        flexDirection: "column"
      },
      children: [
        /* @__PURE__ */ jsxs("div", { style: { display: "flex", alignItems: "stretch", borderBottom: "1px solid #2c313c" }, children: [
          tabBtn("add", "Add"),
          tabBtn("search", "Search"),
          /* @__PURE__ */ jsx(
            "button",
            {
              onClick: () => setCollapsed(true),
              title: "Hide panel",
              style: {
                display: "flex",
                alignItems: "center",
                background: "transparent",
                border: "none",
                borderLeft: "1px solid #2c313c",
                color: "#8892a0",
                cursor: "pointer",
                fontFamily: "inherit",
                padding: "0 10px"
              },
              children: /* @__PURE__ */ jsx(ChevronLeft, { size: 16 })
            }
          )
        ] }),
        /* @__PURE__ */ jsx("div", { style: { padding: "8px 10px", borderBottom: "1px solid #2c313c" }, children: /* @__PURE__ */ jsx(
          "input",
          {
            ref: inputRef,
            value: filter,
            onChange: (e) => setFilter(e.target.value),
            onKeyDown: onKey,
            placeholder: tab === "add" ? "filter components…   (Tab → Search)" : "name, label, alias…   (Tab → Add)",
            spellCheck: false,
            style: {
              width: "100%",
              background: "#222731",
              color: "#cbd3e0",
              border: "1px solid #2c313c",
              borderRadius: 3,
              padding: "5px 7px",
              fontSize: 11,
              fontFamily: "ui-monospace, monospace",
              boxSizing: "border-box",
              outline: "none"
            }
          }
        ) }),
        /* @__PURE__ */ jsx("div", { ref: listRef, style: { flex: 1, overflowY: "auto", padding: "4px 0" }, children: tab === "add" ? addRows.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 12px", color: "#5a6172", fontSize: 11 }, children: "no matches" }) : addRows.map((r, i) => {
          const prev = i > 0 ? addRows[i - 1] : null;
          const showHeader = !prev || prev.group !== r.group;
          return /* @__PURE__ */ jsxs("div", { "data-idx": i, children: [
            showHeader && /* @__PURE__ */ jsx(
              "div",
              {
                style: {
                  padding: "6px 12px 2px",
                  color: "#5a6172",
                  fontSize: 9,
                  textTransform: "uppercase",
                  letterSpacing: 0.5,
                  fontFamily: "ui-monospace, monospace"
                },
                children: r.group
              }
            ),
            /* @__PURE__ */ jsx(
              PaletteItem,
              {
                component: { name: r.name, type: r.type },
                onAdd: () => onAdd(r.type),
                selected: i === sel,
                onHover: () => setSel(i)
              }
            )
          ] }, `${r.type}:${i}`);
        }) : all == null ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 12px", color: "#5a6172", fontSize: 11 }, children: "loading…" }) : searchHits.length === 0 ? /* @__PURE__ */ jsx("div", { style: { padding: "10px 12px", color: "#5a6172", fontSize: 11 }, children: "no matches" }) : searchHits.map((h, i) => /* @__PURE__ */ jsxs(
          "button",
          {
            "data-idx": i,
            onMouseEnter: () => setSel(i),
            onClick: () => onPick(h.compUid),
            style: {
              display: "flex",
              flexDirection: "column",
              gap: 2,
              width: "100%",
              textAlign: "left",
              padding: "6px 12px",
              background: i === sel ? "#2c3a55" : "transparent",
              border: "none",
              borderLeft: `2px solid ${h.here ? "#4a9eff" : "transparent"}`,
              cursor: "pointer",
              fontFamily: "ui-monospace, SFMono-Regular, monospace"
            },
            children: [
              /* @__PURE__ */ jsx("div", { style: { display: "flex", alignItems: "baseline", gap: 6 }, children: h.propName ? /* @__PURE__ */ jsxs(Fragment, { children: [
                /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb", fontSize: 12 }, children: h.label || h.propName }),
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      color: "#7a8aa0",
                      fontSize: 9,
                      border: "1px solid #2c3a55",
                      borderRadius: 3,
                      padding: "0 4px",
                      flexShrink: 0
                    },
                    children: "prop"
                  }
                ),
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      color: "#5a6172",
                      fontSize: 11,
                      marginLeft: "auto",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap"
                    },
                    title: h.path,
                    children: h.compName
                  }
                )
              ] }) : /* @__PURE__ */ jsxs(Fragment, { children: [
                /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb", fontSize: 12 }, children: h.compName }),
                /* @__PURE__ */ jsx(
                  "span",
                  {
                    style: {
                      color: "#5a6172",
                      fontSize: 11,
                      marginLeft: "auto",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap"
                    },
                    title: `${h.path} · ${h.type}`,
                    children: h.here ? h.type : h.path
                  }
                )
              ] }) }),
              h.propName && h.aliasText && /* @__PURE__ */ jsx(
                "div",
                {
                  style: {
                    color: "#5a6172",
                    fontSize: 10,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap"
                  },
                  children: h.aliasText
                }
              )
            ]
          },
          `${h.compUid}:${h.propName ?? ""}:${i}`
        )) }),
        /* @__PURE__ */ jsxs(
          "div",
          {
            style: {
              padding: "6px 12px",
              borderTop: "1px solid #2c313c",
              fontSize: 10,
              color: "#5a6172",
              lineHeight: 1.5
            },
            children: [
              "Tab: switch • ↑↓ select • ↵ ",
              tab === "add" ? "add" : "go",
              tab === "add" && /* @__PURE__ */ jsxs(Fragment, { children: [
                /* @__PURE__ */ jsx("br", {}),
                "drag onto canvas to place • parent uid: ",
                currentParentUid
              ] })
            ]
          }
        )
      ]
    }
  );
}
function PaletteItem({
  component,
  onAdd,
  selected,
  onHover
}) {
  const [dragging, setDragging] = useState(false);
  return /* @__PURE__ */ jsxs(
    "div",
    {
      draggable: true,
      "data-idx-item": true,
      onDragStart: (e) => {
        e.dataTransfer.effectAllowed = "copy";
        e.dataTransfer.setData(DND_TYPE, component.type);
        setDragging(true);
      },
      onDragEnd: () => setDragging(false),
      onDoubleClick: onAdd,
      onMouseEnter: onHover,
      title: `${component.type} — double-click to add, drag to drop on canvas`,
      style: {
        margin: "0 8px 2px 8px",
        padding: "4px 8px 4px 22px",
        background: dragging || selected ? "#2c3a55" : "#1a1d24",
        color: "#cbd3e0",
        border: "1px solid #2c313c",
        borderRadius: 3,
        cursor: "grab",
        fontSize: 11,
        fontFamily: "ui-monospace, SFMono-Regular, monospace",
        userSelect: "none",
        display: "flex",
        flexDirection: "column",
        gap: 1
      },
      children: [
        /* @__PURE__ */ jsx("span", { style: { color: "#e6e8eb" }, children: component.name }),
        /* @__PURE__ */ jsx("span", { style: { fontSize: 9, color: "#5a6172", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }, children: component.type })
      ]
    }
  );
}
function ErrorBanner({
  error,
  onClose
}) {
  const [copied, setCopied] = useState(false);
  const copy = (e) => {
    e.stopPropagation();
    const text = error.debug ?? error.message;
    void navigator.clipboard?.writeText(text).then(
      () => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1200);
      },
      () => {
      }
    );
  };
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        position: "fixed",
        bottom: 12,
        left: "50%",
        transform: "translateX(-50%)",
        zIndex: 30,
        maxWidth: "min(720px, 90vw)",
        background: "#3a1a1a",
        border: "1px solid #6b2a2a",
        color: "#ffb8b8",
        padding: "6px 10px",
        borderRadius: 4,
        fontSize: 12,
        fontFamily: "ui-monospace, monospace",
        display: "flex",
        alignItems: "flex-start",
        gap: 8
      },
      children: [
        /* @__PURE__ */ jsx("span", { style: { whiteSpace: "pre-wrap", overflow: "hidden", flex: 1, maxHeight: 120 }, children: error.message }),
        /* @__PURE__ */ jsx(
          "button",
          {
            onClick: copy,
            title: error.debug ? "Copy request + response" : "Copy error",
            style: {
              flexShrink: 0,
              background: "#5a2a2a",
              color: "#ffd8d8",
              border: "1px solid #6b2a2a",
              borderRadius: 3,
              padding: "1px 8px",
              fontSize: 11,
              cursor: "pointer",
              fontFamily: "inherit"
            },
            children: copied ? "copied" : "copy"
          }
        ),
        /* @__PURE__ */ jsx(
          "button",
          {
            onClick: onClose,
            title: "Dismiss",
            style: {
              flexShrink: 0,
              background: "transparent",
              color: "#ffb8b8",
              border: "none",
              fontSize: 13,
              cursor: "pointer",
              lineHeight: 1,
              padding: "0 2px"
            },
            children: "✕"
          }
        )
      ]
    }
  );
}

function WiresheetPanel({ deviceId }) {
  const [device, setDevice] = React.useState(null);
  const [error, setError] = React.useState(null);
  const [loading, setLoading] = React.useState(true);
  React.useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    fetchTemplate(`${EXTENSION_ID}.device_get`, { device_id: deviceId }).then((rows) => {
      if (!alive) return;
      setDevice(rows[0] ?? null);
      setLoading(false);
    }).catch((e) => {
      if (!alive) return;
      setError(e instanceof Error ? e.message : String(e));
      setLoading(false);
    });
    return () => {
      alive = false;
    };
  }, [deviceId]);
  const base = device ? `http://${device.ip}:${device.port}` : null;
  return /* @__PURE__ */ jsxs(
    "div",
    {
      style: {
        // Full-bleed, but below the host sidebar (which is `fixed … z-10`) so
        // the rubix nav floats over the wiresheet rather than being covered.
        position: "fixed",
        inset: 0,
        zIndex: 5,
        background: "#0b0e14"
      },
      children: [
        loading && /* @__PURE__ */ jsx(Centered, { children: "Loading engine…" }),
        !loading && error && /* @__PURE__ */ jsxs(Centered, { children: [
          "Failed to load engine ",
          /* @__PURE__ */ jsx("code", { children: deviceId }),
          ": ",
          error
        ] }),
        !loading && !error && !device && /* @__PURE__ */ jsxs(Centered, { children: [
          "No such engine: ",
          /* @__PURE__ */ jsx("code", { children: deviceId })
        ] }),
        !loading && base && /* @__PURE__ */ jsx(CeEditor, { base })
      ]
    }
  );
}
function Centered({ children }) {
  return /* @__PURE__ */ jsx(
    "div",
    {
      style: {
        position: "absolute",
        inset: 0,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        color: "#9aa4b2",
        fontSize: 14,
        gap: 6
      },
      children
    }
  );
}

if (typeof window !== "undefined") {
  console.info("[com.nubeio.ce] bundle loaded — build-", "2026-06-13T11:28:35.381Z");
}
function Main() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(MainRouter, {}) });
}
function MainRouter() {
  const route = useExtensionRoute();
  if (route && route.startsWith("wiresheet")) {
    const deviceId = route.split("/")[1] ?? "";
    return /* @__PURE__ */ jsx(WiresheetPanel, { deviceId });
  }
  return /* @__PURE__ */ jsx(Page, { eyebrow: "Control Engine", title: "Devices", children: /* @__PURE__ */ jsx(DevicesPanel, {}) });
}

const TREE = [
  { title: "Devices", href: `/extensions/${EXTENSION_ID}/devices`, icon: LayoutGrid }
];
function NavTree() {
  return /* @__PURE__ */ jsx(BlockShell, { children: /* @__PURE__ */ jsx(NavTreeInner, {}) });
}
const MENU_BUTTON_CLASS = "peer/menu-button flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden ring-sidebar-ring transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 data-[active=true]:bg-sidebar-accent data-[active=true]:font-medium data-[active=true]:text-sidebar-accent-foreground [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 h-8 no-underline";
function NavTreeInner() {
  const path = typeof window !== "undefined" ? window.location.pathname : "";
  return /* @__PURE__ */ jsxs(
    "div",
    {
      "data-slot": "sidebar-group",
      "data-sidebar": "group",
      className: "relative flex w-full min-w-0 flex-col p-2",
      children: [
        /* @__PURE__ */ jsxs(
          "div",
          {
            "data-slot": "sidebar-group-label",
            "data-sidebar": "group-label",
            className: "flex h-8 shrink-0 items-center gap-2 rounded-md px-2 text-xs font-medium text-sidebar-foreground/70",
            children: [
              /* @__PURE__ */ jsx(Cpu, { className: "size-4" }),
              " Control Engine"
            ]
          }
        ),
        /* @__PURE__ */ jsx(
          "ul",
          {
            "data-slot": "sidebar-menu",
            "data-sidebar": "menu",
            className: "flex w-full min-w-0 flex-col gap-1",
            children: TREE.map((leaf) => {
              const isActive = path === leaf.href || path.startsWith(leaf.href + "/");
              const Icon = leaf.icon;
              return /* @__PURE__ */ jsx(
                "li",
                {
                  "data-slot": "sidebar-menu-item",
                  "data-sidebar": "menu-item",
                  className: "group/menu-item relative",
                  children: /* @__PURE__ */ jsxs(
                    "a",
                    {
                      href: leaf.href,
                      "data-slot": "sidebar-menu-button",
                      "data-sidebar": "menu-button",
                      "data-active": isActive,
                      className: MENU_BUTTON_CLASS,
                      children: [
                        /* @__PURE__ */ jsx(Icon, {}),
                        /* @__PURE__ */ jsx("span", { children: leaf.title })
                      ]
                    }
                  )
                },
                leaf.href
              );
            })
          }
        )
      ]
    }
  );
}

const factory = {
  singletons: {
    react: { version: "19.1.0" },
    "react-dom": { version: "19.1.0" }
  },
  init(handle) {
    registerExtensionContributions(handle, {
      components: { Main, NavTree }
    });
  }
};

export { factory as default };
