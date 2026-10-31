import { useEffect, useState } from "react";

import { Announce } from "./Announce.tsx";
import { Button } from "./Button.tsx";

/** How long "Copied" stays on the button before it says "Copy" again. */
const COPIED_FOR_MS = 3000;

export interface CopyButtonProps {
  /** What is put on the clipboard. */
  readonly text: string;
  /** What the button says. */
  readonly label: string;
  /** What the button is called for screen readers when the text alone does not say what it does. */
  readonly name?: string;
  /** What the button says, and a screen reader announces, for a moment after it copied. */
  readonly copiedLabel: string;
}

/** Copies a text to the clipboard and says so, on the button and to screen readers. */
export function CopyButton({ text, label, name, copiedLabel }: CopyButtonProps) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    const timer = copied
      ? setTimeout(() => {
          setCopied(false);
        }, COPIED_FOR_MS)
      : null;
    return () => {
      if (timer !== null) {
        clearTimeout(timer);
      }
    };
  }, [copied]);

  return (
    <>
      <Button
        // The name follows the words on the button, so it must not outlive them.
        {...(copied || name === undefined ? {} : { "aria-label": name })}
        onPress={() => {
          navigator.clipboard.writeText(text).then(
            () => {
              setCopied(true);
            },
            () => {
              setCopied(false);
            },
          );
        }}
      >
        {copied ? copiedLabel : label}
      </Button>
      <Announce>{copied ? copiedLabel : ""}</Announce>
    </>
  );
}
