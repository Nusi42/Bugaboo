import { Alert, alertTone, useLayout, partOf, part } from "@bugaboo/ui";

import { SidePanel, useFocusViewOpen } from "../shell/index.ts";
import { useMoreActions } from "./issue-actions.ts";
import { IssueContent } from "./IssueContent.tsx";
import { IssueHeader } from "./IssueHeader.tsx";
import { PhoneBar } from "./PhoneBar.tsx";
import { useCloseLabel } from "./use-close-label.ts";
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

/**
 * The open issue in its frame: in the focus view it is the page, in the side panel it sits in the
 * panel next to the list, and on a phone the bar of the most used actions stays under it.
 */
export function IssueFrame(props: IssueFrameProps) {
  const { detail, address, show, onClose, onSwitch, changes, blocks } = props;
  const layout = useLayout();
  // An issue that is read, in the focus view, has the bar of its own: a phone has no top bar over it.
  useFocusViewOpen(show === "focus");
  const closeLabel = useCloseLabel();
  const { issue } = detail;
  const { actions, refusal } = useMoreActions(issue, detail.may_delete, onClose);
  const inside = (
    <div {...partOf(part.issuePage)}>
      <IssueHeader
        issue={issue}
        show={show}
        canChangeTitle={blocks.mask.changes("title")}
        actions={actions}
        onTitle={(title) => {
          changes.change({ title });
        }}
        onSwitch={onSwitch}
      />
      {refusal === null ? null : <Alert tone={alertTone.danger}>{refusal}</Alert>}
      <IssueContent
        detail={detail}
        address={address}
        changes={changes}
        blocks={blocks}
        split={layout === "computer" && show === "focus"}
      />
    </div>
  );
  return (
    <article>
      {show === "panel" ? (
        <SidePanel
          title={blocks.readableId}
          onClose={onClose}
          {...(closeLabel === undefined ? {} : { closeLabel })}
        >
          {inside}
        </SidePanel>
      ) : (
        inside
      )}
      {layout === "phone" ? <PhoneBar actions={actions} /> : null}
    </article>
  );
}
