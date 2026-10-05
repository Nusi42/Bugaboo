import { useEffect, useState } from "react";

import { Button } from "./Button.tsx";

/** How long "Copied" stays on the button before it says "Copy" again. */
const COPIED_FOR_MS = 3000;

export interface CopyButtonProps {
  /** What is put on the clipboard. */
  readonly text: string;
  readonly label: string;
  /** What the button says for a moment after it copied. */
  readonly copiedLabel: string;
}

/** Copies a text to the clipboard and says so. */
export function CopyButton({ text, label, copiedLabel }: CopyButtonProps) {
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
    <Button
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
  );
}
