import { t } from "@bugaboo/messages";
import { Button, buttonLook, Window, partOf, part } from "@bugaboo/ui";

import { windowLabels } from "../../shared/window-labels.ts";
import type { DraftChange } from "./draft-model.ts";
import { RulesFields } from "./RulesFields.tsx";
import type { ChangeRules } from "./rules-model.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface RulesWindowProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
  readonly editor: WorkflowEditor;
  readonly change: DraftChange;
  /** The names of the statuses the change goes from and to, for the sentence at the top. */
  readonly from: string;
  readonly to: string;
  /** Called with the rules at every change: they are part of the draft at once. */
  readonly onChange: (rules: ChangeRules) => void;
}

/**
 * The rules of one status change in plain words (WF-03, UX §5.4): *Who can do it*, *Only if …*,
 * *Must be filled in* and *Ask for*. A form opens as a window (R10). Every change goes into the
 * draft at once; *Keep these rules* only closes the window, and the change counts after *Publish*.
 */
export function RulesWindow({
  isOpen,
  onOpenChange,
  editor,
  change,
  from,
  to,
  onChange,
}: RulesWindowProps) {
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={t("workflows.rules-title", { button: change.button })}
      labels={windowLabels()}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button
            look={buttonLook.primary}
            onPress={() => {
              onOpenChange(false);
            }}
          >
            {t("workflows.rules-done")}
          </Button>
        </div>
      }
    >
      <div {...partOf(part.stack)}>
        <p {...partOf(part.muted)}>{t("workflows.rules-intro", { from, to })}</p>
        <RulesFields editor={editor} rules={change.rules} onChange={onChange} />
      </div>
    </Window>
  );
}
