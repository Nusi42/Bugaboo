import { settingChoices, settingKeys, type SettingKey } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  autoComplete,
  Button,
  ChoiceGroup,
  fieldKind,
  TextField,
  partOf,
  part,
  type FieldKind,
} from "@bugaboo/ui";

import { messageOf } from "../../../shared/errors.ts";
import { settingOf, type MySettings } from "../../../shared/my-settings.ts";
import { wordsOf } from "../../settings/index.ts";
import { useMailServerStatus, useSetMailPassword } from "./use-mail-server.ts";

/**
 * Whether each choice of the protection sends nothing encrypted. Every choice of the registry has to
 * be named here, so a new or moved choice cannot leave the warning on the wrong one.
 */
const sendsInTheClear = {
  starttls: false,
  tls: false,
  off: true,
} as const satisfies Record<(typeof settingChoices.mailSecurity)[number], boolean>;

/** Whether the chosen protection sends nothing encrypted. */
function isInTheClear(choice: string): boolean {
  return settingChoices.mailSecurity.some((each) => each === choice && sendsInTheClear[each]);
}

export interface SettingTextProps {
  readonly settings: MySettings;
  readonly settingKey: SettingKey;
  readonly value: string;
  readonly onChange: (next: string) => void;
  readonly kind?: FieldKind;
  readonly error?: string | undefined;
}

/** A box for one setting: its name and its explanation come from the registry. */
export function SettingText({
  settings,
  settingKey,
  value,
  onChange,
  kind,
  error,
}: SettingTextProps) {
  const found = settingOf(settings, settingKey);
  return (
    <TextField
      label={wordsOf(found?.name_key ?? "")}
      description={wordsOf(found?.help_key ?? "")}
      value={value}
      onChange={onChange}
      autoComplete={autoComplete.off}
      {...(kind === undefined ? {} : { kind })}
      errorMessage={error}
    />
  );
}

/** The password of the mail server: a box for a new one, whether one is set, and a way to remove it. */
export function PasswordField({
  value,
  onChange,
}: {
  readonly value: string;
  readonly onChange: (next: string) => void;
}) {
  const status = useMailServerStatus();
  const remove = useSetMailPassword();
  const set = status.data?.credential_set === true;
  return (
    <div {...partOf(part.stack)}>
      <TextField
        label={t("mail-server.password-new")}
        description={t("mail-server.password-help")}
        kind={fieldKind.password}
        autoComplete={autoComplete.newPassword}
        value={value}
        onChange={onChange}
      />
      <div {...partOf(part.row)}>
        <span {...partOf(set ? part.strong : part.muted)}>
          {set ? t("mail-server.password-set") : t("mail-server.password-none")}
        </span>
        {set ? (
          <Button
            isPending={remove.isPending}
            onPress={() => {
              remove.mutate({ body: {} });
            }}
          >
            {t("mail-server.password-remove")}
          </Button>
        ) : null}
      </div>
      {remove.isError ? <Alert tone={alertTone.danger}>{messageOf(remove.error)}</Alert> : null}
    </div>
  );
}

/** How the connection is protected, with the rule about a user name or a password without it. */
export function Protection({
  settings,
  value,
  onChange,
}: {
  readonly settings: MySettings;
  readonly value: string;
  readonly onChange: (next: string) => void;
}) {
  const found = settingOf(settings, settingKeys.mailSecurity);
  const choices = (found?.choices ?? []).map((choice) => ({
    value: choice.value,
    label: wordsOf(choice.label_key),
  }));
  return (
    <div {...partOf(part.stack)}>
      <ChoiceGroup
        legend={wordsOf(found?.name_key ?? "")}
        choices={choices}
        value={value}
        onChange={onChange}
      />
      <p {...partOf(part.muted)}>{wordsOf(found?.help_key ?? "")}</p>
      {isInTheClear(value) ? (
        <Alert tone={alertTone.warning}>{t("mail-server.no-encryption-note")}</Alert>
      ) : null}
    </div>
  );
}
