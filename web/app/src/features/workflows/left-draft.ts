import { workflowEditPart, type DraftTarget, type Schema } from "@bugaboo/api-client";
import { useState } from "react";

import {
  forgetBrowserDraft,
  readBrowserDraft,
  writeBrowserDraft,
} from "../../shared/editing/browser-drafts.ts";
import { signedInPersonId } from "../../shared/session.ts";
import type { Draft, DraftChange, DraftStatus } from "./draft-model.ts";
import { rulesBodyOf, rulesOf } from "./rules-model.ts";
import { isRecord } from "../../shared/is-record.ts";

/**
 * The draft a refused *Publish* left, kept by the browser (ADR-0019: nothing typed is dropped). The
 * server keeps no draft of a workflow, so a reload would lose it; the browser keeps it with the
 * drafts of texts, by person and by workflow, for as long as they are kept.
 */

/** What a workflow's draft is kept for. */
function targetOf(workflow: string): DraftTarget {
  return { kind: "workflow", id: workflow, part: workflowEditPart };
}

function isStatus(value: unknown): value is DraftStatus {
  return (
    isRecord(value) &&
    typeof value["id"] === "string" &&
    typeof value["name"] === "string" &&
    typeof value["group"] === "string" &&
    (value["key"] === null || typeof value["key"] === "string")
  );
}

/** A status change as kept: the rules in the shape *Publish* takes. */
interface KeptChange extends Omit<DraftChange, "rules"> {
  readonly rules: Schema<"StatusRulesBody">;
}

function isChange(value: unknown): value is KeptChange {
  return (
    isRecord(value) &&
    ["id", "from", "to", "button"].every((key) => typeof value[key] === "string") &&
    isRecord(value["rules"])
  );
}

/** The text a draft is kept as. */
export function keptText(draft: Draft): string {
  return JSON.stringify({
    name: draft.name,
    statuses: draft.statuses,
    changes: draft.changes.map((change): KeptChange => ({
      id: change.id,
      from: change.from,
      to: change.to,
      button: change.button,
      rules: rulesBodyOf(change.rules),
    })),
  });
}

/** The draft a kept text holds, or nothing when the text is not one. */
export function draftOfKept(text: string): Draft | undefined {
  try {
    const found: unknown = JSON.parse(text);
    if (
      !isRecord(found) ||
      typeof found["name"] !== "string" ||
      !Array.isArray(found["statuses"]) ||
      !found["statuses"].every(isStatus) ||
      !Array.isArray(found["changes"]) ||
      !found["changes"].every(isChange)
    ) {
      return undefined;
    }
    return {
      name: found["name"],
      statuses: found["statuses"],
      changes: found["changes"].map((change): DraftChange => ({
        id: change.id,
        from: change.from,
        to: change.to,
        button: change.button,
        rules: rulesOf(change.rules),
      })),
    };
  } catch {
    return undefined;
  }
}

/**
 * The draft that a refused publish of this workflow left, and the way to leave one or to throw it
 * away. It is read when the editor opens, so it is there after a reload as well.
 */
export function useLeftDraft(
  workflow: string,
  version: number | undefined,
): readonly [Draft | undefined, (draft?: Draft) => void] {
  const [left, setLeft] = useState<Draft | undefined>(() => {
    const kept = readBrowserDraft(signedInPersonId(), targetOf(workflow));
    return kept === null ? undefined : draftOfKept(kept.text);
  });
  return [
    left,
    (draft) => {
      if (draft === undefined) {
        forgetBrowserDraft(signedInPersonId(), targetOf(workflow));
      } else {
        writeBrowserDraft(signedInPersonId(), targetOf(workflow), {
          text: keptText(draft),
          baseVersion: version ?? null,
        });
      }
      setLeft(draft);
    },
  ];
}
