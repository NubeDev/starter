import type { CustomerRow, RuleOutcome } from "./types";

// Mirror of `evaluate_customer_quality` in `process/src/main.rs`.
// Both implementations must agree — the server-side rule is the
// authority; the UI runs it client-side only for the preview.
export function evaluateCustomerQuality(row: CustomerRow): RuleOutcome {
  const country = (row.country || "").trim();
  if (!country) {
    return {
      outcome: "flag",
      quality: "MissingCountry",
      note: `customer_id=${row.customer_id || "<unknown>"}`,
    };
  }
  const email = (row.email || "").trim();
  if (!email) {
    return {
      outcome: "flag",
      quality: "MissingEmail",
      note: `customer_id=${row.customer_id || "<unknown>"}`,
    };
  }
  if (!email.includes("@")) {
    return { outcome: "flag", quality: "InvalidEmail", note: `email=${email}` };
  }
  const d = row.subscription_date;
  if (d && !/^(20\d{2}|2100)-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$/.test(d)) {
    return { outcome: "flag", quality: "BadDate", note: `subscription_date=${d}` };
  }
  return { outcome: "ok" };
}
