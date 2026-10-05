import { t } from "@bugaboo/messages";
import { Alert, Button } from "@bugaboo/ui";
import { useState } from "react";

import {
  conflictFor,
  definitionOf,
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
  const use = (): void => {
    setShortcutKeys(id, [recorded]);
    setRecorded("");
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
        <p className={styles.help}>{t("shortcuts.new-keys-help")}</p>
      </div>
      {conflict === null ? null : (
        <Alert warning>
          {t("shortcuts.conflict", {
            keys: describeKeys(conflict.keys),
            action: t(definitionOf(conflict.id).message),
          })}
        </Alert>
      )}
      {invalid ? <Alert warning>{t("shortcuts.invalid")}</Alert> : null}
      {saved === "keys" ? <Alert success>{t("shortcuts.saved")}</Alert> : null}
      {saved === "usual" ? <Alert success>{t("shortcuts.reset-done")}</Alert> : null}
      <div className={styles.actions}>
        <Button primary isDisabled={recorded === "" || conflict !== null || invalid} onPress={use}>
          {t("shortcuts.use")}
        </Button>
        <Button onPress={reset}>{t("shortcuts.reset")}</Button>
      </div>
    </>
  );
}
