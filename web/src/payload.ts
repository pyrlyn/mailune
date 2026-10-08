// Shape of `save_draft` params from mailune-rpc. `name` is null when the
// address has no display name, matching serde's Option.

export interface AddressPayload {
  name: string | null;
  email: string;
}

export interface SaveDraftPayload {
  to: AddressPayload[];
  subject: string;
  body: string;
}

export function encodePayload(payload: SaveDraftPayload): string {
  return JSON.stringify(payload);
}

export function decodePayload(text: string): SaveDraftPayload {
  const value: unknown = JSON.parse(text);
  if (!isSaveDraft(value)) {
    throw new Error("payload is not a draft");
  }
  return value;
}

function isSaveDraft(value: unknown): value is SaveDraftPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return (
    Array.isArray(record.to) &&
    record.to.every(isAddress) &&
    typeof record.subject === "string" &&
    typeof record.body === "string"
  );
}

function isAddress(value: unknown): value is AddressPayload {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  const nameOk = record.name === null || typeof record.name === "string";
  return nameOk && typeof record.email === "string";
}
