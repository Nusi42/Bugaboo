import { settingKeys, type Schema, type SettingKey } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "./api.ts";
import { queryClient } from "./query-client.ts";
import { isNoAnswer } from "./errors.ts";

/** One setting of the person, with the value that counts for them and where it comes from. */
export type Setting = Schema<"SettingBody">;
/** The settings a person can choose for themselves (*My settings*). */
export type MySettings = Schema<"SettingsBody">;
/** What a setting holds: yes or no, a whole number, or text. */
export type SettingValue = Setting["value"];

/**
 * The settings of the person who is signed in. The answer is never kept by the browser (the server
 * says so) and the cache forgets it when the person signs out, so the next person at this device
 * finds nothing of it (M2-F13).
 */
export function mySettingsOptions() {
  return api.query.queryOptions("get", "/api/v1/me/settings", undefined, { staleTime: 0 });
}

/**
 * Makes sure the app has the person's settings before the first page is drawn, so the look is the
 * person's from the first frame. When the server cannot answer, the person gets the standard
 * settings and everything else still works.
 */
export async function loadMySettings(): Promise<void> {
  try {
    await queryClient.query({ ...mySettingsOptions(), staleTime: "static" });
  } catch (error) {
    if (!isNoAnswer(error)) {
      throw error;
    }
    // The pages ask again, and a lost sign-in is told to the person by the cache itself.
  }
}

/** The key of the query in the cache of the app. */
export function mySettingsKey() {
  return mySettingsOptions().queryKey;
}

/** The settings of the person, if the app has them (it asks when the person is signed in). */
export function currentMySettings(): MySettings | undefined {
  const known: unknown = queryClient.getQueryData(mySettingsKey());
  return isMySettings(known) ? known : undefined;
}

/** Whether what the cache holds under the key is the answer about the person's settings. */
export function isMySettings(value: unknown): value is MySettings {
  return typeof value === "object" && value !== null && "settings" in value;
}

/** Whether a key the server named is one this app knows. */
export function isSettingKey(key: string): key is SettingKey {
  return Object.values<string>(settingKeys).includes(key);
}

/** One setting by its key. */
export function settingOf(settings: MySettings | undefined, key: SettingKey): Setting | undefined {
  return settings?.settings.find((setting) => setting.key === key);
}

/**
 * Whether the person's settings are being read because the app has none yet: a part that writes times and dates in the
 * person's *Region* waits for them, so it writes them once in it, not first in the device's way (M8-F221). The app
 * reads the settings before the first page; when that read had no answer, the part asks again.
 */
export function useMySettingsReading(): boolean {
  return useQuery({ ...mySettingsOptions(), enabled: currentMySettings() === undefined }).isLoading;
}
