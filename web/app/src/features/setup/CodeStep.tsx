import { t } from "@bugaboo/messages";
import { Alert, alertTone, autoComplete, Card, Form, SubmitButton, TextField } from "@bugaboo/ui";

import styles from "./Setup.module.css";

export interface CodeStepProps {
  readonly code: string;
  readonly onChange: (code: string) => void;
  /** What the server said when the code was wrong, if it did. */
  readonly problem: string | null;
  readonly onNext: () => void;
}

/** The first step: the setup code that the server shows where it runs. */
export function CodeStep({ code, onChange, problem, onNext }: CodeStepProps) {
  return (
    <Card heading={t("first-run.title")} level={1}>
      <p className={styles.text}>{t("first-run.code.intro")}</p>
      <p className={styles.text}>{t("first-run.code.where")}</p>
      <Form onSubmit={onNext}>
        <TextField
          label={t("first-run.code.label")}
          description={t("first-run.code.help")}
          autoComplete={autoComplete.off}
          requiredMessage={t("validation.required")}
          value={code}
          onChange={onChange}
        />
        {problem === null ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <SubmitButton wide>{t("common.continue")}</SubmitButton>
      </Form>
    </Card>
  );
}
