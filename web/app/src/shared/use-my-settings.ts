import { useQuery } from "@tanstack/react-query";
import { useSyncExternalStore } from "react";

import { saveProgress, type SaveProgress } from "./change-my-settings.ts";
import { mySettingsOptions, type MySettings } from "./my-settings.ts";

/** The person's own settings, once the app has them (it asks when the person is signed in). */
export function useMySettings(): MySettings | undefined {
  return useQuery(mySettingsOptions()).data;
}

/** Where the person's changes stand: *Saving…*, *Saved* or *Not saved yet*, and why. */
export function useSaveProgress(): SaveProgress {
  return useSyncExternalStore(saveProgress.subscribe, saveProgress.read);
}
