import { t } from "@bugaboo/messages";
import { Button, buttonLook, Card } from "@bugaboo/ui";

import { CodeAppSection, PasskeysSection } from "../account/index.ts";
import styles from "./Setup.module.css";

export interface SaferStepProps {
  readonly onFinish: () => void;
}

/** The third step: a second check for the account, which the person can also add later. */
export function SaferStep({ onFinish }: SaferStepProps) {
  return (
    <>
      <Card heading={t("first-run.safer.title")} level={1}>
        <p className={styles.text}>{t("first-run.safer.intro")}</p>
      </Card>
      <PasskeysSection />
      <CodeAppSection />
      <Button look={buttonLook.primary} wide onPress={onFinish}>
        {t("first-run.safer.finish")}
      </Button>
    </>
  );
}
