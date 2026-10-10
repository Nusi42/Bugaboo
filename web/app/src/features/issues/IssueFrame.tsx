import { useLayout } from "@bugaboo/ui";

import { SidePanel } from "../shell/index.ts";
import { historyId } from "./history-id.ts";
import { moreActions } from "./issue-actions.ts";
import { IssueBar } from "./IssueBar.tsx";
import { IssueContent } from "./IssueContent.tsx";
import { IssueHeader } from "./IssueHeader.tsx";
import type { IssueShow } from "./issue-show.ts";
import type { IssueDetail } from "./slots.ts";
import type { IssuePageState } from "./use-issue-page.ts";

export interface IssueFrameProps extends IssuePageState {
  readonly detail: IssueDetail;
  readonly address: string;
  readonly show: IssueShow;
  readonly onClose: () => void;
  readonly onSwitch: () => void;
}

/** Goes to a part of the page, as the bar of a phone does. */
function goTo(id: string): void {
  document.querySelector(`#${id}`)?.scrollIntoView({ block: "start" });
}

/** Moves the focus to the first list of words (the status) in the facts bar. */
function focusStatus(): void {
  document.querySelector<HTMLElement>("select")?.focus();
}

/**
 * The open issue in its frame: in the focus view it is the page, in the side panel it sits in the
 * panel next to the list, and on a phone the bar of the most used actions stays under it.
 */
export function IssueFrame(props: IssueFrameProps) {
  const { detail, address, show, onClose, onSwitch, changes, blocks } = props;
  const layout = useLayout();
  const { issue } = detail;
  const inside = (
    <>
      <IssueHeader
        issue={issue}
        show={show}
        canChangeTitle={blocks.mask.changes("title")}
        mayDelete={detail.may_delete}
        onTitle={(title) => {
          changes.change({ title });
        }}
        onClose={onClose}
        onSwitch={onSwitch}
      />
      <IssueContent
        detail={detail}
        address={address}
        changes={changes}
        blocks={blocks}
        split={layout === "computer" && show === "focus"}
      />
    </>
  );
  return (
    <article>
      {show === "panel" ? (
        <SidePanel title={blocks.issueKey} onClose={onClose}>
          {inside}
        </SidePanel>
      ) : (
        inside
      )}
      {layout === "phone" ? (
        <IssueBar
          onStatus={focusStatus}
          onComment={() => {
            goTo(historyId);
          }}
          actions={moreActions(issue, detail.may_delete, onClose)}
        />
      ) : null}
    </article>
  );
}
