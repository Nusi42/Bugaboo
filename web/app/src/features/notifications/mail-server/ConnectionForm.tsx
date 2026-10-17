import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonType,
  fieldKind,
  Form,
  SaveStatus,
  partOf,
  part,
  type ButtonLook,
} from "@bugaboo/ui";

import { messageOf } from "../../../shared/errors.ts";
import { type MySettings } from "../../../shared/my-settings.ts";
import { saveLabels } from "../../../shared/save-labels.ts";
import { PasswordField, Protection, SettingText } from "./ConnectionFields.tsx";
import type { SettingEdits } from "./connection-draft.ts";
import { useConnectionDraft } from "./use-connection-draft.ts";

export interface ConnectionFormProps {
  readonly settings: MySettings;
  readonly edits: SettingEdits;
  /** How the button that saves looks: the main button of the screen, or a plain one where another is. */
  readonly saveLook: ButtonLook;
}

/**
 * The connection to the mail server: host, port, how it is protected, the user name, the password
 * and the address e-mails come from. They are saved together; the password goes on its own and is
 * never read back, only whether it is set (SEC-16).
 */
export function ConnectionForm({ settings, edits, saveLook }: ConnectionFormProps) {
  const { draft, field, problem, save, passwordProblem } = useConnectionDraft(settings, edits);
  const text = { settings };
  return (
    <Form onSubmit={save}>
      <div {...partOf(part.mailServerFields)}>
        <SettingText
          {...text}
          settingKey={settingKeys.mailHost}
          value={draft.host}
          onChange={field("host")}
        />
        <SettingText
          {...text}
          settingKey={settingKeys.mailPort}
          value={draft.port}
          onChange={field("port")}
          error={problem}
        />
        <SettingText
          {...text}
          settingKey={settingKeys.mailUsername}
          value={draft.username}
          onChange={field("username")}
        />
        <SettingText
          {...text}
          settingKey={settingKeys.mailFrom}
          value={draft.from}
          onChange={field("from")}
          kind={fieldKind.email}
        />
      </div>
      <PasswordField value={draft.password} onChange={field("password")} />
      <Protection settings={settings} value={draft.security} onChange={field("security")} />
      <div {...partOf(part.row)}>
        <Button type={buttonType.submit} look={saveLook}>
          {t("mail-server.save")}
        </Button>
        <SaveStatus state={edits.saveState} labels={saveLabels()} />
      </div>
      {edits.problem === null ? null : <Alert tone={alertTone.danger}>{edits.problem}</Alert>}
      {passwordProblem === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(passwordProblem)}</Alert>
      )}
    </Form>
  );
}
