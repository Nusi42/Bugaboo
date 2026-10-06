import { t } from "@bugaboo/messages";
import { useState, type ReactNode } from "react";

import { api } from "../../shared/api.ts";
import { useAskFirst } from "../../shared/ask-first.tsx";
import { messageOf } from "../../shared/errors.ts";
import { removeKeepingFocus } from "../../shared/focus.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { rememberChoice } from "../../shared/recent-choices.ts";
import { refreshAccess } from "../../shared/access-cache.ts";
import type { GroupLine, PersonLine } from "./groups.ts";

/** What changing the members of one group gives a screen. */
export interface MemberChanges {
  /** Puts a person into the group; offers *Undo*. */
  readonly add: (person: PersonLine) => void;
  /** Takes a person out of the group; offers *Undo*. */
  readonly remove: (person: PersonLine) => void;
  /** The plain message when the server refused, else `null`. */
  readonly problem: string | null;
  /** The question that comes before taking a person out of the group that gives them their own way to manage roles. */
  readonly dialog: ReactNode;
}

/** Puts a person into a group (`true`) or takes them out (`false`). */
async function put(group: string, person: string, into: boolean): Promise<void> {
  const params = { params: { path: { group_id: group, person_id: person } } };
  if (into) {
    await api.client.PUT("/api/v1/groups/{group_id}/members/{person_id}", params);
  } else {
    await api.client.DELETE("/api/v1/groups/{group_id}/members/{person_id}", params);
  }
  refreshAccess();
}

/**
 * Putting people into a group and taking them out (SEC-02): each happens at once and offers *Undo*
 * for 10 seconds (UX §6), which does the opposite. The server decides every time, so *Undo* can be
 * refused like anything else (somebody may not put a person into a group that holds more than they
 * hold themselves); the screen then shows why.
 */
export function useMemberChanges(group: GroupLine): MemberChanges {
  const [problem, setProblem] = useState<string | null>(null);
  const { ask, dialog } = useAskFirst();
  const change = (person: PersonLine, into: boolean): void => {
    setProblem(null);
    put(group.id, person.id, into)
      .then(() => {
        undoStore.offer({
          message: t(into ? "groups.added" : "groups.removed", {
            name: person.name,
            group: group.name,
          }),
          undo: () => {
            put(group.id, person.id, !into).catch((error: unknown) => {
              setProblem(messageOf(error));
            });
          },
        });
        if (into) {
          rememberChoice("person", person.id);
        }
      })
      .catch((error: unknown) => {
        setProblem(messageOf(error));
      });
  };
  return {
    add: (person) => {
      change(person, true);
    },
    remove: (person) => {
      void ask({ kind: "leave-group", group: group.id, person: person.id }).then((go) => {
        if (go) {
          removeKeepingFocus(() => {
            change(person, false);
          });
        }
      });
    },
    problem,
    dialog,
  };
}
