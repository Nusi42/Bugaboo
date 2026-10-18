import { t } from "@bugaboo/messages";
import { Button, buttonLook, StatusPill, TextField, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import {
  nameOf,
  removeChange,
  setButton,
  setRules,
  type Draft,
  type DraftChange,
} from "./draft-model.ts";
import { RulesWindow } from "./RulesWindow.tsx";
import { hasRules } from "./rules-model.ts";
import type { Edit } from "./use-draft-workflow.ts";
import type { WorkflowEditor } from "./workflows.ts";

export interface ChangeRowProps {
  readonly editor: WorkflowEditor;
  readonly draft: Draft;
  readonly change: DraftChange;
  readonly edit: Edit;
}

/** *Rules* and, when the change has some, the words that say so. */
function RulesCell({
  change,
  onOpen,
}: {
  readonly change: DraftChange;
  readonly onOpen: () => void;
}) {
  return (
    <div {...partOf(part.workflowChangeRules)}>
      <Button
        look={buttonLook.quiet}
        aria-label={t("workflows.rules-open", { button: change.button })}
        onPress={onOpen}
      >
        {t("workflows.rules-link")}
      </Button>
      {hasRules(change.rules) ? (
        <span {...partOf(part.muted)}>{t("workflows.has-rules")}</span>
      ) : null}
    </div>
  );
}

/**
 * One status change in the list: where it leads, the words of its button, a way to open its rules,
 * and a way to take it away. Clicking *Rules* opens them in plain words (UX §5.4).
 */
export function ChangeRow({ editor, draft, change, edit }: ChangeRowProps) {
  const [open, setOpen] = useState(false);
  const [from, to] = [nameOf(draft, change.from), nameOf(draft, change.to)];
  const target = draft.statuses.find((each) => each.id === change.to);
  return (
    <li {...partOf(part.workflowChange)}>
      <span {...partOf(part.workflowChangeTo)}>
        {target === undefined ? to : <StatusPill group={target.group} label={target.name} />}
      </span>
      <div {...partOf(part.workflowChangeField)}>
        <TextField
          label={t("workflows.button-text", { from, to })}
          value={change.button}
          onChange={(button) => {
            edit((before) => setButton(before, change.id, button));
          }}
        />
      </div>
      <RulesCell
        change={change}
        onOpen={() => {
          setOpen(true);
        }}
      />
      <div {...partOf(part.workflowChangeRemove)}>
        <Button
          look={buttonLook.quiet}
          aria-label={t("workflows.change-remove", { from, to })}
          onPress={() => {
            edit((before) => removeChange(before, change.id));
          }}
        >
          {t("workflows.remove")}
        </Button>
      </div>
      <RulesWindow
        isOpen={open}
        onOpenChange={setOpen}
        editor={editor}
        change={change}
        from={from}
        to={to}
        onChange={(rules) => {
          edit((before) => setRules(before, change.id, rules));
        }}
      />
    </li>
  );
}
