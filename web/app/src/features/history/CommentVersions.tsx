import { t } from "@bugaboo/messages";
import { Alert, alertTone, MarkdownView, partOf, part } from "@bugaboo/ui";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { formatDateTime } from "../../shared/format.ts";
import { pageSize } from "../../shared/paged.ts";

/** Every text a comment had, the current one last: shown to whoever may see the comment. */
export function CommentVersions({ comment }: { readonly comment: string }) {
  const versions = api.query.useQuery("get", "/api/v1/comments/{comment}/versions", {
    params: { path: { comment }, query: { limit: pageSize.largest } },
  });
  if (versions.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(versions.error)}</Alert>;
  }
  return (
    <ol {...partOf(part.list)}>
      {(versions.data?.items ?? []).map((each) => (
        <li key={each.version} {...partOf(part.item)}>
          <span {...partOf(part.muted)}>
            {t("comments.version-saved", {
              version: each.version,
              when: formatDateTime(each.saved_at),
            })}
          </span>
          <MarkdownView html={each.html} />
        </li>
      ))}
    </ol>
  );
}
