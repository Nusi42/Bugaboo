import { inPlace } from "./in-place.ts";
import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { TextArea, TextField } from "react-aria-components";

import { partOf } from "./parts.ts";
import { composing, plainEnter } from "./plain-enter.ts";
import { buttonType } from "./props.ts";
import { part } from "./tokens.ts";

/** Words of the keyboard (the standard's names), not text for people. */
const keys = { enter: "Enter", escape: "Escape" } as const;

export interface InPlaceTextProps {
  /** What the text is, for screen readers: the name of the box while it is typed in. */
  readonly label: string;
  /** What the button that starts changing the text is called for screen readers, from the text it shows. */
  readonly buttonLabel: (shown: string) => string;
  readonly value: string;
  /** The text may not be empty; otherwise an empty text is saved as it is. */
  readonly required?: boolean;
  /** The text is shown and cannot be changed (the person may not change it). */
  readonly readOnly?: boolean;
  /**
   * Called with the new text when it was changed and the person pressed Enter or left the box;
   * `settled` hears whether it was saved, and a text that was not saved is no longer shown.
   */
  readonly onSave: (value: string, settled: (saved: boolean) => void) => void;
}

interface BoxProps {
  readonly label: string;
  readonly value: string;
  readonly required: boolean;
  /** Called when the box is left: with the text to save, or nothing when Esc put the old text back. */
  readonly onDone: (value: string | undefined) => void;
}

/** The box and what is typed in it. */
interface Typed extends BoxProps {
  readonly draft: string;
}

/** The text is one line: a line break typed or pasted into it becomes a space. */
function oneLine(text: string): string {
  return text.replaceAll(/[\r\n]+/gu, " ");
}

/** What the keys do in the box: Enter alone saves, Esc cancels; keys of a composition are left alone. */
function onKeyOf(finish: (save: boolean) => void): (event: KeyboardEvent) => void {
  return (event) => {
    // A key that ends the composing of a letter (an input method) belongs to that composition.
    if (composing(event)) {
      return;
    }
    if (event.key === keys.enter) {
      const save = plainEnter(event);
      // The text is one line: no Enter starts a new one.
      event.preventDefault();
      if (save) {
        finish(true);
      }
    } else if (event.key === keys.escape) {
      // Only this change is cancelled: the panel or view around it stays.
      event.preventDefault();
      finish(false);
    }
  };
}

/**
 * The box, for its ref: the caret starts at the end of its text, and the change ends once, on Enter,
 * Esc or leaving the box.
 */
function useEnds(props: Typed): RefObject<HTMLTextAreaElement | null> {
  const box = useRef<HTMLTextAreaElement>(null);
  const latest = useRef(props);
  useLayoutEffect(() => {
    latest.current = props;
  });
  useEffect(() => {
    const element = box.current;
    element?.focus(inPlace);
    element?.setSelectionRange(element.value.length, element.value.length);
    // Enter ends the change, and the box then goes, which the browser also tells as a blur: the change
    // is made once, not a second time on the version before it (a conflict with the person's own).
    let finished = false;
    const finish = (save: boolean): void => {
      if (finished) {
        return;
      }
      finished = true;
      const { draft, value, required, onDone } = latest.current;
      const next = draft.trim();
      onDone(save && next !== value && !(required && next === "") ? next : undefined);
    };
    const onKeyDown = onKeyOf(finish);
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
  return box;
}

/**
 * The box the text is typed in, which wraps its words and grows with them as the text does. Enter or
 * leaving saves it; Esc cancels and stays inside this box.
 */
function Box(props: BoxProps) {
  const [draft, setDraft] = useState(props.value);
  const box = useEnds({ ...props, draft });
  return (
    <TextField
      value={draft}
      onChange={(text) => {
        setDraft(oneLine(text));
      }}
      aria-label={props.label}
    >
      <span {...partOf(part.inPlaceGrow)}>
        <span aria-hidden {...partOf(part.inPlaceMirror)}>
          {draft}
        </span>
        <TextArea ref={box} rows={1} {...partOf(part.inPlaceInput)} />
      </span>
    </TextField>
  );
}

/**
 * The text to show: what was saved shows at once, until the value changes (the change reaches the
 * value a moment later, and the old text would stand there for a frame in between, UX §6.4), and
 * goes when the save was refused, so the value shows again.
 */
function useShown(value: string) {
  const [saved, setSaved] = useState<{ readonly from: string; readonly text: string } | null>(null);
  if (saved !== null && saved.from !== value) {
    setSaved(null);
  }
  return {
    shown: saved === null ? value : saved.text,
    tried: (text: string) => {
      const tried = { from: value, text };
      setSaved(tried);
      return (done: boolean): void => {
        if (!done) {
          setSaved((now) => (now === tried ? null : now));
        }
      };
    },
  };
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
  const { shown, tried } = useShown(value);
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
            onSave(next, tried(next));
          }
        }}
      />
    );
  }
  return (
    <button
      type={buttonType.button}
      {...partOf(part.inPlaceButton)}
      aria-label={buttonLabel(shown)}
      onClick={() => {
        setTyping(true);
      }}
    >
      {shown}
    </button>
  );
}
