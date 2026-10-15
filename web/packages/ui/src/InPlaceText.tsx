import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Input, TextField } from "react-aria-components";

import { partOf } from "./parts.ts";
import { buttonType } from "./props.ts";
import { part } from "./tokens.ts";

/** Words of the keyboard (the standard's names), not text for people. */
const keys = { enter: "Enter", escape: "Escape" } as const;

export interface InPlaceTextProps {
  /** What the text is, for screen readers: the name of the box while it is typed in. */
  readonly label: string;
  /** What the button that starts changing the text is called for screen readers (it holds the text). */
  readonly buttonLabel: string;
  readonly value: string;
  /** The text may not be empty; otherwise an empty text is saved as it is. */
  readonly required?: boolean;
  /** The text is shown and cannot be changed (the person may not change it). */
  readonly readOnly?: boolean;
  /** Called with the new text when it was changed and the person pressed Enter or left the box. */
  readonly onSave: (value: string) => void;
}

interface BoxProps {
  readonly label: string;
  readonly value: string;
  readonly required: boolean;
  /** Called when the box is left: with the text to save, or nothing when Esc put the old text back. */
  readonly onDone: (value: string | undefined) => void;
}

/** The box the text is typed in. Enter or leaving saves it; Esc cancels and stays inside this box. */
function Box({ label, value, required, onDone }: BoxProps) {
  const [draft, setDraft] = useState(value);
  const box = useRef<HTMLInputElement>(null);
  const latest = useRef({ draft, value, required, onDone });
  useLayoutEffect(() => {
    latest.current = { draft, value, required, onDone };
  });
  useEffect(() => {
    const element = box.current;
    element?.focus();
    const finish = (save: boolean): void => {
      const { draft: typed, value: was, required: must, onDone: done } = latest.current;
      const next = typed.trim();
      done(save && next !== was && !(must && next === "") ? next : undefined);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === keys.enter) {
        event.preventDefault();
        finish(true);
      } else if (event.key === keys.escape) {
        // Only this change is cancelled: the panel or view around it stays.
        event.preventDefault();
        finish(false);
      }
    };
    const onBlur = (): void => {
      finish(true);
    };
    element?.addEventListener("keydown", onKeyDown);
    element?.addEventListener("blur", onBlur);
    return () => {
      element?.removeEventListener("keydown", onKeyDown);
      element?.removeEventListener("blur", onBlur);
    };
  }, []);
  return (
    <TextField value={draft} onChange={setDraft} aria-label={label}>
      <Input ref={box} {...partOf(part.inPlaceInput)} />
    </TextField>
  );
}

/**
 * A value that is changed where it stands (UX §6): it looks like text, a press turns it into a
 * box, Enter or leaving the box saves it, and Esc puts the old text back.
 */
export function InPlaceText({
  label,
  buttonLabel,
  value,
  required = false,
  readOnly = false,
  onSave,
}: InPlaceTextProps) {
  const [typing, setTyping] = useState(false);
  if (readOnly) {
    return <span {...partOf(part.inPlaceText)}>{value}</span>;
  }
  if (typing) {
    return (
      <Box
        label={label}
        value={value}
        required={required}
        onDone={(next) => {
          setTyping(false);
          if (next !== undefined) {
            onSave(next);
          }
        }}
      />
    );
  }
  return (
    <button
      type={buttonType.button}
      {...partOf(part.inPlaceButton)}
      aria-label={buttonLabel}
      onClick={() => {
        setTyping(true);
      }}
    >
      {value}
    </button>
  );
}
