# M2C follow-ups

Points found while building M2C that were accepted for later. Each names where it will be done. The IDs (`M2C-F01` …) are what the review refers to. C-02 uses F01 and up, C-03 F20 and up.

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M2C-F20 | The test of `bb-store` (`crates/bb-store/src/tests/access_keys.rs`) still has its own `type Failure = Box<dyn std::error::Error + Send + Sync>` next to `bb_testing::Failure` (audit B8) | C-02 refactors `bb-store` at the same time and owns its files; a one-line change there would only add a merge conflict | after C-02 is merged: use `bb_testing::Failure` (one line) |
| M2C-F21 | The failure limit that never limits is written twice: `NoLimit` in the `bb-app` tests (`crates/bb-app/src/tests/world.rs`) and `Unlimited` in `bb-sample` (`crates/bb-sample/src/limits.rs`) (audit B11) | The one home is `bb-app` (the trait `FailureLimit` lives there), which C-02 owns | after C-02 is merged: one `NoFailureLimit` in `bb-app`, used by both |
| M2C-F22 | The test passwords `PASSWORD` and `OTHER_PASSWORD` are defined in `bb-app` (`tests/world.rs`) and in the sign-in rig of `bb-server` (`tests/sign_in/rig.rs`) | A shared home (`bb-testing`) needs a dev-dependency in `bb-app`, a crate C-02 owns | after C-02 is merged: define them once in `bb-testing` |
| M2C-F24 | `operations()` does not list an HTTP `QUERY` operation (`PathItem::query` of utoipa): `http::Method` has no constant for it and no route uses it | Nothing to walk today; adding it blind would be dead code | the first `QUERY` route: add it to `each_operation` in `crates/bb-api/src/openapi.rs` |
| M2C-F26 | The helpers `inviter`, `role_with`, `checker` and `switch` of the server tests take `two_step: bool` (audit B12, boolean flags) | The flag is the value of the field `two_step` of the use cases `MakeRole` and `ChangeRole` that they fill in, not a switch between behaviours; audit B12 is C-04's | C-04 decides whether a name for the switch is worth it |
