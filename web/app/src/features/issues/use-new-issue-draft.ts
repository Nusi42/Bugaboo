import { saveDraft, throwAwayDraft, type DraftTarget } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { forgetBrowserDraft, writeBrowserDraft } from "../../shared/editing/browser-drafts.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { useLater } from "../../shared/use-later.ts";
import { useMe } from "../../shared/use-me.ts";
import { findDraft, type FormDraft } from "./find-new-issue-draft.ts";
import { encodeDraft, hasWriting, type NewIssueValues } from "./new-issue-form.ts";

/** How long the server waits for the next key before it keeps the draft (the browser keeps every key). */
const SERVER_DELAY_MS = 1500;

/** What the form does with its draft. */
export interface NewIssueDraft {
  /** The draft found for this project, until it is used or thrown away. */
  readonly found: FormDraft | null;
  /** Keeps what is written now (the browser at once, the server a moment after the last key). */
  readonly keep: (values: NewIssueValues) => void;
  /** Throws the draft away on both sides: the issue was made, or the person started again. */
  readonly forget: () => void;
}

/**
 * The draft of the new-issue form of a project (ISS-11, M3-Q9): kept in the browser and on the
 * server while the person writes, offered as *Continue your draft* when the form opens, and gone
 * after *Create*. Without a project there is no draft yet.
 */
export function useNewIssueDraft(project: string | undefined): NewIssueDraft {
  const person = useMe()?.id ?? null;
  const target: DraftTarget | undefined =
    project === undefined ? undefined : { kind: "issue", project };
  const found = useQuery({
    queryKey: queryKeys.issues.drafts(project ?? ""),
    queryFn: () => (target === undefined || person === null ? null : findDraft(person, target)),
    enabled: target !== undefined && person !== null,
    staleTime: Infinity,
    gcTime: 0,
  });
  const later = useLater();
  const forget = (): void => {
    later.cancel();
    if (target !== undefined) {
      forgetBrowserDraft(person, target);
      void throwAwayDraft(api.client, target).catch(() => null);
    }
  };
  return {
    found: found.data ?? null,
    keep: (values) => {
      if (target === undefined) {
        return;
      }
      if (!hasWriting(values)) {
        forget();
        return;
      }
      writeBrowserDraft(person, target, { text: encodeDraft(values), baseVersion: null });
      later.schedule(SERVER_DELAY_MS, () => {
        void saveDraft(api.client, target, { text: encodeDraft(values) }).catch(() => null);
      });
    },
    forget,
  };
}
