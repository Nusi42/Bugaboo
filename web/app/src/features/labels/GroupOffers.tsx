import { partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { pageSize } from "../../shared/paged.ts";
import { OfferedIn, offerChangeBody } from "../projects/index.ts";
import { refreshLabels, type LabelGroup } from "./labels.ts";

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
      <OfferedIn
        createdIn={group.created_in}
        rows={rows.data?.items ?? []}
        isPending={change.isPending}
        error={change.isError ? messageOf(change.error) : null}
        onChange={({ project, offered }) => {
          change.mutate({ ...path, body: offerChangeBody(group.version, project, offered) });
        }}
      />
    </div>
  );
}
