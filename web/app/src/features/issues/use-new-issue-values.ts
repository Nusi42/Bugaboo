import { useState } from "react";

import { issueTypeLinesOf, useIssueTypes } from "../issue-types/index.ts";
import { useProject } from "../projects/index.ts";
import type { FormDraft } from "./find-new-issue-draft.ts";
import { emptyForm, hasWriting, type NewIssueValues } from "./new-issue-form.ts";
import { lastProject } from "./new-issue-state.ts";
import { useNewIssueDraft } from "./use-new-issue-draft.ts";

/** The type that counts: the one picked, else the first of the list. */
function withType(
  typed: NewIssueValues,
  types: readonly { readonly id: string }[],
): NewIssueValues {
  return { ...typed, issueType: typed.issueType === "" ? (types[0]?.id ?? "") : typed.issueType };
}

/** What the fields of the form of a new issue are made of. */
export interface NewIssueValuesState {
  /** The project as the form was told: a short name. */
  readonly address: string | undefined;
  readonly pick: (address: string) => void;
  /** The project, once it is read: its name and short name. */
  readonly project: { readonly name: string; readonly shortName: string } | undefined;
  readonly types: readonly { readonly id: string; readonly name: string }[];
  readonly values: NewIssueValues;
  readonly change: (values: NewIssueValues) => void;
  /** The draft found for this project, until it is used or thrown away. */
  readonly offered: FormDraft | null;
  readonly continueDraft: (draft: FormDraft) => void;
  readonly dropDraft: () => void;
  /** The issue was made: the draft goes, and the form starts again with the same project and type. */
  readonly startAgain: () => void;
  /** The issue was made and the dialog is done: the draft goes. */
  readonly forgetDraft: () => void;
}

/**
 * The values of the form of a new issue (ISS-11): the project it starts with, the type that is the
 * first until another is picked, and the draft that is kept while the person writes and offered
 * when the form opens.
 */
export function useNewIssueValues(): NewIssueValuesState {
  const [address, setAddress] = useState<string | undefined>(lastProject());
  const [typed, setTyped] = useState<NewIssueValues>(emptyForm);
  const [dropped, setDropped] = useState(false);
  const found = useProject(address).data;
  const types = issueTypeLinesOf(useIssueTypes().data?.pages).filter((type) => !type.retired);
  const draft = useNewIssueDraft(found?.project.id);
  const values = withType(typed, types);
  const change = (next: NewIssueValues): void => {
    setTyped(next);
    draft.keep(next);
  };
  return {
    address,
    types,
    values,
    change,
    project:
      found === undefined
        ? undefined
        : { name: found.project.name, shortName: found.project.short_name },
    offered: draft.found !== null && !dropped && !hasWriting(typed) ? draft.found : null,
    pick: (picked) => {
      setAddress(picked);
      setDropped(false);
    },
    continueDraft: (kept) => {
      change(kept.values);
      setDropped(true);
    },
    dropDraft: () => {
      draft.forget();
      setDropped(true);
    },
    startAgain: () => {
      draft.forget();
      setTyped({ ...emptyForm, issueType: values.issueType });
    },
    forgetDraft: draft.forget,
  };
}
