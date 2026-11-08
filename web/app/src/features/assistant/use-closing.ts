import { useState } from "react";

import { closeAssistantPanel } from "../shell/index.ts";

/** Closing the panel: at once when nothing would be lost, else after the question whether to end the conversation. */
export function useClosing(hasTurns: boolean): {
  readonly closing: boolean;
  readonly setClosing: (closing: boolean) => void;
  readonly close: () => void;
} {
  const [closing, setClosing] = useState(false);
  return {
    closing,
    setClosing,
    close: () => {
      if (hasTurns) {
        setClosing(true);
      } else {
        closeAssistantPanel();
      }
    },
  };
}
