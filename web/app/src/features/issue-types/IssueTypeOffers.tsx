import { partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { pageSize } from "../../shared/paged.ts";
import { refreshIssueTypes } from "../../shared/query-keys.ts";
import { OfferedIn } from "../projects/index.ts";
import type { IssueTypeLine } from "./issue-types.ts";

/**
 * *Offered in* for an issue type (ISS-02): where it was created, the lines that say where it is
 * offered, and the form that adds one. Each line is judged where it is for, so a Project lead can
 * switch a type of the organization off in their own project and nowhere else.
 */
export function IssueTypeOffers({ issueType }: { readonly issueType: IssueTypeLine }) {
  const path = { params: { path: { issue_type: issueType.id } } } as const;
  const rows = api.query.useQuery("get", "/api/v1/issue-types/{issue_type}/offers", {
    params: { ...path.params, query: { limit: pageSize.largest } },
  });
  const change = api.query.useMutation("put", "/api/v1/issue-types/{issue_type}/offers", {
    onSuccess: () => {
      refreshIssueTypes();
      void rows.refetch();
    },
  });
  return (
    <div {...partOf(part.stack)}>
      <OfferedIn
        createdIn={issueType.createdIn}
        rows={rows.data?.items ?? []}
        isPending={change.isPending}
        error={change.isError ? messageOf(change.error) : null}
        onChange={({ project, offered }) => {
          change.mutate({
            ...path,
            body: {
              version: issueType.version,
              changes: [
                {
                  ...(project === null ? {} : { project }),
                  ...(offered === null ? {} : { offered }),
                },
              ],
            },
          });
        }}
      />
    </div>
  );
}
