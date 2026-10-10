import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { ConflictNote } from "./ConflictNote.tsx";
import { FactsBar } from "./FactsBar.tsx";
import { FieldsPanel } from "./FieldsPanel.tsx";
import { IssueMain } from "./IssueMain.tsx";
import { IssueSaved } from "./IssueSaved.tsx";
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
 * Everything under the title of an issue: what was saved, a conflict, the facts bar, and the body
 * with the description and the details (UX §4.4). The details are a column of their own when the
 * screen has room, and sit behind *All details* otherwise.
 */
export function IssueContent({ detail, address, changes, blocks, split }: IssueContentProps) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <IssueSaved changes={changes} />
      {changes.conflict === null ? null : <ConflictNote conflict={changes.conflict} />}
      <FactsBar issue={detail.issue} mask={blocks.mask} onChange={changes.change} />
      <div {...partOf(part.issueBody, { variants: [!split && "stacked"] })}>
        <IssueMain detail={detail} address={address} blocks={blocks} />
        {split || open ? <FieldsPanel blocks={blocks} stacked={!split} /> : null}
        {split ? null : (
          <Button
            onPress={() => {
              setOpen(!open);
            }}
          >
            {open ? t("issues.hide-details") : t("issues.all-details")}
          </Button>
        )}
      </div>
    </>
  );
}
