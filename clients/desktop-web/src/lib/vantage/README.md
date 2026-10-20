# Pinned Vantage viewer source

`core/` and `three/` are the web viewer source from Vantage v0.15.1
(`953f0ac78d62de751a935b8e8d5be2d711d6fd56`), licensed under the
accompanying MIT `LICENSE`. MSC imports these TypeScript modules directly.
The separate Zig renderer remains under `third_party/vantage/`.

MSC's local changes are limited to camera streaming, refresh replacement, and
the public detail-radius status needed by the Worlds map. Preserve those
changes when updating the pinned Vantage version.
