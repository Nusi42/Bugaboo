import { t } from "@bugaboo/messages";
import { Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { usePageTitle } from "../../../shared/use-page.ts";
import { KeyList } from "./KeyList.tsx";
import { CreateKeyForm } from "./CreateKeyForm.tsx";
import { ShownOnce } from "./ShownOnce.tsx";

/** What the page shows apart from the list: the button, the form, or the key that was just made. */
type Making =
  | { readonly step: "idle" }
  | { readonly step: "form" }
  | { readonly step: "shown"; readonly secret: string };

/** *Access keys* under *My settings*: the keys of the signed-in person, making one and deleting one. */
export function AccessKeysPage() {
  usePageTitle(t("access-keys.title"));
  const [making, setMaking] = useState<Making>({ step: "idle" });
  const idle = (): void => {
    setMaking({ step: "idle" });
  };
  return (
    <div {...partOf(part.stack)}>
      <Card heading={t("access-keys.title")} level={1} intro={t("access-keys.intro")}>
        {making.step === "idle" ? (
          <div {...partOf(part.row, { variants: ["stretch"] })}>
            <Button
              look={buttonLook.primary}
              onPress={() => {
                setMaking({ step: "form" });
              }}
            >
              {t("access-keys.create")}
            </Button>
          </div>
        ) : null}
      </Card>
      {making.step === "shown" ? <ShownOnce secret={making.secret} onDone={idle} /> : null}
      {making.step === "form" ? (
        <Card heading={t("access-keys.create")}>
          <CreateKeyForm
            onMade={(secret) => {
              setMaking({ step: "shown", secret });
            }}
            onCancel={idle}
          />
        </Card>
      ) : null}
      <KeyList />
    </div>
  );
}
