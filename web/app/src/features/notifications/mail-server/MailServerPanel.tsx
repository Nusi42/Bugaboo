import { settingKeys, type SettingKey } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, Switch, partOf, part, type ButtonLook } from "@bugaboo/ui";
import { type ReactNode } from "react";

import { type MySettings } from "../../../shared/my-settings.ts";
import { QueryGate } from "../../shell/index.ts";
import { useOrganizationSettings, wordsOf } from "../../settings/index.ts";
import { ConnectionForm } from "./ConnectionForm.tsx";
import { type SettingEdits } from "./connection-draft.ts";
import { TestEmail } from "./TestEmail.tsx";

export interface MailServerPanelProps {
  /** How the button that saves the mail server looks. */
  readonly saveLook?: ButtonLook;
}

/** A switch for a yes or no setting of the organization, with its explanation under it. */
function SettingSwitch({
  settings,
  edits,
  settingKey,
}: {
  readonly settings: MySettings;
  readonly edits: SettingEdits;
  readonly settingKey: SettingKey;
}) {
  const found = settings.settings.find((each) => each.key === settingKey);
  if (found === undefined) {
    return null;
  }
  return (
    <div {...partOf(part.column)}>
      <Switch
        label={wordsOf(found.name_key)}
        isSelected={found.value === true}
        onChange={(next) => {
          edits.change({ kind: "change", key: settingKey, value: next });
        }}
      />
      <p {...partOf(part.muted)}>{wordsOf(found.help_key)}</p>
    </div>
  );
}

/** A section of the page with its heading. */
function Section({ title, children }: { readonly title: string; readonly children: ReactNode }) {
  return (
    <section {...partOf(part.formSection)} aria-label={title}>
      <h2 {...partOf(part.formSectionTitle)}>{title}</h2>
      {children}
    </section>
  );
}

/** Whether e-mail is switched on while the mail server is not complete. */
function isIncomplete(settings: MySettings): boolean {
  const value = (key: SettingKey) =>
    String(settings.settings.find((each) => each.key === key)?.value ?? "");
  return value(settingKeys.mailHost).trim() === "" || value(settingKeys.mailFrom).trim() === "";
}

/**
 * Everything about the mail server on one screen part: whether e-mail is on, the connection, *Send me
 * a test e-mail*, and *Short e-mails*. E-mail stays off until an administrator has entered the mail
 * server and switched it on (PRIV-08). Used by *Organization settings* and by the first start.
 */
export function MailServerPanel({ saveLook = buttonLook.primary }: MailServerPanelProps) {
  const { settings, failed, edits } = useOrganizationSettings("mail.");
  return (
    <QueryGate error={failed} data={settings} loading={null}>
      {(shown) => (
        <div {...partOf(part.stack)}>
          <Section title={t("mail-server.sending-title")}>
            <SettingSwitch settings={shown} edits={edits} settingKey={settingKeys.mailEnabled} />
            {shown.settings.some(
              (each) => each.key === settingKeys.mailEnabled && each.value === true,
            ) && isIncomplete(shown) ? (
              <Alert tone={alertTone.warning}>{t("error.mail.off")}</Alert>
            ) : null}
            <p {...partOf(part.muted)}>{t("mail-server.sign-up-note")}</p>
          </Section>
          <Section title={t("mail-server.connection")}>
            <ConnectionForm settings={shown} edits={edits} saveLook={saveLook} />
          </Section>
          <Section title={t("mail-server.test-title")}>
            <TestEmail />
          </Section>
          <Section title={t("mail-server.short-title")}>
            <SettingSwitch settings={shown} edits={edits} settingKey={settingKeys.mailShort} />
          </Section>
        </div>
      )}
    </QueryGate>
  );
}
