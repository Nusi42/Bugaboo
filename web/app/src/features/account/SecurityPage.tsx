import { t } from "@bugaboo/messages";
import { Card } from "@bugaboo/ui";

import { usePageTitle } from "../../shared/use-page.ts";
import styles from "./Account.module.css";
import { BackupCodesSection } from "./BackupCodesSection.tsx";
import { CodeAppSection } from "./CodeAppSection.tsx";
import { PasskeysSection } from "./PasskeysSection.tsx";
import { PasswordSection } from "./PasswordSection.tsx";

/** *Sign-in & security*: password, passkeys, code app and backup codes. */
export function SecurityPage() {
  usePageTitle(t("security.title"));
  return (
    <div className={styles.stack}>
      <Card heading={t("security.title")} level={1}>
        <p className={styles.text}>{t("security.intro")}</p>
      </Card>
      <PasswordSection />
      <PasskeysSection />
      <CodeAppSection />
      <BackupCodesSection />
    </div>
  );
}
