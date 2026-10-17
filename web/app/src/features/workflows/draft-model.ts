import type { components, Schema } from "@bugaboo/api-client";

import { newId } from "../../shared/new-ids.ts";
import { noRules, rulesBodyOf, rulesOf, sameRules, type ChangeRules } from "./rules-model.ts";

/** The status group of a status. */
export type StatusGroupWord = Schema<"StatusRef">["group"];

/** A status in the draft. A new status has no key yet: the server creates it from the name. */
export interface DraftStatus {
  /** Names the status in the draft: the key it has, or an invented one while it is new. */
  readonly id: string;
  readonly key: string | null;
  readonly name: string;
  readonly group: StatusGroupWord;
}

/** A status change in the draft, between two statuses of the draft (by their `id`). */
export interface DraftChange {
  readonly id: string;
  readonly from: string;
  readonly to: string;
  /** The words of its button. */
  readonly button: string;
  readonly rules: ChangeRules;
}

/** Everything a person changed in a workflow until *Publish*. */
export interface Draft {
  readonly name: string;
  /** In order; the first is where a new issue starts. */
  readonly statuses: readonly DraftStatus[];
  readonly changes: readonly DraftChange[];
}

/** The draft that starts from what the editor shows. */
export function draftOf(editor: Schema<"WorkflowEditorBody">): Draft {
  return {
    name: editor.name,
    statuses: editor.statuses.map((status) => ({
      id: status.key,
      key: status.key,
      name: status.name,
      group: status.group,
    })),
    changes: editor.changes.map((change) => ({
      id: `${change.from}>${change.to}`,
      from: change.from,
      to: change.to,
      button: change.button,
      rules: rulesOf(change.rules),
    })),
  };
}

/** How the body of *Publish* names a status in a status change: by its key, a new one by its name. */
function nameInBody(draft: Draft, id: string): string {
  const status = draft.statuses.find((each) => each.id === id);
  return status === undefined ? id : (status.key ?? status.name);
}

/** The name of a status of the draft by its ID in the draft. */
export function nameOf(draft: Draft, id: string): string {
  return draft.statuses.find((each) => each.id === id)?.name ?? id;
}

/** The statuses and status changes as *Publish* takes them. */
export function bodyOf(
  draft: Draft,
): Pick<components["schemas"]["PublishWorkflowBody"], "statuses" | "changes"> {
  return {
    statuses: draft.statuses.map((status) =>
      status.key === null
        ? { name: status.name, group: status.group }
        : { key: status.key, name: status.name, group: status.group },
    ),
    changes: draft.changes.map((change) => ({
      from: nameInBody(draft, change.from),
      to: nameInBody(draft, change.to),
      button: change.button,
      rules: rulesBodyOf(change.rules),
    })),
  };
}

/** Whether two status changes say the same, as *Publish* would send them. */
function sameChange(
  a: DraftChange,
  b: DraftChange,
  draft: { readonly a: Draft; readonly b: Draft },
) {
  return (
    nameInBody(draft.a, a.from) === nameInBody(draft.b, b.from) &&
    nameInBody(draft.a, a.to) === nameInBody(draft.b, b.to) &&
    a.button === b.button &&
    sameRules(a.rules, b.rules)
  );
}

/** Whether two drafts say the same: the invented IDs do not count. */
export function sameDraft(a: Draft, b: Draft): boolean {
  return (
    a.name === b.name &&
    JSON.stringify(bodyOf(a).statuses) === JSON.stringify(bodyOf(b).statuses) &&
    a.changes.length === b.changes.length &&
    a.changes.every((change, at) => {
      const other = b.changes[at];
      return other !== undefined && sameChange(change, other, { a, b });
    })
  );
}

/** The status changes that start in a status, in order. */
export function changesFrom(draft: Draft, status: string): readonly DraftChange[] {
  return draft.changes.filter((change) => change.from === status);
}

/** Whether a name is taken by a status already, in any case. */
function taken(draft: Draft, name: string): boolean {
  return draft.statuses.some((status) => status.name.toLowerCase() === name.toLowerCase());
}

/** The draft with a new status at the end; the same draft for no name or a name that is taken. */
export function addStatus(draft: Draft, name: string, group: StatusGroupWord): Draft {
  const trimmed = name.trim();
  if (trimmed === "" || taken(draft, trimmed)) {
    return draft;
  }
  const added: DraftStatus = { id: newId("status"), key: null, name: trimmed, group };
  return { ...draft, statuses: [...draft.statuses, added] };
}

/** The draft with one status changed. */
function withStatus(draft: Draft, id: string, change: Partial<DraftStatus>): Draft {
  return {
    ...draft,
    statuses: draft.statuses.map((status) =>
      status.id === id ? { ...status, ...change } : status,
    ),
  };
}

/** The draft with a status called something else; its key stays. */
export function renameStatus(draft: Draft, id: string, name: string): Draft {
  return withStatus(draft, id, { name });
}

/** The draft with a status in another status group. */
export function setGroup(draft: Draft, id: string, group: StatusGroupWord): Draft {
  return withStatus(draft, id, { group });
}

/** The draft without a status, and without the status changes from and to it. */
export function removeStatus(draft: Draft, id: string): Draft {
  return {
    ...draft,
    statuses: draft.statuses.filter((status) => status.id !== id),
    changes: draft.changes.filter((change) => change.from !== id && change.to !== id),
  };
}

/** The draft with a status one place up (`-1`) or down (`1`); the same draft at the ends. */
export function moveStatus(draft: Draft, id: string, by: -1 | 1): Draft {
  const at = draft.statuses.findIndex((status) => status.id === id);
  const target = at + by;
  const [mine, theirs] = [draft.statuses[at], draft.statuses[target]];
  if (mine === undefined || theirs === undefined) {
    return draft;
  }
  const statuses = draft.statuses.map((status, place) => {
    if (place === at) {
      return theirs;
    }
    return place === target ? mine : status;
  });
  return { ...draft, statuses };
}

/**
 * The draft with a new status change; the same draft for one between a status and itself, one that
 * is there already, or one with no words.
 */
export function addChange(draft: Draft, from: string, to: string, button: string): Draft {
  const words = button.trim();
  const there = draft.changes.some((change) => change.from === from && change.to === to);
  if (words === "" || from === to || there) {
    return draft;
  }
  const added: DraftChange = { id: newId("change"), from, to, button: words, rules: noRules };
  return { ...draft, changes: [...draft.changes, added] };
}

/** The draft without a status change. */
export function removeChange(draft: Draft, id: string): Draft {
  return { ...draft, changes: draft.changes.filter((change) => change.id !== id) };
}

/** The draft with one status change changed. */
function withChange(draft: Draft, id: string, change: Partial<DraftChange>): Draft {
  return {
    ...draft,
    changes: draft.changes.map((each) => (each.id === id ? { ...each, ...change } : each)),
  };
}

/** The draft with other words on a button. */
export function setButton(draft: Draft, id: string, button: string): Draft {
  return withChange(draft, id, { button });
}

/** The draft with other rules on a status change. */
export function setRules(draft: Draft, id: string, rules: ChangeRules): Draft {
  return withChange(draft, id, { rules });
}

/** The draft with another name for the workflow. */
export function renameWorkflow(draft: Draft, name: string): Draft {
  return { ...draft, name };
}
