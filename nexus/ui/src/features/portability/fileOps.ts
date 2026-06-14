// Small browser file helpers for the export/import pages: trigger a JSON
// download, read a picked file as text, and copy to the clipboard. Kept in one
// module so the DOM/Blob plumbing stays out of the page components.

/** Download `content` as a file named `filename` (a transient object URL,
 *  revoked after the click so it doesn't leak). */
export function downloadTextFile(
  filename: string,
  content: string,
  type = "application/json",
): void {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

/** Read a `File` (from an `<input type=file>`) as UTF-8 text. */
export function readFileAsText(file: File): Promise<string> {
  return file.text();
}

/** Copy `text` to the clipboard, resolving to whether it succeeded (the API is
 *  unavailable on insecure origins / older browsers, so callers fall back to a
 *  manual-copy affordance). */
export async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** A filesystem-safe export filename derived from a dashboard slug. */
export function exportFilename(slug: string): string {
  const safe = slug.replace(/[^a-z0-9-_]+/gi, "-").replace(/^-+|-+$/g, "");
  return `${safe || "dashboard"}.dashboard.json`;
}
