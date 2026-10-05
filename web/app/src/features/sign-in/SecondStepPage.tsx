import { t } from "@bugaboo/messages";
import { Card, Link, OrDivider } from "@bugaboo/ui";
import { useSearch } from "@tanstack/react-router";

import { paths } from "../../shared/paths.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { CodeForm } from "./CodeForm.tsx";
import { PasskeySecondStep } from "./PasskeySecondStep.tsx";
import styles from "./SignIn.module.css";

/** *One more step* after the password: a code, or a passkey. */
export function SecondStepPage() {
  // Which ways there are is in the address: `?app=true&passkey=true`.
  const { app = false, passkey = false } = useSearch({ strict: false });
  usePageTitle(t("second-step.title"));
  return (
    <Card heading={t("second-step.title")} level={1}>
      <p className={styles.text}>{t("second-step.intro")}</p>
      {app ? <CodeForm /> : null}
      {app && passkey ? <OrDivider label={t("sign-in.or")} /> : null}
      {passkey ? (
        <>
          <p className={styles.hint}>{t("second-step.passkey-help")}</p>
          <PasskeySecondStep />
        </>
      ) : null}
      <p className={styles.hint}>
        <Link href={paths.signIn}>{t("second-step.back")}</Link>
      </p>
    </Card>
  );
}
