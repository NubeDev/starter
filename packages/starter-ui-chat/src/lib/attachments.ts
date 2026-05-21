import type { ChatAttachment } from "../types/index.js";
import { makeId } from "./utils.js";

// Convert a browser `File` into a `ChatAttachment`. The blob is kept on
// the attachment so adapters can upload bytes; a `blob:` object URL is
// generated for inline previews. Caller is responsible for revoking the
// URL when the message scrolls off-screen / chat is cleared if memory
// pressure matters (typically negligible for chat-sized payloads).
export function fileToAttachment(file: File): ChatAttachment {
  const url =
    typeof URL !== "undefined" && typeof URL.createObjectURL === "function"
      ? URL.createObjectURL(file)
      : undefined;
  return {
    id: makeId("att"),
    name: file.name,
    mimeType: file.type || "application/octet-stream",
    sizeBytes: file.size,
    url,
    file,
  };
}

export function isImageAttachment(a: ChatAttachment): boolean {
  return a.mimeType.startsWith("image/");
}

export function formatBytes(n?: number): string {
  if (n == null || !Number.isFinite(n)) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
