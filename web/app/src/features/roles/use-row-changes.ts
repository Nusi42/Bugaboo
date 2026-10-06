import { t } from "@bugaboo/messages";
import { useState } from "react";

import { refreshAccess } from "../../shared/access-cache.ts";
import { appliesToText, roleText } from "../../shared/access-words.ts";
import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { removeKeepingFocus } from "../../shared/focus.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import type { RowLine, Subject } from "./assignments.ts";

/** A row to give: to whom, which role (or *No access*), and where it counts. */
export interface RowSpec {
  readonly who: Subject;
  /** The ID of the role; `null` for *No access*. */
  readonly role: string | null;
  readonly appliesTo: "project" | "project-and-sub-projects";
}

/** What changing the rows of one place gives a screen. */
export interface RowChanges {
  /** Gives a row. Rejects with the error of the server. */
  readonly give: (spec: RowSpec) => Promise<void>;
  /** Takes a row away; offers *Undo*. */
  readonly remove: (row: RowLine) => void;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
}

/** The person or group of a body. */
function whoOf(who: Subject): { readonly person?: string; readonly group?: string } {
  return who.kind === "person" ? { person: who.id } : { group: who.id };
}

/** Gives a row for the whole organization (`project` is `null`) or for a project. */
async function giveRow(project: string | null, spec: RowSpec): Promise<void> {
  if (project === null) {
    if (spec.role === null) {
      return;
    }
    await api.client.POST("/api/v1/assignments", {
      body: { who: whoOf(spec.who), role: spec.role },
    });
  } else {
    await api.client.POST("/api/v1/projects/{project}/assignments", {
      params: { path: { project } },
      body: {
        who: whoOf(spec.who),
        applies_to: spec.appliesTo,
        ...(spec.role === null ? {} : { role: spec.role }),
      },
    });
  }
  refreshAccess();
}

/** What a row says, for the sentences about taking it away. */
function sentenceOf(row: RowLine): { readonly who: string; readonly what: string } {
  return {
    who: row.who.name,
    what: `${roleText(row.role)}, ${appliesToText(row.appliesTo)}`,
  };
}

/** The spec that gives a row again: what *Undo* of taking it away does. */
function specOf(row: RowLine): RowSpec {
  return {
    who: row.who,
    role: row.role === null ? null : row.role.id,
    appliesTo: row.appliesTo === "project" ? "project" : "project-and-sub-projects",
  };
}

/**
 * Giving rows and taking them away in one place: the whole organization (`project` is `null`) or a
 * project (its ID). Taking a row away happens at once and offers *Undo* for 10 seconds (UX §6),
 * which gives the row again; the server decides every time, so *Undo* can be refused too.
 */
export function useRowChanges(project: string | null): RowChanges {
  const [problem, setProblem] = useState<string | null>(null);
  const fail = (error: unknown): void => {
    setProblem(messageOf(error));
  };
  const remove = (row: RowLine): void => {
    setProblem(null);
    removeKeepingFocus(() => {
      api.client
        .DELETE("/api/v1/assignments/{assignment_id}", {
          params: { path: { assignment_id: row.id } },
        })
        .then(() => {
          refreshAccess();
          undoStore.offer({
            message: t("assignments.removed", sentenceOf(row)),
            undo: () => {
              giveRow(project, specOf(row)).catch(fail);
            },
          });
        })
        .catch(fail);
    });
  };
  return {
    give: async (spec) => {
      setProblem(null);
      await giveRow(project, spec);
    },
    remove,
    problem,
  };
}
