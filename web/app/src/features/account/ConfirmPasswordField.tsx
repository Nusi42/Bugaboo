import { t } from "@bugaboo/messages";
import { autoComplete, TextField } from "@bugaboo/ui";

export interface ConfirmPasswordFieldProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/** The password, typed again before something that could lock the person out or let a thief in. */
export function ConfirmPasswordField({ value, onChange }: ConfirmPasswordFieldProps) {
  return (
    <TextField
      label={t("field.confirm-password")}
      description={t("field.confirm-password-help")}
      password
      autoComplete={autoComplete.currentPassword}
      requiredMessage={t("validation.required")}
      value={value}
      onChange={onChange}
    />
  );
}
