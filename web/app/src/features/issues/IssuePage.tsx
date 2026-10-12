import { NotFoundPage, QueryGate } from "../shell/index.ts";
import { IssueFrame } from "./IssueFrame.tsx";
import type { IssueShow } from "./issue-show.ts";
import { useIssue } from "./issues.ts";
import type { IssueDetail } from "./slots.ts";
import { useIssuePage } from "./use-issue-page.ts";

export interface IssuePageProps {
  /** The readable or internal ID the page was opened with. */
  readonly address: string;
  /** In the focus view, or in the side panel next to the list. */
  readonly show: IssueShow;
  /** Close the panel, or go back to the list. */
  readonly onClose: () => void;
  /** Open it in the other view. */
  readonly onSwitch: () => void;
}

/** The issue once it is read. */
function Loaded(props: IssuePageProps & { readonly detail: IssueDetail }) {
  const page = useIssuePage(props.detail, props.address);
  return <IssueFrame {...props} {...page} />;
}

/**
 * An issue open in the focus view or the side panel (UX §3.4, §4.4): read by its ID, with what the
 * person may do with it. An issue the person may not see is *Not found*, like one that does not
 * exist.
 */
export function IssuePage(props: IssuePageProps) {
  const found = useIssue(props.address);
  return (
    <QueryGate error={found.error} data={found.data} notFound={<NotFoundPage />}>
      {(detail) => <Loaded {...props} detail={detail} />}
    </QueryGate>
  );
}
