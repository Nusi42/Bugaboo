import { t } from "@bugaboo/messages";
import { useLayout } from "@bugaboo/ui";
import { useMemo } from "react";

import { useVersionWatch } from "../../shared/editing/index.ts";
import { useFormLayout } from "../layouts/index.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { readAgain } from "./issue-requests.ts";
import { valuesOf } from "./issue-values.ts";
import { maskOf, type IssueBlockProps, type IssueDetail } from "./slots.ts";
import { useIssueChanges, type IssueChanges } from "./use-issue-changes.ts";

/** What the parts of an open issue share: how it is changed, and what the blocks of other packages get. */
export interface IssuePageState {
  readonly changes: IssueChanges;
  readonly blocks: IssueBlockProps;
}

/**
 * The state of an open issue: the changes of short values, the blocks for other packages, the title
 * of the browser tab, and the check every 30 seconds and on return to the tab (ADR-0019) that reads
 * the issue again when somebody else changed it.
 */
export function useIssuePage(detail: IssueDetail, address: string): IssuePageState {
  const layout = useLayout();
  const changes = useIssueChanges(detail, address);
  const { issue } = detail;
  const form = useFormLayout({
    project: issue.project?.short_name,
    issueType: issue.issue_type?.id,
    kind: "issue-page",
  });
  const key = issue.key ?? address;
  const version = issue.version ?? 1;
  const watched = useMemo(
    () => [{ record: { kind: "issue", id: issue.id ?? address }, version }],
    [issue.id, address, version],
  );
  useVersionWatch(watched, () => {
    readAgain(address);
  });
  usePageTitle(t("issues.title-of", { key, title: issue.title ?? "" }));
  return {
    changes,
    blocks: {
      issue,
      address,
      issueKey: key,
      ownFieldsHtml: detail.own_fields_html ?? [],
      mask: maskOf(detail),
      form,
      version,
      onChange: (change) => {
        changes.change(valuesOf(change));
        return Promise.resolve();
      },
      layout: layout === "computer" ? "wide" : "narrow",
    },
  };
}
