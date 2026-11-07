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

/**
 * Reads the address of the consent step into the request of the app. The members are read from the
 * text of the address as they were written: the router would turn `123` into a number and `{}` into
 * an object, and a code challenge or a state of an app is a text whatever it looks like. A member
 * that is not there is an empty text, and the server judges the request.
 */
export function connectRequestOf(search: string): ConnectRequest {
  const members = new URLSearchParams(search);
  const text = (name: string): string => members.get(name) ?? "";
  return {
    client: text("client_id"),
    response_type: text("response_type"),
    redirect_uri: text("redirect_uri"),
    code_challenge: text("code_challenge"),
    code_challenge_method: text("code_challenge_method"),
    scope: text("scope"),
    resource: text("resource"),
    state: text("state"),
  };
}
