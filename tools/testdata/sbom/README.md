# Schemas of the parts list (test data)

These are the published JSON Schemas that `cargo xtask parts-list` and the check DEP-SBOM validate
the parts list against (COMP-01). They are copied unchanged. They are test data: nothing here
ships in the program.

| File | Format | Copied from | License |
|------|--------|-------------|---------|
| `bom-1.6.schema.json` | CycloneDX 1.6 | `CycloneDX/specification`, tag `1.6`, `schema/bom-1.6.schema.json` | Apache-2.0 (`LICENSE-cyclonedx.txt`) |
| `spdx.schema.json` | the SPDX license identifiers CycloneDX uses | the same tag, `schema/spdx.schema.json` | Apache-2.0 (`LICENSE-cyclonedx.txt`) |
| `jsf-0.82.schema.json` | the signature format CycloneDX refers to | the same tag, `schema/jsf-0.82.schema.json` | Apache-2.0 (`LICENSE-cyclonedx.txt`) |
| `spdx-2.3.schema.json` | SPDX 2.3 | `spdx/spdx-spec`, tag `v2.3`, `schemas/spdx-schema.json` | CC-BY-3.0 (`LICENSE-spdx.txt`) |

To update one, copy the new published file over it and run `cargo test -p xtask release::`.
