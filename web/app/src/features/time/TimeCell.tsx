import { CellInput, gridColumnSelector, partOf, part } from "@bugaboo/ui";
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";

import { CellDraft } from "./cell-draft.ts";
import { gridSelector } from "./html-words.ts";

/** What a typed cell comes to: a value to save, a cleared cell, or the sentence of what is wrong. */
export type CellRead<Value> =
  | { readonly value: Value }
  | { readonly empty: true }
  | { readonly mistake: string };

/** How saving a typed cell went: the sentence of why it could not be, or that nothing was different. */
export interface Committed {
  readonly problem?: string;
  readonly unchanged?: true;
  /** What to tell the person beside the cell when nothing is wrong, but nothing is saved yet. */
  readonly note?: string;
}

export interface TimeCellProps<Value> {
  /** What the cell says, for people who cannot see its row and column: *Start, Thursday 1*. */
  readonly label: string;
  /** The text of the saved value; empty for none. */
  readonly text: string;
  /** Whether a rule of the day was broken, which the figure shows in the warning color. */
  readonly warns?: boolean;
  /** Reads the text the person typed. */
  readonly read: (typed: string) => CellRead<Value>;
  /** Saves what was typed and answers how it went. A cell that was cleared saves `undefined`. */
  readonly commit: (value: Value | undefined) => Promise<Committed>;
  /** The column of the grid, so *Enter* can go to the next row of it. */
  readonly column: number;
  readonly readOnly?: boolean;
}

/** What is typed in a cell (`CellDraft` says what it may take of the saved text). */
function useDraft(text: string) {
  const [draft] = useState(() => new CellDraft(text));
  const [typed, setTyped] = useState(text);
  const shown = useRef(text);
  const takes = useRef(false);
  useEffect(() => {
    shown.current = text;
    takes.current = draft.sync(text);
    if (takes.current) {
      setTyped(text);
    }
  }, [draft, text]);
  return {
    typed,
    type: (value: string): void => {
      draft.type(value);
      setTyped(value);
    },
    enter: () => {
      draft.enter();
    },
    leaving: () => {
      draft.leave();
    },
    giveUp: (saved: string): void => {
      draft.giveUp(saved);
      setTyped(saved);
    },
    /** A save of `value` is on its way; the answer is given to the function it returns. */
    saving: (value: string): ((saved: boolean) => void) => {
      const answer = draft.saving(value);
      return (saved) => {
        answer(saved);
        if (draft.syncHeld(shown.current)) {
          setTyped(shown.current);
        }
      };
    },
  };
}

/** What a cell remembers while it is typed in: the draft, the sentence of a problem, and whether it saves. */
function useTyping<Value>(props: TimeCellProps<Value>) {
  const { text, read, commit } = props;
  const draft = useDraft(text);
  const { typed, type } = draft;
  const [problem, setProblem] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  const [note, setNote] = useState<string | undefined>();
  const leave = async (): Promise<void> => {
    draft.leaving();
    if (typed.trim() === text.trim()) {
      draft.giveUp(text);
      setProblem(undefined);
      return;
    }
    const understood = read(typed);
    if ("mistake" in understood) {
      setProblem(understood.mistake);
      return;
    }
    setProblem(undefined);
    setSaving(true);
    const done = draft.saving(typed);
    const result = await commit("value" in understood ? understood.value : undefined);
    setSaving(false);
    setProblem(result.problem);
    setNote(result.note);
    done(result.problem === undefined);
    if (result.unchanged === true) {
      draft.giveUp(text);
    }
  };
  const giveUp = (): void => {
    draft.giveUp(text);
    setProblem(undefined);
  };
  return { typed, setTyped: type, problem, note, saving, enter: draft.enter, leave, giveUp };
}

/** *Enter* goes to the cell under this one in the same column, and leaves the last one. */
function goDown(event: KeyboardEvent<HTMLInputElement>, column: number): void {
  event.preventDefault();
  const here = event.currentTarget;
  const cells = [
    ...(here.closest(gridSelector)?.querySelectorAll(gridColumnSelector(column)) ?? []),
  ];
  const next = cells[cells.indexOf(here) + 1];
  if (next instanceof HTMLInputElement) {
    next.focus();
    next.select();
  } else {
    here.blur();
  }
}

/**
 * One cell of the grid: a box that takes a time of day or a length, saves when the focus leaves it
 * (UX §6), and marks a wrong value in place with the sentence of what to type instead. *Enter* goes
 * down like in a spreadsheet and *Tab* to the next cell; *Escape* gives back the saved value. The
 * parent gives it a new `key` when the saved value changes, so a value that was saved shows at once.
 */
export function TimeCell<Value>(props: TimeCellProps<Value>) {
  const { label, warns = false, column, readOnly = false } = props;
  const cell = useTyping(props);
  const problemId = useId();
  return (
    <>
      <CellInput
        label={label}
        value={cell.typed}
        onChange={cell.setTyped}
        onEnter={cell.enter}
        onLeave={() => {
          void cell.leave();
        }}
        onKey={(event) => {
          if (event.key === "Escape") {
            cell.giveUp();
          } else if (event.key === "Enter") {
            goDown(event, column);
          }
        }}
        column={column}
        readOnly={readOnly}
        invalid={cell.problem !== undefined}
        saving={cell.saving}
        warns={warns}
        describedBy={cell.problem === undefined ? undefined : problemId}
      />
      {cell.problem === undefined ? null : (
        <span id={problemId} {...partOf(part.sheetProblem)}>
          {cell.problem}
        </span>
      )}
      {cell.note === undefined || cell.problem !== undefined ? null : (
        <span {...partOf(part.sheetNote)}>{cell.note}</span>
      )}
    </>
  );
}
