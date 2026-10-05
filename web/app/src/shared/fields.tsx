import { t } from "@bugaboo/messages";
import { autoComplete, TextField } from "@bugaboo/ui";

export interface FieldProps {
  readonly value: string;
  readonly onChange: (value: string) => void;
}

/** The name other people see. */
export function NameField({ value, onChange }: FieldProps) {
  return (
    <TextField
      label={t("field.name")}
      description={t("field.name-help")}
      autoComplete={autoComplete.name}
      requiredMessage={t("validation.required")}
      value={value}
      onChange={onChange}
    />
  );
}

/** The e-mail address that is used to sign in. */
export function EmailField({ value, onChange }: FieldProps) {
  return (
    <TextField
      label={t("field.email")}
      email
      autoComplete={autoComplete.email}
      requiredMessage={t("validation.required")}
      emailMessage={t("validation.email")}
      value={value}
      onChange={onChange}
    />
  );
}

export interface NewPasswordFieldProps extends FieldProps {
  /** What the server found wrong with the password: too short, or too common. */
  readonly problem?: string | undefined;
}

/** A password to choose, with the rule in words. */
export function NewPasswordField({ value, onChange, problem }: NewPasswordFieldProps) {
  return (
    <TextField
      label={t("field.password")}
      description={t("field.password-help")}
      password
      autoComplete={autoComplete.newPassword}
      requiredMessage={t("validation.required")}
      errorMessage={problem}
      value={value}
      onChange={onChange}
    />
  );
}
