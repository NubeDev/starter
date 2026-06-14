// Browser file helpers for flow share/import: turn a JSON model into a
// downloaded file, and read a user-picked file back into a parsed object. Kept
// framework-free so the dashboards feature can lift them later.

// A filesystem-safe filename stem from a flow name: lowercase, non-alphanumerics
// collapsed to a single hyphen, trimmed. Empty names fall back to "flow".
export function fileStem(name: string): string {
  const stem = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stem || "flow";
}

// Trigger a browser download of `data` as pretty-printed JSON named `filename`.
// Uses an object URL + a transient anchor; revokes the URL after the click so it
// is not leaked.
export function downloadJson(filename: string, data: unknown): void {
  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

// Read a user-picked file and parse it as JSON. Rejects with a readable message
// when the file isn't valid JSON, so the caller can surface it.
export function readJsonFile(file: File): Promise<unknown> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("Couldn't read the file."));
    reader.onload = () => {
      try {
        resolve(JSON.parse(String(reader.result)));
      } catch {
        reject(new Error("That file isn't valid JSON."));
      }
    };
    reader.readAsText(file);
  });
}
