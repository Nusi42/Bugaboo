import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { DetailsToggle } from "./DetailsToggle.tsx";
import { FieldsPanel } from "./FieldsPanel.tsx";
import { IssueMain } from "./IssueMain.tsx";
import { IssueSaved } from "./IssueSaved.tsx";
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
}

/**
 * Everything under the title of an issue: the big status button, what was saved, a conflict, and
 * the body with the description and the details (UX §4.4). The details are a column of their own
 * when the screen has room; otherwise they sit under the status behind *All details*, which says
 * who it is assigned to and the priority while it is closed (UX §3.3).
 */
export function IssueContent({ detail, address, changes, blocks, split }: IssueContentProps) {
  const [open, setOpen] = useState(false);
  const details = <FieldsPanel blocks={blocks} changes={changes} stacked={!split} />;
  return (
    <>
      <StatusChange blocks={blocks} changes={changes} />
      <IssueSaved changes={changes} />
      <div {...partOf(part.issueBody, { variants: [!split && "stacked"] })}>
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
    </>
  );
}
