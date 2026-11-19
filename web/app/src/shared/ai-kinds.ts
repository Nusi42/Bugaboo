// The keys of the switches of AI apps and agents: shared, because the browser tests and the load
// driver turn them on without loading the screens of the settings.

/**
 * The key of the setting that lets agents work: the organization has it, and a project can switch its agents off.
 * Written here because the generated list `settingKeys` holds only the settings of `settings/catalog.rs`, and this one is
 * declared in `settings/agents.rs` (the settings registry, ADR-0018).
 */
export const agentsEnabledKey = "agents.enabled";

/** The key of the setting that lets AI apps work (`settings/ai_apps.rs`): the same switch, for the apps (WP-15). */
export const aiAppsEnabledKey = "ai-apps.enabled";
