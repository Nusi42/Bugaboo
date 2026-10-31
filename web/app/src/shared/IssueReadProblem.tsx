import { Alert, alertTone } from "@bugaboo/ui";

import { codeOf, messageOf } from "./errors.ts";

/**
 * Why a part of the issue page (its history, files, relations, sub-issues, labels or tags) could not be read, in that
 * part. A part says nothing when the issue itself is not there any more: the reads of every part fail alike then, and
 * the page says it once, not once in each part (a delete that the server refused because somebody deleted the issue
 * first showed the same sentence three times).
 */
export function IssueReadProblem({ error }: { readonly error: unknown }) {
  return codeOf(error) === "resource.not-found" ? null : (
    <Alert tone={alertTone.danger}>{messageOf(error)}</Alert>
  );
}
