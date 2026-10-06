/** The pages of the settings of a project (words of the code, not text for people). */
export const settingsTab = { general: "general", roles: "roles" } as const;

/** One of the tabs. */
export type SettingsTab = (typeof settingsTab)[keyof typeof settingsTab];
