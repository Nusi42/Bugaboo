import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, partOf, part } from "@bugaboo/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { BackupCodes } from "./BackupCodes.tsx";
import { CodeAppSetup } from "./CodeAppSetup.tsx";
import { SwitchOff } from "./SwitchOff.tsx";
import { twoStepOptions } from "./two-step.ts";

/** *Code app*: set it up, or switch it off. The backup codes of a new set-up are shown here. */
export function CodeAppSection() {
  const on = useQuery(twoStepOptions()).data?.code_app === true;
  const [codes, setCodes] = useState<readonly string[] | null>(null);
  const [turnedOff, setTurnedOff] = useState(false);
  return (
    <Card heading={t("code-app.title")} intro={t("code-app.intro")}>
      {codes === null ? null : (
        <>
          <Alert tone={alertTone.success}>{t("code-app.switched-on")}</Alert>
          <BackupCodes
            codes={codes}
            onSaved={() => {
              setCodes(null);
            }}
          />
        </>
      )}
      {turnedOff && !on ? <Alert tone={alertTone.success}>{t("code-app.turned-off")}</Alert> : null}
      {on ? (
        <SwitchOff
          onSwitchedOff={() => {
            setTurnedOff(true);
          }}
        />
      ) : (
        <>
          <p {...partOf(part.text)}>{t("code-app.off")}</p>
          <CodeAppSetup onSwitchedOn={setCodes} />
        </>
      )}
    </Card>
  );
}
