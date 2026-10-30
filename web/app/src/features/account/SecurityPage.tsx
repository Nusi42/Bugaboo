import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import { BackupCodesSection } from "./BackupCodesSection.tsx";
import { CodeAppSection } from "./CodeAppSection.tsx";
import { PasskeysSection } from "./PasskeysSection.tsx";
import { PasswordSection } from "./PasswordSection.tsx";

/** *Sign-in & security*: password, passkeys, code app and backup codes. */
export function SecurityPage() {
  usePageTitle(t("security.title"));
  return (
    <div {...partOf(part.stack)}>
      <Card
        heading={t("security.title")}
        level={1}
        look={cardLook.pageHead}
        intro={t("security.intro")}
      />
      <PasswordSection />
      <PasskeysSection />
      <CodeAppSection />
      <BackupCodesSection />
    </div>
  );
}
