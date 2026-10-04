# P18.15 resource inspection

This inspects saved Java blocks and resource evidence. It does not adopt captured meshes or repair rendering. Existing maps keep their renderer. The isolated P18.14 proof remains separate.

Build the updated executable:

```sh
cargo build -p msc-agent --bin msc
```

Run the updated agent through your normal development startup before using the commands below. Do not start another agent against the same application data while an installed agent is running. Select the intended Java server and stop it before inspecting its active world; archived slots can be inspected without stopping the server. Worlds permission is required.

List slots and obtain the selected slot’s binding revision:

```sh
target/debug/msc --json world list
target/debug/msc --json world map-assets status --slot SLOT_UUID
```

Replace `SLOT_UUID` with the listed UUID and `REVISION` with `binding.revision` from status. Bounds are inclusive, in block coordinates. For a small overworld region:

```sh
target/debug/msc --json world map-assets check --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld --min 0 64 0 --max 15 80 15
target/debug/msc --json world map-assets report --slot SLOT_UUID
```

Check waits for the asynchronous operation and prints its report. `--no-wait` instead returns its operation ID. Ctrl-C uses existing operation cancellation. `needs_input` returns a nonzero CLI exit code while retaining the diagnostic report; it is not a repaired/ready result. A stale revision refuses publication: reread status rather than reusing the old binding.

Inspect original namespaced IDs, source hashes and declared mod identities, requested area, snapshot/resource generation, sample coordinates, classifications and omitted counts. `missing_saved_chunk` counts chunks rather than blocks. An explicit empty model is distinct from a missing model. A block entity’s presence does not prove its rendered contents are available. Unknown client selection and unsupported loader/material/renderer features stay explicit. A saved-world version mismatch refuses geometry adoption. `visibleFaces` and geometry generation remain absent because this stage does not render geometry.

The inventory reads installed JARs, declared nested JARs, retained manifest receipts and existing server `resource-packs` archives. It reads only an existing exact-version vanilla helper cache; it does not download missing assets, execute mods, infer pack priority or collect a client installation automatically. Explicit client acquisition/staging is the next step. Diagnostics are a bounded independent resolver inspection, not evidence that the production Vantage renderer can display a modded model.

Acceptance remains manual: check a known broken-model region and an intentional empty model; inspect a second slot/server and confirm bindings are distinct; open existing Java and Bedrock maps and confirm their baseline rendering remains intact. Broader mod/platform acceptance is unchanged. Numeric limits and conservative generation/quota behavior are recorded in [the design](world-map-assets-design.md#p1815-implementation-record--2026-10-04).
