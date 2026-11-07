/**
 * What an AI app puts in the address when it sends the person to the consent step (the words of
 * the standard it speaks: AI-22). Every member is a text; the server judges them, and a request
 * that is not one is told to the person in plain words.
 */
export interface ConnectRequest {
  readonly client: string;
  readonly response_type: string;
  readonly redirect_uri: string;
  readonly code_challenge: string;
  readonly code_challenge_method: string;
  readonly scope: string;
  readonly resource: string;
  readonly state: string;
}

/** A member of the address, as a text; none is an empty text. */
function text(value: unknown): string {
  return typeof value === "string" ? value : "";
}

/** Reads the address of the consent step into the request of the app. */
export function connectRequestOf(search: Readonly<Record<string, unknown>>): ConnectRequest {
  return {
    client: text(search["client_id"]),
    response_type: text(search["response_type"]),
    redirect_uri: text(search["redirect_uri"]),
    code_challenge: text(search["code_challenge"]),
    code_challenge_method: text(search["code_challenge_method"]),
    scope: text(search["scope"]),
    resource: text(search["resource"]),
    state: text(search["state"]),
  };
}
