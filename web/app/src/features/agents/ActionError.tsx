import { Alert, alertTone } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";

/** What the server refused, in plain words, under the button that asked; nothing while there is no refusal. */
export function ActionError({ error }: { readonly error: unknown }) {
  return error === null || error === undefined ? null : (
    <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
  );
}
