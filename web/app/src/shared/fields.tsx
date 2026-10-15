import { t } from "@bugaboo/messages";
import { autoComplete, fieldKind, TextField } from "@bugaboo/ui";

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
      requiredMessage={t("error.field.required")}
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
      kind={fieldKind.email}
      autoComplete={autoComplete.email}
      requiredMessage={t("error.field.required")}
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
      kind={fieldKind.password}
      autoComplete={autoComplete.newPassword}
      requiredMessage={t("error.field.required")}
      errorMessage={problem}
      value={value}
      onChange={onChange}
    />
  );
}
