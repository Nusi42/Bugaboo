import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { pageSize } from "../../shared/paged.ts";
import { OfferedIn } from "../projects/index.ts";
import { refreshLabels, type LabelGroup } from "./labels.ts";

/** Where the place of creation of a group says it is offered, as a sentence. */
function createdText(group: LabelGroup): string {
  return group.created_in.kind === "project"
    ? t("projects.offered-in-created-project", { project: group.created_in.name })
    : t("projects.offered-in-created-organization");
}

/**
 * *Offered in* for a group of labels (TAG-02): where it was created, the lines that say where it
 * is offered, and the form that adds one. Each line is judged where it is for.
 */
export function GroupOffers({ group }: { readonly group: LabelGroup }) {
  const path = { params: { path: { group: group.id } } } as const;
  const rows = api.query.useQuery("get", "/api/v1/label-groups/{group}/offers", {
    params: { ...path.params, query: { limit: pageSize.largest } },
  });
  const change = api.query.useMutation("put", "/api/v1/label-groups/{group}/offers", {
    onSuccess: () => {
      refreshLabels();
      void rows.refetch();
    },
  });
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.muted)}>{createdText(group)}</p>
      <OfferedIn
        rows={rows.data?.items ?? []}
        isPending={change.isPending}
        error={change.isError ? messageOf(change.error) : null}
        onChange={({ project, offered }) => {
          change.mutate({
            ...path,
            body: {
              version: group.version,
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
