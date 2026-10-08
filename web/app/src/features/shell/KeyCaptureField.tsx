import { t } from "@bugaboo/messages";
import { useEffect, useId, useLayoutEffect, useRef } from "react";
import { partOf, part } from "@bugaboo/ui";

import { chordOf, sequenceMs } from "../../shared/shortcuts-engine.ts";
import { describeKeys } from "./key-words.ts";

export interface KeyCaptureFieldProps {
  /** The keys recorded so far, written as in the registry; empty before the first key. */
  readonly value: string;
  readonly onChange: (sequence: string) => void;
}

/** Keys the field leaves alone, so the focus can still leave it and the panel can still close. */
const passThrough = new Set(["Tab", "Escape"]);

/**
 * A box that records the keys that are pressed in it instead of typing them: one key, or two in a
 * row (`g` and then `h`). It is the way to choose the keys of a shortcut without writing them in a
 * code. The box is a real field, so shortcuts stay silent while it has the focus.
 */
export function KeyCaptureField({ value, onChange }: KeyCaptureFieldProps) {
  const box = useRef<HTMLInputElement>(null);
  const inputId = useId();
  const helpId = useId();
  const latest = useRef({ value, onChange });
  const lastPress = useRef(0);
  useLayoutEffect(() => {
    latest.current = { value, onChange };
  });
  useEffect(() => {
    const element = box.current;
    const onKeyDown = (event: KeyboardEvent): void => {
      const chord = chordOf(event);
      if (chord === null || passThrough.has(event.key)) {
        return;
      }
      event.preventDefault();
      const { value: so_far, onChange: change } = latest.current;
      const now = Date.now();
      const second =
        so_far !== "" && !so_far.includes(" ") && now - lastPress.current <= sequenceMs;
      lastPress.current = now;
      change(second ? `${so_far} ${chord}` : chord);
    };
    element?.addEventListener("keydown", onKeyDown);
    return () => {
      element?.removeEventListener("keydown", onKeyDown);
    };
  }, []);
  return (
    <div {...partOf(part.keyCaptureField)}>
      <label htmlFor={inputId} {...partOf(part.keyCaptureFieldLabel)}>
        {t("shortcuts.new-keys")}
      </label>
      <input
        ref={box}
        id={inputId}
        {...partOf(part.keyCaptureFieldBox)}
        readOnly
        aria-describedby={helpId}
        value={value === "" ? "" : describeKeys(value)}
      />
      <p id={helpId} {...partOf(part.muted)}>
        {t("shortcuts.new-keys-help")}
      </p>
    </div>
  );
}
