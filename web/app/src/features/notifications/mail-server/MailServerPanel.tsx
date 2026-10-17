import { settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { buttonLook, partOf, part, type ButtonLook } from "@bugaboo/ui";
import { type ReactNode } from "react";

import { QueryGate } from "../../shell/index.ts";
import { useOrganizationSettings } from "../../settings/index.ts";
import { ConnectionForm } from "./ConnectionForm.tsx";
import { NotReady } from "./NotReady.tsx";
import { SettingSwitch } from "./SettingSwitch.tsx";
import { TestEmail } from "./TestEmail.tsx";

export interface MailServerPanelProps {
  /** How the button that saves the mail server looks. */
  readonly saveLook?: ButtonLook;
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
            <NotReady
              enabled={shown.settings.some(
                (each) => each.key === settingKeys.mailEnabled && each.value === true,
              )}
            />
            <p {...partOf(part.muted)}>{t("mail-server.sign-up-note")}</p>
          </Section>
          <Section title={t("mail-server.connection")}>
            <ConnectionForm settings={shown} saveLook={saveLook} />
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
