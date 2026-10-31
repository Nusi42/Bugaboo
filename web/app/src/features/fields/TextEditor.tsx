import { fieldKind, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { WithLengthReader, type LengthReader } from "../../shared/with-length-reader.tsx";
import { boxWords } from "./box-words.ts";
import { SaveForm } from "./SaveForm.tsx";
import { useSynced } from "./use-synced.ts";
import type { ValueEditorProps } from "./ValueEditor.tsx";
import { isUnchanged, sentFrom, textOf, type Kept } from "./value-text.ts";

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
}: ValueEditorProps & { readonly readLength?: LengthReader }) {
  const [text, setText] = useSynced(textOf(field, value));
  const [local, setLocal] = useState<string | undefined>();
  const [kept, setKept] = useState<Kept | undefined>();
  const commit = (typed: string): void => {
    const shown = textOf(field, value);
    // A box left without a change is no change, also before the page read the value it sent.
    if (inRow && isUnchanged(typed, shown, kept)) {
      return;
    }
    const sent = sentFrom(field, typed, readLength);
    setLocal("problem" in sent ? sent.problem : undefined);
    if ("value" in sent) {
      setKept({ from: shown, text: typed });
      onCommit(sent.value);
    }
  };
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
