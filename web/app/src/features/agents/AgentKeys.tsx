import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, Card, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { ShownOnce } from "../my-settings/index.ts";
import { type AgentKey } from "./agents.ts";
import { KeyForm } from "./KeyForm.tsx";
import { KeyRow } from "./KeyRow.tsx";

interface StepProps {
  readonly agent: string;
  readonly making: Making;
  readonly onMade: (secret: string) => void;
  readonly onDone: () => void;
}

/** Under the list: the form that makes or renews a key, or the key that was just made. */
function KeyStep({ agent, making, onMade, onDone }: StepProps) {
  if (making.step === "shown") {
    return (
      <ShownOnce
        secret={making.secret}
        onDone={onDone}
        title={t("agents.key-shown-title")}
        label={t("agents.key-label")}
      />
    );
  }
  if (making.step === "form") {
    return (
      <Card
        heading={making.renewing === null ? t("agents.key-create") : t("agents.key-renew-title")}
      >
        <KeyForm agent={agent} renewing={making.renewing} onMade={onMade} onCancel={onDone} />
      </Card>
    );
  }
  return null;
}

/** What the keys part of the page shows apart from the list: the button, the form, or the key that was just made. */
type Making =
  | { readonly step: "idle" }
  | { readonly step: "form"; readonly renewing: AgentKey | null }
  | { readonly step: "shown"; readonly secret: string };

interface KeyListProps {
  readonly agent: string;
  readonly keys: readonly AgentKey[];
  readonly stopped: boolean;
  readonly onRenew: (item: AgentKey) => void;
  readonly onTakenBack: (name: string) => void;
}

/** The keys of the agent, or the line that it has none that works. */
function KeyList({ agent, keys, stopped, onRenew, onTakenBack }: KeyListProps) {
  return (
    <>
      {keys.length === 0 ? <p {...partOf(part.muted)}>{t("agents.keys-none")}</p> : null}
      <ul {...partOf(part.list)} aria-label={t("agents.keys")}>
        {keys.map((item) => (
          <KeyRow
            key={item.id}
            agent={agent}
            item={item}
            stopped={stopped}
            onRenew={() => {
              onRenew(item);
            }}
            onTakenBack={onTakenBack}
          />
        ))}
      </ul>
    </>
  );
}

export interface AgentKeysProps {
  readonly agent: string;
  readonly keys: readonly AgentKey[];
  readonly stopped: boolean;
}

/**
 * The keys of an agent (UX §4.17): each one runs out; *Create an agent key* shows the secret once,
 * *Renew* makes a new one with the same name, *Take back* ends one at once. The key part is the one
 * of access keys (the form of the day, the key shown once), not a second screen.
 */
export function AgentKeys({ agent, keys, stopped }: AgentKeysProps) {
  const [making, setMaking] = useState<Making>({ step: "idle" });
  // The name of the key that was taken back just now: its row is gone, so the card says so.
  const [takenBack, setTakenBack] = useState<string | null>(null);
  const idle = (): void => {
    setMaking({ step: "idle" });
  };
  return (
    <>
      <Card heading={t("agents.keys")} intro={t("agents.keys-intro")}>
        {takenBack === null ? null : (
          <Alert tone={alertTone.success}>{t("agents.key-taken-back", { name: takenBack })}</Alert>
        )}
        <KeyList
          agent={agent}
          keys={keys}
          stopped={stopped}
          onRenew={(item) => {
            setMaking({ step: "form", renewing: item });
          }}
          onTakenBack={setTakenBack}
        />
        {making.step === "idle" && !stopped ? (
          <div {...partOf(part.row, { variants: ["stretch"] })}>
            <Button
              look={buttonLook.primary}
              onPress={() => {
                setMaking({ step: "form", renewing: null });
              }}
            >
              {t("agents.key-create")}
            </Button>
          </div>
        ) : null}
      </Card>
      <KeyStep
        agent={agent}
        making={making}
        onMade={(secret) => {
          setTakenBack(null);
          setMaking({ step: "shown", secret });
        }}
        onDone={idle}
      />
    </>
  );
}
