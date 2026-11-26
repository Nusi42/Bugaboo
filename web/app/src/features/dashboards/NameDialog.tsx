import { t } from "@bugaboo/messages";
import { AskDialog, TextField } from "@bugaboo/ui";
import { useState } from "react";

export interface NameDialogProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** The title of the window: *Create a dashboard*, *Save a copy*, *Rename*. */
  readonly title: string;
  /** What the name is for, in a sentence. */
  readonly text: string;
  readonly confirm: string;
  /** The name the box starts with. */
  readonly start: string;
  readonly onConfirm: (name: string) => void;
}

/** Asks for the name of a dashboard: to create it, to save a copy of it, or to rename it. */
export function NameDialog({
  isOpen,
  onOpenChange,
  title,
  text,
  confirm,
  start,
  onConfirm,
}: NameDialogProps) {
  const [name, setName] = useState(start);
  return (
    <AskDialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        setName(start);
        onOpenChange(open);
      }}
      title={title}
      text={text}
      confirmLabel={confirm}
      cancelLabel={t("common.cancel")}
      confirmDisabled={name.trim().length === 0}
      confirmHint={t("dashboards.name-need")}
      onConfirm={() => {
        onConfirm(name.trim());
      }}
    >
      <TextField label={t("dashboards.name")} value={name} onChange={setName} />
    </AskDialog>
  );
}
