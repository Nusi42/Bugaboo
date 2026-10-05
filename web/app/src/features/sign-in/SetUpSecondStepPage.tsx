import { t } from "@bugaboo/messages";
import { Alert, Button, Card } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";

import { paths } from "../../shared/paths.ts";
import { forgetPerson } from "../../shared/session.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { CodeAppSection, PasskeysSection, useHasSecondStep } from "../account/index.ts";
import styles from "./SignIn.module.css";

/**
 * *Set up your second step*: what a person sees, and all they can do, when their role or the
 * organization asks for two-step sign-in and they have none yet (AUTH-03). A code app or a passkey
 * is set up here, with the password typed again; then they can go on.
 */
export function SetUpSecondStepPage() {
  usePageTitle(t("set-up-second-step.title"));
  const ready = useHasSecondStep();
  const navigate = useNavigate();
  return (
    <div className={styles.stack}>
      <Card heading={t("set-up-second-step.title")} level={1}>
        <p className={styles.text}>{t("set-up-second-step.why")}</p>
        <p className={styles.text}>{t("set-up-second-step.how")}</p>
        {ready ? (
          <>
            <Alert success>{t("set-up-second-step.ready")}</Alert>
            <div>
              <Button
                primary
                onPress={() => {
                  // The app asks the server again who this is: the way is open now.
                  forgetPerson();
                  void navigate({ to: paths.home });
                }}
              >
                {t("set-up-second-step.continue")}
              </Button>
            </div>
          </>
        ) : (
          <p className={styles.hint}>{t("set-up-second-step.waiting")}</p>
        )}
      </Card>
      <CodeAppSection />
      <PasskeysSection />
    </div>
  );
}
