import { t, type MessageKey } from "@bugaboo/messages";
import { useState } from "react";

import { useFocusHeading } from "../../shared/focus.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { AccountStep } from "./AccountStep.tsx";
import { CodeStep } from "./CodeStep.tsx";
import { DoneStep } from "./DoneStep.tsx";
import { SaferStep } from "./SaferStep.tsx";
import styles from "./Setup.module.css";

const steps = ["code", "account", "safer", "done"] as const;
type Step = (typeof steps)[number];

const stepNames: Readonly<Record<Step, MessageKey>> = {
  code: "first-run.step.code",
  account: "first-run.step.account",
  safer: "first-run.step.safer",
  done: "first-run.step.done",
};

/**
 * The first start in the browser: the setup code, the first administrator, a second check for
 * the account, done. The steps live in this page and not in the address: once the account
 * exists, the setup address leads to the sign-in page like for everybody else.
 */
export function SetupPage() {
  usePageTitle(t("first-run.title"));
  const [step, setStep] = useState<Step>("code");
  const [code, setCode] = useState("");
  const [codeProblem, setCodeProblem] = useState<string | null>(null);
  const container = useFocusHeading(step);
  return (
    <div className={styles.stack} ref={container}>
      <p className={styles.progress}>
        {t("first-run.step-of", {
          step: steps.indexOf(step) + 1,
          total: steps.length,
          name: t(stepNames[step]),
        })}
      </p>
      {step === "code" ? (
        <CodeStep
          code={code}
          onChange={setCode}
          problem={codeProblem}
          onNext={() => {
            setCodeProblem(null);
            setStep("account");
          }}
        />
      ) : null}
      {step === "account" ? (
        <AccountStep
          code={code}
          onCreated={() => {
            setStep("safer");
          }}
          onWrongCode={(message) => {
            setCodeProblem(message);
            setStep("code");
          }}
        />
      ) : null}
      {step === "safer" ? (
        <SaferStep
          onFinish={() => {
            setStep("done");
          }}
        />
      ) : null}
      {step === "done" ? <DoneStep /> : null}
    </div>
  );
}
