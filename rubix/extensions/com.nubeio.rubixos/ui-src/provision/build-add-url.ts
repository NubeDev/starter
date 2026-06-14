// Build the canonical `rubix://add?…` payload (BARCODE.md §2) that a sticker
// QR encodes. Single source of truth for the on-sticker grammar so a hand-made
// sticker scans back through `bc_decode` identically to a factory one.
//
// `model` is the device *type* (must match a template); the address slot is
// `eui` for lora, `addr` for bacnet, and `ip` for rubix/REST.

export interface AddUrlParts {
  id: string;
  model: string;
  network: string;
  address?: string;
}

export function buildAddUrl({ id, model, network, address }: AddUrlParts): string {
  const params = new URLSearchParams({ v: "1", id, model, network });
  const addr = address?.trim();
  if (addr) {
    const slot = network === "bacnet" ? "addr" : network === "lora" ? "eui" : "ip";
    params.set(slot, addr);
  }
  return `rubix://add?${params.toString()}`;
}

// Human label for the address field, per the template's network.
export function addressLabel(network: string): string {
  switch (network) {
    case "lora":
      return "DevEUI";
    case "bacnet":
      return "BACnet address";
    default:
      return "IP address";
  }
}

// Mint a readable serial like `DRP-9F2C18` from a template key. Uses
// crypto.randomUUID where available; callers keep it out of render paths.
export function mintSerial(template: string): string {
  const prefix = template.slice(0, 3).toUpperCase();
  const rnd =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID().replace(/-/g, "").slice(0, 6).toUpperCase()
      : Math.abs(template.length * 7919).toString(16).toUpperCase();
  return `${prefix}-${rnd}`;
}
