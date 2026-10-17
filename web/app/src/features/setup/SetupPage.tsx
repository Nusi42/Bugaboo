import { t, type MessageKey } from "@bugaboo/messages";
import { useState } from "react";

import { useFocusHeading } from "../../shared/focus.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { AccountStep } from "./AccountStep.tsx";
import { CodeStep } from "./CodeStep.tsx";
import { DoneStep } from "./DoneStep.tsx";
import { MailStep } from "./MailStep.tsx";
import { SaferStep } from "./SaferStep.tsx";
import { partOf, part } from "@bugaboo/ui";

const steps = ["code", "account", "safer", "mail", "done"] as const;
type Step = (typeof steps)[number];

const stepNames: Readonly<Record<Step, MessageKey>> = {
  code: "first-run.step.code",
  account: "first-run.step.account",
  safer: "first-run.step.safer",
  mail: "first-run.step.mail",
  done: "first-run.step.done",
};

/** Which step of how many the person is on. */
function Progress({ step }: { readonly step: Step }) {
  return (
    <p {...partOf(part.setupProgress)}>
      {t("first-run.step-of", {
        step: steps.indexOf(step) + 1,
        total: steps.length,
        name: t(stepNames[step]),
      })}
    </p>
  );
}

/**
 * The first start in the browser: the setup code, the first administrator, a second check for
 * the account, the mail server, done. The steps live in this page and not in the address: once the
 * account exists, the setup address leads to the sign-in page like for everybody else.
 */
export function SetupPage() {
  usePageTitle(t("first-run.title"));
  const [step, setStep] = useState<Step>("code");
  const [code, setCode] = useState("");
  const [codeProblem, setCodeProblem] = useState<string | null>(null);
  const container = useFocusHeading(step);
  const goTo = (next: Step) => () => {
    setStep(next);
  };
  return (
    <div {...partOf(part.stack)} ref={container}>
      <Progress step={step} />
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
          onCreated={goTo("safer")}
          onWrongCode={(message) => {
            setCodeProblem(message);
            setStep("code");
          }}
        />
      ) : null}
      {step === "safer" ? <SaferStep onFinish={goTo("mail")} /> : null}
      {step === "mail" ? <MailStep onFinish={goTo("done")} /> : null}
      {step === "done" ? <DoneStep /> : null}
    </div>
  );
}
