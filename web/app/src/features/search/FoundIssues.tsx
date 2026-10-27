import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Link, part, partOf } from "@bugaboo/ui";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { issueAddress } from "../../shared/paths.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The first issues that a search finds are listed here until the list of WP-08 takes the place. */
const shown = 50;

export interface FoundIssuesProps {
  /** The canonical text of the last search that was fixed. */
  readonly text: string;
  /** The ID of the project the search looks in. */
  readonly within?: string | undefined;
  /** The text in the box has a mistake: the results of the last fixed search stay, dimmed. */
  readonly stale: boolean;
}

/**
 * What a search finds: the number and the first issues. The results of the last search that was
 * fixed stay while the text has a mistake, dimmed, until it is fixed (UX design section 4.2).
 */
export function FoundIssues({ text, within, stale }: FoundIssuesProps) {
  const query = useQuery({
    queryKey: queryKeys.search.issues(text, within),
    placeholderData: keepPreviousData,
    queryFn: async (): Promise<Schema<"Page_IssueItem">> =>
      dataOf(
        await api.client.GET("/api/v1/search/issues", {
          params: {
            query: {
              ...(text === "" ? {} : { q: text }),
              ...(within === undefined ? {} : { within }),
              limit: shown,
              fields: "readable_id,title",
            },
          },
        }),
      ),
  });
  if (query.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(query.error)}</Alert>;
  }
  const items = query.data?.items ?? [];
  return (
    <div
      {...partOf(part.searchResults, { states: [stale && "stale"] })}
      aria-busy={query.isFetching}
    >
      {stale ? <output {...partOf(part.text)}>{t("search.found-stale")}</output> : null}
      {query.data === undefined ? null : items.length === 0 ? (
        <p {...partOf(part.text)}>{t("search.found-none")}</p>
      ) : (
        <ul {...partOf(part.searchList)} aria-label={t("search.found-list")}>
          {items.map((issue, index) => (
            <li key={issue.readable_id ?? index} {...partOf(part.searchListItem)}>
              {issue.readable_id === undefined || issue.readable_id === null ? null : (
                <Link href={issueAddress(issue.readable_id)}>
                  {t("search.found-issue", { id: issue.readable_id, title: issue.title ?? "" })}
                </Link>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
