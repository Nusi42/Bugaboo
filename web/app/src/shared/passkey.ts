/**
 * Passkeys in the browser (AUTH-02). The server writes its challenges in the JSON form of the
 * WebAuthn standard, and the browser can read and write that form itself, so nothing here
 * converts bytes by hand.
 */

/** The person did not use the passkey: they closed the question, or no passkey fits. */
export class PasskeyCancelled extends Error {
  constructor() {
    super("passkey cancelled");
    this.name = "PasskeyCancelled";
  }
}

/** The answer of a device, in the JSON form that the server reads. */
export type PasskeyAnswer = Record<string, unknown>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Whether this browser can make and use passkeys at all. */
export function passkeysSupported(): boolean {
  return (
    typeof PublicKeyCredential !== "undefined" &&
    typeof PublicKeyCredential.parseCreationOptionsFromJSON === "function" &&
    typeof PublicKeyCredential.parseRequestOptionsFromJSON === "function"
  );
}

/** Whether the challenge for a new passkey has what the standard asks for. */
function isCreationOptions(value: unknown): value is PublicKeyCredentialCreationOptionsJSON {
  return (
    isRecord(value) &&
    isRecord(value["rp"]) &&
    isRecord(value["user"]) &&
    typeof value["challenge"] === "string" &&
    Array.isArray(value["pubKeyCredParams"])
  );
}

/** Whether the challenge for using a passkey has what the standard asks for. */
function isRequestOptions(value: unknown): value is PublicKeyCredentialRequestOptionsJSON {
  return isRecord(value) && typeof value["challenge"] === "string";
}

/** The device did not answer: the person closed the question (`NotAllowedError`) or none fits. */
function cancelled(error: unknown): boolean {
  return (
    error instanceof DOMException &&
    (error.name === "NotAllowedError" ||
      error.name === "AbortError" ||
      error.name === "InvalidStateError")
  );
}

/** What a device made, as the server reads it. */
function answerOf(credential: Credential | null): PasskeyAnswer {
  if (credential instanceof PublicKeyCredential) {
    return { ...credential.toJSON() };
  }
  throw new PasskeyCancelled();
}

/** Makes a new passkey on this device, for the challenge of the server. */
export async function createPasskey(
  options: Readonly<Record<string, unknown>>,
): Promise<PasskeyAnswer> {
  try {
    // The server wraps the challenge as { publicKey: … }, which is what the browser reads.
    const { publicKey: challenge } = options;
    if (!isCreationOptions(challenge)) {
      throw new PasskeyCancelled();
    }
    const publicKey = PublicKeyCredential.parseCreationOptionsFromJSON(challenge);
    return answerOf(await navigator.credentials.create({ publicKey }));
  } catch (error) {
    throw cancelled(error) ? new PasskeyCancelled() : error;
  }
}

/** Answers the challenge of the server with a passkey of this device, for the challenge of the server. */
export async function signWithPasskey(
  options: Readonly<Record<string, unknown>>,
): Promise<PasskeyAnswer> {
  try {
    const { publicKey: challenge } = options;
    if (!isRequestOptions(challenge)) {
      throw new PasskeyCancelled();
    }
    const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(challenge);
    return answerOf(await navigator.credentials.get({ publicKey }));
  } catch (error) {
    throw cancelled(error) ? new PasskeyCancelled() : error;
  }
}
