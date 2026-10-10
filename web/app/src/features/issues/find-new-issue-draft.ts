import { readDraft, type DraftTarget } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { readBrowserDraft } from "../../shared/editing/browser-drafts.ts";
import { decodeDraft, hasWriting, type NewIssueValues } from "./new-issue-form.ts";

/** A draft of the form of a new issue, found when the dialog opened. */
export interface FormDraft {
  readonly values: NewIssueValues;
  /** When it was kept, as a moment in time. */
  readonly savedAt: string;
}

/** A copy of a draft: the text and when it was kept. */
interface Copy {
  readonly text: string;
  readonly savedAt: string;
}

/** The draft a copy holds, when it is a draft of this form with something written in it. */
function draftOf(copy: Copy | null): FormDraft | undefined {
  const values = copy === null ? undefined : decodeDraft(copy.text);
  return copy === null || values === undefined || !hasWriting(values)
    ? undefined
    : { values, savedAt: copy.savedAt };
}

/** What this person kept for the form of a project: the newer of the browser's and the server's copy. */
export async function findDraft(person: string, target: DraftTarget): Promise<FormDraft | null> {
  const here = readBrowserDraft(person, target);
  const there = await readDraft(api.client, target).catch(() => null);
  const drafts = [
    draftOf(here === null ? null : { text: here.text, savedAt: here.savedAt }),
    draftOf(there === null ? null : { text: there.text, savedAt: there.saved_at }),
  ];
  let newest: FormDraft | null = null;
  for (const draft of drafts) {
    if (draft !== undefined && (newest === null || draft.savedAt > newest.savedAt)) {
      newest = draft;
    }
  }
  return newest;
}
