import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { IssuePicker } from "../search/index.ts";
import type { IssuesProps } from "./sheet-props.ts";

/** *Add an issue*: the finder of issues by ID or by words of the title opens in the place of the button. */
export function AddIssue({ onAdd, addProblem }: Pick<IssuesProps, "onAdd" | "addProblem">) {
  const [open, setOpen] = useState(false);
  const problem =
    addProblem === undefined ? null : <span {...partOf(part.sheetProblem)}>{addProblem}</span>;
  if (!open) {
    return (
      <span {...partOf(part.sheetAdd)}>
        <Button
          look={buttonLook.quiet}
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("time.sheet.add-issue")}
        </Button>
        {problem}
      </span>
    );
  }
  return (
    <IssuePicker
      onPick={(key) => {
        onAdd(key);
        setOpen(false);
      }}
    />
  );
}
