import type { SeriesPoint } from "@/data/types";

type Op = "+" | "-" | "*" | "/";

// Add a calculated field to every row: `field = left <op> right`, where
// `left`/`right` are other column names. Rows where either operand is
// non-numeric (or a division by zero) get `null` for the new field rather
// than NaN, so downstream renderers treat it as missing (F0). Pure.
export function applyCalculated(
  rows: ReadonlyArray<SeriesPoint>,
  field: string,
  left: string,
  op: Op,
  right: string,
): SeriesPoint[] {
  if (!field) return [...rows];
  return rows.map((row) => {
    const a = numeric(row[left]);
    const b = numeric(row[right]);
    return { ...row, [field]: compute(a, b, op) };
  });
}

function compute(a: number | null, b: number | null, op: Op): number | null {
  if (a == null || b == null) return null;
  switch (op) {
    case "+":
      return a + b;
    case "-":
      return a - b;
    case "*":
      return a * b;
    case "/":
      return b === 0 ? null : a / b;
  }
}

function numeric(v: SeriesPoint[string]): number | null {
  if (typeof v === "number") return Number.isFinite(v) ? v : null;
  if (typeof v === "string" && v.trim() !== "") {
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  }
  return null;
}
