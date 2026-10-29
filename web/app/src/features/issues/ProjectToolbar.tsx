import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf, useLayout } from "@bugaboo/ui";
import { useEffect } from "react";

import { LegendButton } from "./LegendButton.tsx";
import { IssueLegend } from "./IssueLegend.tsx";
import { openNewIssue, rememberProject } from "./new-issue-state.ts";

export interface ProjectToolbarProps {
  readonly shortName: string;
  /** An issue is open beside the list, which leaves the toolbar little room. */
  readonly beside: boolean;
}

/** *New issue* for the project. */
function NewIssue({ shortName }: { readonly shortName: string }) {
  return (
    <Button
      look={buttonLook.primary}
      onPress={() => {
        openNewIssue(shortName);
      }}
    >
      {t("issues.new")}
    </Button>
  );
}

/**
 * What a phone has of the toolbar, in the row of *Filters* (it has one row of three buttons:
 * *New issue*, *Legend* and *Filters*): the search bar draws it first.
 */
export function PhoneActions({ shortName }: { readonly shortName: string }) {
  return (
    <>
      <NewIssue shortName={shortName} />
      <LegendButton />
    </>
  );
}

/**
 * The row above a project's list on a computer or a tablet: *New issue* with the legend of the colors
 * of the rows at the end (a *Legend* button where it has no room). A phone has it in the row of its
 * search bar. The project is remembered for the next new issue.
 */
export function ProjectToolbar({ shortName, beside }: ProjectToolbarProps) {
  const layout = useLayout();
  useEffect(() => {
    rememberProject(shortName);
  }, [shortName]);
  if (layout === "phone") {
    return null;
  }
  return (
    <div {...partOf(part.listToolbar)}>
      <NewIssue shortName={shortName} />
      {layout === "computer" && !beside ? <IssueLegend /> : <LegendButton />}
    </div>
  );
}
