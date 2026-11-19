import { t } from "@bugaboo/messages";
import { Alert, alertTone, autoComplete, fieldKind, TextField } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";

export interface PasswordAgainProps {
  /** What the password is asked for, under the field. */
  readonly help: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** What the server answered when the request failed; none before or after a success. */
  readonly failure: unknown;
}

/**
 * The password asked again before a key is made or an approval gives agents (architecture §12),
 * and what went wrong: the one field of a key, of several agents and of an approval.
 */
export function PasswordAgain({ help, value, onChange, failure }: PasswordAgainProps) {
  return (
    <>
      <TextField
        label={t("field.confirm-password")}
        description={help}
        kind={fieldKind.password}
        autoComplete={autoComplete.currentPassword}
        requiredMessage={t("error.field.required")}
        value={value}
        onChange={onChange}
      />
      {failure === null || failure === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(failure)}</Alert>
      )}
    </>
  );
}
