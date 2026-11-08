import { useState } from "react";

import type { AiSetup } from "./ai.ts";

/** What the setup form holds while the administrator types: nothing is sent before *Save*. */
export interface SetupDraft {
  readonly kind: AiSetup["provider"];
  readonly model: string;
  readonly address: string;
  /** A new key; empty keeps the one that is stored. */
  readonly key: string;
  /** The administrator's password, asked again for these changes. */
  readonly password: string;
}

/** The draft of the form, starting from the setup as it is, and the way to change one part of it. */
export function useSetupDraft(setup: AiSetup): {
  readonly draft: SetupDraft;
  readonly set: <Part extends keyof SetupDraft>(part: Part) => (value: SetupDraft[Part]) => void;
} {
  const [draft, setDraft] = useState<SetupDraft>({
    kind: setup.provider,
    model: setup.model,
    address: setup.address,
    key: "",
    password: "",
  });
  return {
    draft,
    set: (part) => (value) => {
      setDraft((before) => ({ ...before, [part]: value }));
    },
  };
}
