import { t } from "@bugaboo/messages";
import { Button, buttonLook, Card, cardLook, useLayout, partOf, part } from "@bugaboo/ui";
import { useEffect } from "react";

import { IssueLegend } from "./IssueLegend.tsx";
import { IssueList } from "./IssueList.tsx";
import { openNewIssue, rememberProject } from "./new-issue-state.ts";
import { OpenIssue } from "./OpenIssue.tsx";
import { useOpenIssuesIn } from "./open-in.ts";
import { Selected } from "./Selected.tsx";
import { useSelection } from "./use-selection.ts";

export interface IssuesWithPanelProps {
  readonly shortName: string;
  readonly name: string;
  /** The issue open in the side panel, from the address. */
  readonly open: string | undefined;
  /** Opens an issue in the focus view. */
  readonly onFocus: (key: string) => void;
  /** Closes the side panel. */
  readonly onClose: () => void;
}

/** The list of a project and, beside it, the issue that is open in the side panel. */
export function IssuesWithPanel({ shortName, name, open, onFocus, onClose }: IssuesWithPanelProps) {
  const openIn = useOpenIssuesIn();
  const selection = useSelection();
  // A phone shows one thing at a time: the open issue takes the screen and Back closes it.
  const layout = useLayout();
  const alone = layout === "phone" && open !== undefined;
  useEffect(() => {
    rememberProject(shortName);
  }, [shortName]);
  return (
    <Card heading={t("issues.page-title", { project: name })} level={1} look={cardLook.openOnPhone}>
      <div {...partOf(part.listEditor)}>
        <div {...partOf(part.listEditorList, { states: [alone && "hidden"] })} hidden={alone}>
          <div {...partOf(part.listToolbar)}>
            <Button
              look={buttonLook.primary}
              onPress={() => {
                openNewIssue(shortName);
              }}
            >
              {t("issues.new")}
            </Button>
            <IssueLegend />
          </div>
          <Selected selection={selection} />
          <IssueList project={shortName} openIn={openIn} open={open} selection={selection} />
        </div>
        {open === undefined ? null : <OpenIssue open={open} onFocus={onFocus} onClose={onClose} />}
      </div>
    </Card>
  );
}
