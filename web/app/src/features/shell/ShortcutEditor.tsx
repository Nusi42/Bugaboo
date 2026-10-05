import { t } from "@bugaboo/messages";
import { Alert, Button } from "@bugaboo/ui";
import { useState } from "react";

import {
  conflictFor,
  type Conflict,
  definitionOf,
  isReservedKeys,
  isValidSequence,
  keysOf,
  type ShortcutId,
  type ShortcutOverrides,
} from "../../shared/shortcuts.ts";
import { setShortcutKeys } from "../../shared/use-shortcuts.ts";
import { describeKeys } from "./key-words.ts";
import { KeyCaptureField } from "./KeyCaptureField.tsx";
import { Keys } from "./Keys.tsx";
import styles from "./ShortcutEditor.module.css";

export interface ShortcutEditorProps {
  readonly id: ShortcutId;
  readonly overrides: ShortcutOverrides;
}

/** The keys of one shortcut, each alternative drawn as keys. */
function KeyRow({ label, keys }: { readonly label: string; readonly keys: readonly string[] }) {
  return (
    <p className={styles.row}>
      <span className={styles.label}>{label}</span>
      {keys.map((sequence) => (
        <Keys key={sequence} sequence={sequence} />
      ))}
    </p>
  );
}

interface ProblemsProps {
  readonly conflict: Conflict | null;
  readonly invalid: boolean;
  readonly reserved: boolean;
  readonly saved: "keys" | "usual" | null;
}

/** What is wrong with the keys that were recorded, or the good news that they are kept. */
function Problems({ conflict, invalid, reserved, saved }: ProblemsProps) {
  return (
    <>
      {conflict === null ? null : (
        <Alert warning>
          {t("shortcuts.conflict", {
            keys: describeKeys(conflict.keys),
            action: t(definitionOf(conflict.id).message),
          })}
        </Alert>
      )}
      {invalid ? <Alert warning>{t("shortcuts.invalid")}</Alert> : null}
      {reserved ? <Alert warning>{t("shortcuts.reserved")}</Alert> : null}
      {saved === "keys" ? <Alert success>{t("shortcuts.saved")}</Alert> : null}
      {saved === "usual" ? <Alert success>{t("shortcuts.reset-done")}</Alert> : null}
    </>
  );
}

/**
 * The editor in the side panel of *Keyboard shortcuts*: record new keys, or go back to the usual
 * ones. Keys another shortcut already uses are refused with a sentence that names it.
 */
export function ShortcutEditor({ id, overrides }: ShortcutEditorProps) {
  const [recorded, setRecorded] = useState("");
  const [saved, setSaved] = useState<"keys" | "usual" | null>(null);
  const definition = definitionOf(id);
  const conflict = recorded === "" ? null : conflictFor(id, recorded, overrides);
  const invalid = recorded !== "" && !isValidSequence(recorded);
  const reserved = recorded !== "" && isReservedKeys(recorded);
  const use = (): void => {
    setShortcutKeys(id, [recorded]);
    setSaved("keys");
  };
  const reset = (): void => {
    setShortcutKeys(id, null);
    setRecorded("");
    setSaved("usual");
  };
  return (
    <>
      <KeyRow label={t("shortcuts.now")} keys={keysOf(id, overrides)} />
      <KeyRow label={t("shortcuts.usual")} keys={definition.keys} />
      <div className={styles.capture}>
        <KeyCaptureField
          value={recorded}
          onChange={(sequence) => {
            setRecorded(sequence);
            setSaved(null);
          }}
        />
      </div>
      <Problems conflict={conflict} invalid={invalid} reserved={reserved} saved={saved} />
      <div className={styles.actions}>
        <Button
          primary
          isDisabled={recorded === "" || conflict !== null || invalid || reserved}
          onPress={use}
        >
          {t("shortcuts.use")}
        </Button>
        <Button onPress={reset}>{t("shortcuts.reset")}</Button>
      </div>
    </>
  );
}
