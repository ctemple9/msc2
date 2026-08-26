# Java player NBT fixture boundary

These fixtures split the reader's behavior into two layers.

`crates/msc-domain/src/nbt.rs` already covers the generic Java NBT machinery
ported in P6.9: gzip decompression, big-endian primitive decoding, the twelve
NBT tag types, compound/list traversal, root-compound validation, and
truncated-input failure. The corrupt-gzip, truncated-payload, and
non-compound-root fixtures below repeat those boundaries at the player-file
entry point so P12.2c preserves the same three failure stages; they are not a
reason to build another tag reader.

The player-specific rules are the extraction contract from
`PlayerNBTReader.swift`: stats defaults and field names, `Attributes` max
health aliases, numeric-to-display dimension mapping, inventory entry
filtering, slot/count type variants, legacy `tag` versus modern `components`,
enchantments and stored enchantments, JSON text-component names, and integer
damage. `iconName`, `displayName`, and enchantment display names are the
small DTO projections that the frozen player contract derives from the
reader's `InventoryItem`/`ItemEnchantment` values.

`samples/campak-live-player.dat` is an unchanged, local-only capture from
Cameron's live MSC 1 Java `campak` server. The JSON fixture records its hash
and provenance; the raw file is intentionally ignored. The remaining valid
NBT payloads are hand-built synthetic cases, labeled as such because the live
capture does not happen to contain every legacy/modern item-tag variant.
