import { useState } from "react";

import { buttonLook, ConfirmDialog, type ConfirmDialogProps } from "@bugaboo/ui";

import { ConfirmPasswordField } from "./ConfirmPasswordField.tsx";

export interface RemoveWithPasswordProps extends Omit<
  ConfirmDialogProps,
  "children" | "confirmDisabled" | "onClosed" | "onConfirm"
> {
  /** Called with the password when the person confirms. */
  readonly onConfirm: (password: string) => void;
}

/** A question that wants the password typed again before it is answered with yes. */
export function RemoveWithPassword({ onConfirm, ...question }: RemoveWithPasswordProps) {
  const [password, setPassword] = useState("");
  const empty = password.length === 0;
  return (
    <ConfirmDialog
      {...question}
      confirmLook={buttonLook.danger}
      confirmDisabled={empty}
      onClosed={() => {
        setPassword("");
      }}
      onConfirm={() => {
        onConfirm(password);
      }}
    >
      <ConfirmPasswordField value={password} onChange={setPassword} />
    </ConfirmDialog>
  );
}
