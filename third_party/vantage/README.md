# Vantage renderer source

This is the MIT-licensed Vantage MC 0.15.1 renderer from commit
`953f0ac78d62de751a935b8e8d5be2d711d6fd56`, with MSC's persistent Java
tile-cache change in `src/main.zig` and `src/texture.zig`. Its viewer package
remains the separately pinned `@thoughts-on-things/vantage-mc` 0.15.1 dependency.
MSC additionally fingerprints each compressed chunk payload so a saved-map
refresh invalidates tiles around changed chunks instead of every tile sharing
the same 32×32-chunk region file. This changes MSC's persistent cache format;
older tile entries are discarded once when the renderer is updated.

MSC builds the renderer with Zig 0.16.0 during agent staging. The source is
vendored so an installed or headless build never fetches a renderer with a
different tile-cache protocol. The original license is in `LICENSE`.
