import { PaneWithHead, paneLook, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { DetailsToggle } from "./DetailsToggle.tsx";
import { DetailsPanel } from "./DetailsPanel.tsx";
import { IssueMain } from "./IssueMain.tsx";
import { IssueSaved, IssueSaveState } from "./IssueSaved.tsx";
import { StatusChange } from "./StatusChange.tsx";
import type { IssueBlockProps, IssueDetail } from "./slots.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

export interface IssueContentProps {
  readonly detail: IssueDetail;
  readonly address: string;
  readonly changes: IssueChanges;
  /** The blocks of other packages get these. */
  readonly blocks: IssueBlockProps;
  /** The details are a column next to the main column (a computer, in the focus view). */
  readonly split: boolean;
  /** The top of the issue above the status: its type, ID, title and actions, and a refusal of them. */
  readonly head: ReactNode;
}

/**
 * The issue under its head: the head with the big status button and what was saved, and the body with the
 * description and the details (UX §4.4). The details are a column of their own when the screen has room, and then the
 * head folds while the sections or the details scroll (UX §6.5); otherwise they sit under the status behind *All
 * details*, which says who it is assigned to and the priority while it is closed (UX §3.3).
 */
export function IssueContent({ detail, address, changes, blocks, split, head }: IssueContentProps) {
  const [open, setOpen] = useState(false);
  const details = <DetailsPanel blocks={blocks} changes={changes} stacked={!split} />;
  const top = (
    <>
      {head}
      <StatusChange
        blocks={blocks}
        changes={changes}
        saved={<IssueSaveState changes={changes} />}
      />
      <IssueSaved changes={changes} />
    </>
  );
  const body = (
    <div {...partOf(part.issueBody, { variants: [split ? "split" : "stacked"] })}>
      {split ? null : (
        <DetailsToggle
          issue={detail.issue}
          open={open}
          onToggle={() => {
            setOpen(!open);
          }}
        />
      )}
      {!split && open ? details : null}
      <IssueMain detail={detail} address={address} blocks={blocks} />
      {split ? details : null}
    </div>
  );
  // One tree whether the head folds or not, so what is typed in the issue stays when the window changes its layout.
  return (
    <PaneWithHead head={top} look={split ? paneLook.folds : paneLook.still}>
      {body}
    </PaneWithHead>
  );
}
