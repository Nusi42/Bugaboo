import { type Schema, dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { percentText } from "../../shared/number.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, personName } from "../../shared/person.ts";
import { refreshWorkTimes } from "../../shared/work-times-reads.ts";

/** A person of a project with their share there, as the server answers it for the reader. */
export type MemberShare = Schema<"MemberShareBody">;

/** The people of a project with their shares, a page at a time. */
export function useProjectShares(project: string) {
  return usePaged({
    // The key the client makes for the route, so a change of work times reads the list again.
    queryKey: api.query.queryOptions("get", "/api/v1/projects/{project}/shares", {
      params: { path: { project } },
    }).queryKey,
    read: (after) =>
      api.client.GET("/api/v1/projects/{project}/shares", {
        params: { path: { project }, query: { limit: pageSize.usual, ...after } },
      }),
  });
}

/** Sets the share of a person in a project. */
async function setShare(project: string, person: string, percent: number): Promise<void> {
  dataOf(
    await api.client.PUT("/api/v1/projects/{project}/shares/{person}", {
      params: { path: { project, person } },
      body: { percent },
    }),
  );
}

/** What *Project members* does with a share: set it in place with *Undo*, and what the server refused. */
export interface ShareChanges {
  readonly set: (member: MemberShare, percent: number) => void;
  /** The plain message of the last refusal, with the person it was about; `null` when nothing was refused. */
  readonly problem: { readonly person: string; readonly message: string } | null;
}

/**
 * Setting shares (PLAN-15): a share saves in place and offers *Undo* for 10 seconds (R7), which sets
 * the share it had. The server decides who may set it and what a share may be.
 */
export function useShareChanges(project: string): ShareChanges {
  const [problem, setProblem] = useState<ShareChanges["problem"]>(null);
  const put = (member: MemberShare, percent: number, back?: number): void => {
    const person = namedPerson(member.person)?.id;
    if (person === undefined) {
      return;
    }
    setProblem(null);
    setShare(project, person, percent)
      .then(() => {
        refreshWorkTimes();
        if (back !== undefined) {
          undoStore.offer({
            message: t("project-members.share-set", {
              name: personName(member.person),
              percent: percentText(percent),
            }),
            undo: () => {
              put(member, back);
            },
          });
        }
      })
      .catch((error: unknown) => {
        setProblem({ person, message: messageOf(error) });
      });
  };
  return {
    set: (member, percent) => {
      put(member, percent, member.percent);
    },
    problem,
  };
}
