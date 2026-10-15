import { t } from "@bugaboo/messages";
import { Overlay, overlayEdge } from "@bugaboo/ui";

import { issueShow } from "./issue-show.ts";
import { IssuePage } from "./IssuePage.tsx";
import { useIssue } from "./issues.ts";
import { useIssueTakesScreen } from "./use-issue-takes-screen.ts";

export interface OpenIssueProps {
  /** The issue that is open next to the list, from the address. */
  readonly open: string;
  /** Opens the issue in the focus view. */
  readonly onFocus: (key: string) => void;
  /** Closes the issue (Esc and a press outside the sheet do this). */
  readonly onClose: () => void;
}

/**
 * The issue that is open next to the list: in the side panel, or, where a tablet is held upright
 * and has no room for a panel next to the list, as a full-height sheet over the list (UX §3.2).
 * The sheet is named after the issue, once it is read (the page of the issue reads it too, and
 * both share the answer).
 */
export function OpenIssue({ open, onFocus, onClose }: OpenIssueProps) {
  const { sheet } = useIssueTakesScreen();
  const found = useIssue(sheet ? open : undefined).data;
  const key = found?.issue.key;
  const page = (
    <IssuePage
      address={open}
      show={issueShow.panel}
      onClose={onClose}
      onSwitch={() => {
        onFocus(open);
      }}
    />
  );
  if (!sheet) {
    return page;
  }
  return (
    <Overlay
      isOpen
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
      label={
        key === undefined || key === null
          ? t("issues.sheet")
          : t("issues.title-of", { key, title: found?.issue.title ?? "" })
      }
      from={overlayEdge.full}
    >
      {page}
    </Overlay>
  );
}
