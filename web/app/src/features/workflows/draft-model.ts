import type { components, Schema } from "@bugaboo/api-client";
import type { NameKey } from "@bugaboo/wasm";

import { newId } from "../../shared/new-ids.ts";
import { nameAmong, nameStandingTwice } from "../../shared/same-name.ts";
import { noRules, rulesBodyOf, rulesOf, sameRules, type ChangeRules } from "./rules-model.ts";

/** A status in the draft. A new status has no key yet: the server creates it from the name. */
export interface DraftStatus {
  /** Names the status in the draft: the key it has, or an invented one while it is new. */
  readonly id: string;
  readonly key: string | null;
  readonly name: string;
  readonly group: Schema<"StatusGroup">;
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
      group: status.status_group,
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

/** How *Publish* names a status in a status change: by its key, a new one (it has no key yet) by its name. */
export function statusRefOf(status: DraftStatus): string {
  return status.key ?? status.name;
}

/** How the body of *Publish* names the status of the draft with this ID. */
function nameInBody(draft: Draft, id: string): string {
  const status = draft.statuses.find((each) => each.id === id);
  return status === undefined ? id : statusRefOf(status);
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
        ? { name: status.name, status_group: status.group }
        : { key: status.key, name: status.name, status_group: status.group },
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

/**
 * The draft with a new status at the end. The draft holds what the person typed: whether the name is
 * a good one (taken, empty) is the server's to say, when the draft is published; the rule is not
 * written a second time here.
 */
export function addStatus(draft: Draft, name: string, group: Schema<"StatusGroup">): Draft {
  const added: DraftStatus = { id: newId("status"), key: null, name: name.trim(), group };
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
export function setGroup(draft: Draft, id: string, group: Schema<"StatusGroup">): Draft {
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

/** The draft with a new status change, as typed; *Publish* judges it, the draft does not. */
export function addChange(draft: Draft, from: string, to: string, button: string): Draft {
  const added: DraftChange = {
    id: newId("change"),
    from,
    to,
    button: button.trim(),
    rules: noRules,
  };
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

/** Whether another status of the draft has this name (the rule of the server, `nameAmong`); `id` is the status being renamed, none for a new one. */
export function statusNameTaken(
  draft: Draft,
  id: string | null,
  name: string,
  keyOf: NameKey,
): boolean {
  return nameAmong(
    draft.statuses.filter((status) => status.id !== id).map((status) => status.name),
    name,
    keyOf,
  );
}

/** The name of the first status that another one in the draft has too, if any: *Publish* is refused for it. */
export function statusNamedTwice(draft: Draft, keyOf: NameKey): string | undefined {
  return nameStandingTwice(
    draft.statuses.map((status) => status.name),
    keyOf,
  );
}
