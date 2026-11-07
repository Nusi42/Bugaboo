import { fieldKind, TextField } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { WithLengthReader, type LengthReader } from "../../shared/with-length-reader.tsx";
import { boxWords } from "./box-words.ts";
import type { BoxField, Sent } from "./fields.ts";
import { SaveForm } from "./SaveForm.tsx";
import { useSynced } from "./use-synced.ts";
import type { ValueEditorProps } from "./ValueEditor.tsx";
import { readBox, textOf, type Kept } from "./value-text.ts";

type TextEditorProps = Omit<ValueEditorProps, "field"> & {
  readonly field: BoxField;
  readonly readLength?: LengthReader;
  /** What the empty box says quietly inside itself (the box of a row of the Details). */
  readonly placeholder?: string;
  /** The hint under the box when it is words with lengths in them, which a text cannot hold. */
  readonly hint?: ReactNode | undefined;
  /** Told when the text typed is no value, so a form can hold back what it sends. */
  readonly onMistake?: () => void;
};

interface CommitParts {
  readonly field: BoxField;
  readonly value: unknown;
  readonly inRow: boolean;
  readonly readLength: LengthReader | undefined;
  readonly onCommit: (value: Sent) => void;
  readonly onMistake: (() => void) | undefined;
}

/**
 * What a box does with a text it is given: what `readBox` makes of it is sent, or told as a mistake
 * (and the sentence that says why is kept for the box, `local`), or left alone.
 */
function useCommit({ field, value, inRow, readLength, onCommit, onMistake }: CommitParts) {
  const [local, setLocal] = useState<string | undefined>();
  const [kept, setKept] = useState<Kept | undefined>();
  const commit = (typed: string): void => {
    const shown = textOf(field, value);
    const read = readBox(field, typed, shown, { kept, inRow, readLength });
    if (read.kind === "unchanged") {
      return;
    }
    setLocal(read.kind === "mistake" ? read.problem : undefined);
    if (read.kind === "mistake") {
      onMistake?.();
      return;
    }
    setKept({ from: shown, text: typed });
    onCommit(read.value);
  };
  return { commit, local };
}

/** A box for the kinds of value a person types: text, numbers and lengths of time. */
export function TextEditor({
  field,
  value,
  onCommit,
  problem,
  isPending = false,
  live = false,
  inRow = false,
  readLength,
  onMistake,
  hint,
  placeholder,
}: TextEditorProps) {
  const [text, setText] = useSynced(textOf(field, value));
  const { commit, local } = useCommit({ field, value, inRow, readLength, onCommit, onMistake });
  return (
    <SaveForm
      name={field.name}
      isPending={isPending}
      live={live}
      inRow={inRow}
      onSubmit={() => {
        commit(text);
      }}
    >
      <TextField
        {...boxWords(field, local ?? problem, inRow)}
        {...(hint === undefined ? {} : { description: hint })}
        {...(placeholder === undefined ? {} : { placeholder })}
        kind={field.kind === "long-text" ? fieldKind.multiline : fieldKind.text}
        value={text}
        onChange={(typed) => {
          setText(typed);
          if (live) {
            commit(typed);
          }
        }}
      />
    </SaveForm>
  );
}

/**
 * A box for a length of time: what is typed is read by the one reader of `bb-core`, whose module is
 * loaded when the box opens; a skeleton stands in until then (`WithLengthReader`).
 */
export function DurationEditor(props: ValueEditorProps) {
  return (
    <WithLengthReader>
      {(readLength) => <TextEditor {...props} readLength={readLength} />}
    </WithLengthReader>
  );
}
