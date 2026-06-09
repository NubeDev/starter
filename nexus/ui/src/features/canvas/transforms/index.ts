// Client-side transform pipeline applied to query rows before render.
// Barrel: re-exports the orchestrator and each pure transform so callers
// import from one place. Each transform lives in its own verb file.

export { applyTransforms } from "@/features/canvas/transforms/apply";
export { applyRename } from "@/features/canvas/transforms/rename";
export { applyCalculated } from "@/features/canvas/transforms/calculated";
export { applyFilter } from "@/features/canvas/transforms/filter";
export { applyGroupBy } from "@/features/canvas/transforms/groupBy";
export { applyReduce } from "@/features/canvas/transforms/reduce";
export { applyOrganize } from "@/features/canvas/transforms/organize";
