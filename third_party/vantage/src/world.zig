//! World-level helpers for the friendly `render` command.
//!
//! Locates a save's overworld region directory (classic `<save>/region` or the
//! newer `<save>/dimensions/minecraft/overworld/region`), loads its region
//! files once, scans the populated chunk bounds from their location tables, and
//! assembles a block grid spanning many regions — so cross-region face culling
//! is automatic, with no seams between regions.

const std = @import("std");
const region = @import("region.zig");
const grid = @import("grid.zig");
const chunk = @import("chunk.zig");
const compress = @import("compress.zig");
const nbt = @import("nbt.zig");

pub const LoadedRegion = struct {
    x: i32,
    z: i32,
    /// The file's location table — its first 4 KiB (shorter if truncated).
    /// Chunk payloads are NOT held here: they're read from `path` per tile
    /// window, so memory stays flat no matter how many regions a world has.
    table: []const u8,
    path: []const u8,
    /// Changes whenever the region's location table or file metadata changes.
    revision: u64,
    /// Content revision for each region-local chunk (z*32+x). Zero means the
    /// location table says that chunk is absent. Kept with the location table
    /// so tile revisions can distinguish edits within one large region file.
    chunk_revisions: []const u64 = &.{},
};

pub const ChunkBounds = struct {
    min_cx: i32 = 0,
    min_cz: i32 = 0,
    max_cx: i32 = 0,
    max_cz: i32 = 0,
    count: usize = 0,

    pub fn spanX(self: ChunkBounds) i32 {
        return self.max_cx - self.min_cx + 1;
    }
    pub fn spanZ(self: ChunkBounds) i32 {
        return self.max_cz - self.min_cz + 1;
    }
};

const FINGERPRINT_READ_BYTES = 8 * 1024 * 1024;
const ChunkLocation = struct { index: usize, start: usize, end: usize };

/// Find the overworld region directory for a save path. Accepts a save dir
/// (either layout) or a region dir handed in directly. Returns null if none has
/// `.mca` files.
pub fn findRegionDir(arena: std.mem.Allocator, io: std.Io, save_dir: []const u8) !?[]const u8 {
    const subs = [_][]const u8{
        "", // already a region dir
        "/region",
        "/dimensions/minecraft/overworld/region",
    };
    for (subs) |sub| {
        const path = try std.fmt.allocPrint(arena, "{s}{s}", .{ save_dir, sub });
        if (try dirHasMca(io, path)) return path;
    }
    return null;
}

fn dirHasMca(io: std.Io, path: []const u8) !bool {
    var dir = std.Io.Dir.cwd().openDir(io, path, .{ .iterate = true }) catch return false;
    defer dir.close(io);
    var it = dir.iterate();
    while (try it.next(io)) |e| {
        if (e.kind == .file and parseRegionName(e.name) != null) return true;
    }
    return false;
}

/// Index every `r.X.Z.mca` file in a region directory (once). Only the 4 KiB
/// location table of each file is read here — a large world's region set can
/// be tens of GB, which used to be loaded whole and held for the entire
/// render, OOM-killing the process. Payloads stream in `assembleWindow`.
pub fn loadRegions(arena: std.mem.Allocator, io: std.Io, region_dir: []const u8) ![]LoadedRegion {
    var out: std.ArrayList(LoadedRegion) = .empty;
    const fingerprint_buffer = try std.heap.page_allocator.alloc(u8, FINGERPRINT_READ_BYTES);
    defer std.heap.page_allocator.free(fingerprint_buffer);
    var dir = try std.Io.Dir.cwd().openDir(io, region_dir, .{ .iterate = true });
    defer dir.close(io);
    var it = dir.iterate();
    while (try it.next(io)) |e| {
        if (e.kind != .file) continue;
        const xz = parseRegionName(e.name) orelse continue;
        const path = try std.fmt.allocPrint(arena, "{s}/{s}", .{ region_dir, e.name });
        const snapshot = readStableLocationTable(arena, io, path, fingerprint_buffer) catch |err| switch (err) {
            // An unstable file is expected during a Minecraft save, but it must
            // fail the whole candidate epoch. Publishing a catalog with that
            // region silently missing would create a transient map deletion.
            error.RegionChangedDuringRead => return err,
            else => continue,
        };
        try out.append(arena, .{
            .x = xz[0],
            .z = xz[1],
            .table = snapshot.table,
            .path = path,
            .revision = snapshot.revision,
            .chunk_revisions = try arena.dupe(u64, &snapshot.chunk_revisions),
        });
    }
    std.mem.sort(LoadedRegion, out.items, {}, struct {
        fn lessThan(_: void, lhs: LoadedRegion, rhs: LoadedRegion) bool {
            return lhs.z < rhs.z or (lhs.z == rhs.z and lhs.x < rhs.x);
        }
    }.lessThan);
    return out.toOwnedSlice(arena);
}

/// Aggregate identity of a region catalog. Because `loadRegions` sorts by
/// coordinate, the result is independent of directory enumeration order.
pub fn catalogRevision(regions: []const LoadedRegion) u64 {
    var hasher = std.hash.Wyhash.init(0x5641_4e54_4147_4531); // "VANTAGE1"
    for (regions) |r| {
        hasher.update(std.mem.asBytes(&r.x));
        hasher.update(std.mem.asBytes(&r.z));
        hasher.update(std.mem.asBytes(&r.revision));
    }
    return hasher.final();
}

/// Cheap change gate for a long-running server scan: filenames, sizes and
/// mtimes only. Location tables are re-read only when this value advances.
pub fn catalogMetadataRevision(arena: std.mem.Allocator, io: std.Io, region_dir: []const u8) !u64 {
    // mtime widens to i128: @sizeOf(i96) is 16 but only 12 bytes are defined
    // on store, so hashing an i96's bytes would fold undefined padding into
    // the fingerprint and could spuriously "change" the catalog every scan.
    const Metadata = struct { x: i32, z: i32, size: u64, mtime: i128 };
    var entries: std.ArrayList(Metadata) = .empty;
    var dir = try std.Io.Dir.cwd().openDir(io, region_dir, .{ .iterate = true });
    defer dir.close(io);
    var it = dir.iterate();
    while (try it.next(io)) |entry| {
        if (entry.kind != .file) continue;
        const xz = parseRegionName(entry.name) orelse continue;
        const stat = dir.statFile(io, entry.name, .{}) catch continue;
        try entries.append(arena, .{ .x = xz[0], .z = xz[1], .size = stat.size, .mtime = stat.mtime.nanoseconds });
    }
    std.mem.sort(Metadata, entries.items, {}, struct {
        fn lessThan(_: void, lhs: Metadata, rhs: Metadata) bool {
            return lhs.z < rhs.z or (lhs.z == rhs.z and lhs.x < rhs.x);
        }
    }.lessThan);
    var hasher = std.hash.Wyhash.init(0x5641_4e54_4d45_5441); // "VANTMETA"
    for (entries.items) |entry| {
        // Hash fields individually: hashing the struct itself would include
        // padding bytes whose contents are not part of the metadata identity.
        hasher.update(std.mem.asBytes(&entry.x));
        hasher.update(std.mem.asBytes(&entry.z));
        hasher.update(std.mem.asBytes(&entry.size));
        hasher.update(std.mem.asBytes(&entry.mtime));
    }
    return hasher.final();
}

pub const RegionRevisionIndex = std.AutoHashMap(u64, usize);

pub fn indexRegionRevisions(arena: std.mem.Allocator, regions: []const LoadedRegion) !RegionRevisionIndex {
    var index = RegionRevisionIndex.init(arena);
    for (regions, 0..) |r, i| try index.put(packChunk(r.x, r.z), i);
    return index;
}

/// Revision of one Vantage tile, including its one-chunk seam apron. Chunk
/// payload content, rather than the containing .mca file, controls invalidation.
pub fn tileRevision(regions: []const LoadedRegion, tx: i32, tz: i32, tile_chunks: i32) u64 {
    const cx0 = tx * tile_chunks - 1;
    const cz0 = tz * tile_chunks - 1;
    const cx1 = (tx + 1) * tile_chunks;
    const cz1 = (tz + 1) * tile_chunks;
    var hasher = std.hash.Wyhash.init(packChunk(tx, tz));
    for (regions) |r| {
        const rcx0 = r.x * 32;
        const rcz0 = r.z * 32;
        if (rcx0 + 31 < cx0 or rcx0 > cx1 or rcz0 + 31 < cz0 or rcz0 > cz1) continue;
        const x0 = @max(cx0, rcx0);
        const z0 = @max(cz0, rcz0);
        const x1 = @min(cx1, rcx0 + 31);
        const z1 = @min(cz1, rcz0 + 31);
        for (z0..z1 + 1) |cz| for (x0..x1 + 1) |cx| {
            const lx: usize = @intCast(cx - rcx0);
            const lz: usize = @intCast(cz - rcz0);
            const revision = r.chunk_revisions[lz * 32 + lx];
            if (revision == 0) continue;
            const chunk_x: i32 = @intCast(cx);
            const chunk_z: i32 = @intCast(cz);
            hasher.update(std.mem.asBytes(&chunk_x));
            hasher.update(std.mem.asBytes(&chunk_z));
            hasher.update(std.mem.asBytes(&revision));
        };
    }
    // Zero is reserved for non-revising local live sessions.
    return hasher.final() | 1;
}

/// Indexed form used while building a live server epoch. The lookup is bounded
/// by the tile footprint and its seam apron, independent of total world regions.
pub fn tileRevisionIndexed(index: *const RegionRevisionIndex, regions: []const LoadedRegion, tx: i32, tz: i32, tile_chunks: i32) u64 {
    const cx0 = tx * tile_chunks - 1;
    const cz0 = tz * tile_chunks - 1;
    const cx1 = (tx + 1) * tile_chunks;
    const cz1 = (tz + 1) * tile_chunks;
    var hasher = std.hash.Wyhash.init(packChunk(tx, tz));
    var cz = cz0;
    while (cz <= cz1) : (cz += 1) {
        var cx = cx0;
        while (cx <= cx1) : (cx += 1) {
            const rx = @divFloor(cx, 32);
            const rz = @divFloor(cz, 32);
            const region_index = index.get(packChunk(rx, rz)) orelse continue;
            const r = regions[region_index];
            const lx: usize = @intCast(@mod(cx, 32));
            const lz: usize = @intCast(@mod(cz, 32));
            const revision = r.chunk_revisions[lz * 32 + lx];
            if (revision == 0) continue;
            hasher.update(std.mem.asBytes(&cx));
            hasher.update(std.mem.asBytes(&cz));
            hasher.update(std.mem.asBytes(&revision));
        }
    }
    return hasher.final() | 1;
}

fn readLocationTable(io: std.Io, path: []const u8, buf: *[region.SECTOR]u8) ![]const u8 {
    var file = try std.Io.Dir.cwd().openFile(io, path, .{});
    defer file.close(io);
    const n = try file.readPositionalAll(io, buf, 0);
    return buf[0..n];
}

const LocationSnapshot = struct {
    table: []const u8,
    revision: u64,
    chunk_revisions: [1024]u64,
};

/// Minecraft may be appending sectors while Vantage scans. Read metadata on
/// both sides of the 4 KiB table and retry (with a short backoff, so a burst
/// of writes to this file can finish) if it moved underneath us; after the
/// last attempt the whole candidate epoch fails rather than publishing torn
/// state — a later server scan retries, a one-shot render reports the error.
/// Region payloads stream through one bounded shared buffer; the arena retains
/// only the location table and compact per-chunk revisions for this snapshot.
fn readStableLocationTable(arena: std.mem.Allocator, io: std.Io, path: []const u8, payload_buf: []u8) !LocationSnapshot {
    var buf: [region.SECTOR]u8 = undefined;
    var attempt: usize = 0;
    while (attempt < 4) : (attempt += 1) {
        if (attempt > 0) std.Io.sleep(io, std.Io.Duration.fromMilliseconds(15), .awake) catch {};
        const before = try std.Io.Dir.cwd().statFile(io, path, .{});
        @memset(buf[0..], 0);
        const table = try readLocationTable(io, path, &buf);
        var chunk_revisions = [_]u64{0} ** 1024;
        {
            var file = try std.Io.Dir.cwd().openFile(io, path, .{});
            defer file.close(io);
            var locations: [1024]ChunkLocation = undefined;
            var location_count: usize = 0;
            for (0..1024) |i| {
                const e = i * 4;
                const off = (@as(u32, buf[e]) << 16) | (@as(u32, buf[e + 1]) << 8) | buf[e + 2];
                const sectors = buf[e + 3];
                if (off == 0 and sectors == 0) continue;
                const start_u64 = @as(u64, off) * region.SECTOR;
                const end_u64 = start_u64 + @as(u64, sectors) * region.SECTOR;
                if (sectors == 0 or off < 2 or end_u64 > before.size) {
                    chunk_revisions[i] = std.hash.Wyhash.hash(0x4348_554e_4b31, buf[e .. e + 4]) | 1;
                    continue;
                }
                locations[location_count] = .{ .index = i, .start = @intCast(start_u64), .end = @intCast(end_u64) };
                location_count += 1;
            }
            std.mem.sort(ChunkLocation, locations[0..location_count], {}, struct {
                fn lessThan(_: void, lhs: ChunkLocation, rhs: ChunkLocation) bool {
                    return lhs.start < rhs.start;
                }
            }.lessThan);
            var first: usize = 0;
            while (first < location_count) {
                var last = first + 1;
                var end = locations[first].end;
                while (last < location_count and locations[last].end - locations[first].start <= payload_buf.len) : (last += 1) {
                    end = @max(end, locations[last].end);
                }
                const start = locations[first].start;
                const read_len = end - start;
                const n = try file.readPositionalAll(io, payload_buf[0..read_len], start);
                for (locations[first..last]) |location| {
                    if (location.end - start > n) continue;
                    const chunk_bytes = payload_buf[location.start - start .. location.end - start];
                    chunk_revisions[location.index] = std.hash.Wyhash.hash(0x4348_554e_4b31, chunk_bytes) | 1;
                }
                first = last;
            }
        }
        const after = try std.Io.Dir.cwd().statFile(io, path, .{});
        if (before.size != after.size or before.mtime.nanoseconds != after.mtime.nanoseconds) continue;
        const timestamp_bits: u96 = @bitCast(after.mtime.nanoseconds);
        const seed = after.size ^ @as(u64, @truncate(timestamp_bits));
        return .{
            .table = try arena.dupe(u8, table),
            .revision = std.hash.Wyhash.hash(seed, table),
            .chunk_revisions = chunk_revisions,
        };
    }
    return error.RegionChangedDuringRead;
}

/// "r.-2.1.mca" -> {-2, 1}.
fn parseRegionName(name: []const u8) ?[2]i32 {
    if (!std.mem.startsWith(u8, name, "r.")) return null;
    if (!std.mem.endsWith(u8, name, ".mca")) return null;
    const mid = name["r.".len .. name.len - ".mca".len];
    const dot = std.mem.indexOfScalar(u8, mid, '.') orelse return null;
    const x = std.fmt.parseInt(i32, mid[0..dot], 10) catch return null;
    const z = std.fmt.parseInt(i32, mid[dot + 1 ..], 10) catch return null;
    return .{ x, z };
}

/// Bounding box + count of populated chunks, read from region location tables
/// already in memory (no extra I/O).
pub fn populatedBounds(regions: []const LoadedRegion) ChunkBounds {
    var b: ChunkBounds = .{
        .min_cx = std.math.maxInt(i32),
        .min_cz = std.math.maxInt(i32),
        .max_cx = std.math.minInt(i32),
        .max_cz = std.math.minInt(i32),
    };
    for (regions) |r| {
        if (r.table.len < region.SECTOR) continue;
        var i: usize = 0;
        while (i < 1024) : (i += 1) {
            const e = i * 4;
            const off = (@as(u32, r.table[e]) << 16) | (@as(u32, r.table[e + 1]) << 8) | r.table[e + 2];
            if (off == 0 and r.table[e + 3] == 0) continue;
            const cx = r.x * 32 + @as(i32, @intCast(i % 32));
            const cz = r.z * 32 + @as(i32, @intCast(i / 32));
            b.min_cx = @min(b.min_cx, cx);
            b.max_cx = @max(b.max_cx, cx);
            b.min_cz = @min(b.min_cz, cz);
            b.max_cz = @max(b.max_cz, cz);
            b.count += 1;
        }
    }
    if (b.count == 0) b = .{};
    return b;
}

/// Pack signed chunk coords into one map key.
pub fn packChunk(cx: i32, cz: i32) u64 {
    return (@as(u64, @as(u32, @bitCast(cx))) << 32) | @as(u32, @bitCast(cz));
}

/// The set of populated chunk coords (packed with `packChunk`), read from the
/// region location tables already in memory — no chunk I/O. Sparse worlds (a
/// few trails across hundreds of regions) tile-enumerate from this instead of
/// the bounding box, so empty tiles are never visited.
pub fn populatedChunks(arena: std.mem.Allocator, regions: []const LoadedRegion) !std.AutoHashMap(u64, void) {
    var set = std.AutoHashMap(u64, void).init(arena);
    for (regions) |r| {
        if (r.table.len < region.SECTOR) continue;
        var i: usize = 0;
        while (i < 1024) : (i += 1) {
            const e = i * 4;
            const off = (@as(u32, r.table[e]) << 16) | (@as(u32, r.table[e + 1]) << 8) | r.table[e + 2];
            if (off == 0 and r.table[e + 3] == 0) continue;
            const cx = r.x * 32 + @as(i32, @intCast(i % 32));
            const cz = r.z * 32 + @as(i32, @intCast(i / 32));
            try set.put(packChunk(cx, cz), {});
        }
    }
    return set;
}

/// World spawn from `<save>/level.dat` (gzip-wrapped NBT). Modern saves
/// (1.21.5+) store `Data.spawn.pos` as a 3-int array; older ones store
/// `Data.SpawnX/Y/Z`. Null when the file is absent or unreadable — callers
/// fall back to centring on the populated bounds.
pub fn readSpawn(arena: std.mem.Allocator, io: std.Io, save_dir: []const u8) ?[3]i32 {
    const path = std.fmt.allocPrint(arena, "{s}/level.dat", .{save_dir}) catch return null;
    const raw = std.Io.Dir.cwd().readFileAlloc(io, path, arena, .unlimited) catch return null;
    const bytes = compress.inflateGzip(arena, raw) catch return null;
    var parser = nbt.Parser{ .buf = bytes, .arena = arena };
    const root = parser.parseRoot() catch return null;
    const data = nbt.get(root, "Data") orelse return null;
    if (data.* != .compound) return null;

    // Modern layout: spawn { pos: [x, y, z], dimension, yaw, pitch }.
    if (nbt.get(data.compound, "spawn")) |sp| {
        if (sp.* == .compound) {
            if (nbt.get(sp.compound, "pos")) |pos| {
                if (pos.* == .int_array and pos.int_array.len >= 3) {
                    return .{ pos.int_array[0], pos.int_array[1], pos.int_array[2] };
                }
            }
        }
    }

    // Legacy layout: SpawnX / SpawnY / SpawnZ ints.
    const sx = nbt.get(data.compound, "SpawnX") orelse return null;
    const sy = nbt.get(data.compound, "SpawnY") orelse return null;
    const sz = nbt.get(data.compound, "SpawnZ") orelse return null;
    if (sx.* != .int or sy.* != .int or sz.* != .int) return null;
    return .{ sx.int, sy.int, sz.int };
}

/// Assemble a grid over the world-chunk window [cx0..cx1] x [cz0..cz1] (inclusive,
/// world chunk coords), reading from whichever loaded regions overlap it. Chunk
/// payloads are read from disk here (positional reads guided by the in-memory
/// location tables) into `arena` — a tile's window is the only chunk data ever
/// resident, so memory doesn't scale with world size. Decoded chunks share one
/// grid, so faces cull across region boundaries. `prog`, if given, advances
/// once per chunk attempted.
pub fn assembleWindow(
    arena: std.mem.Allocator,
    io: std.Io,
    loaded: []const LoadedRegion,
    cx0: i32,
    cz0: i32,
    cx1: i32,
    cz1: i32,
    stats: *grid.Stats,
    prog: ?std.Progress.Node,
) !grid.Grid {
    var chunks: std.ArrayList(chunk.Chunk) = .empty;
    for (loaded) |r| {
        const rcx0 = r.x * 32;
        const rcz0 = r.z * 32;
        const rcx1 = rcx0 + 31;
        const rcz1 = rcz0 + 31;
        if (rcx1 < cx0 or rcx0 > cx1 or rcz1 < cz0 or rcz0 > cz1) continue;
        // One handle per overlapping region for the whole window — workers on
        // other tiles hold their own, so there's no shared-seek state.
        var file = std.Io.Dir.cwd().openFile(io, r.path, .{}) catch {
            // The file vanished or turned unreadable since the scan; its
            // chunks are counted missing as the window walks them below.
            continue;
        };
        defer file.close(io);
        var wz = @max(cz0, rcz0);
        while (wz <= @min(cz1, rcz1)) : (wz += 1) {
            var wx = @max(cx0, rcx0);
            while (wx <= @min(cx1, rcx1)) : (wx += 1) {
                if (prog) |p| p.completeOne();
                const lx: u5 = @intCast(wx - rcx0);
                const lz: u5 = @intCast(wz - rcz0);
                const loc = region.locate(r.table, lx, lz) orelse {
                    stats.chunks_missing += 1;
                    continue;
                };
                const ch = decodeChunkAt(arena, io, file, loc) catch |err| {
                    switch (err) {
                        error.PreModernChunk => stats.chunks_premodern += 1,
                        error.UnsupportedCompression, error.ExternalChunk => stats.chunks_unreadable += 1,
                        else => {},
                    }
                    stats.chunks_missing += 1;
                    continue;
                } orelse {
                    stats.chunks_missing += 1;
                    continue;
                };
                if (ch.sections.len == 0) {
                    stats.chunks_missing += 1;
                    continue;
                }
                stats.chunks_loaded += 1;
                try chunks.append(arena, ch);
            }
        }
    }
    return grid.buildGrid(arena, chunks.items, stats);
}

/// Read one chunk's sectors from an open region file and decode it. The raw
/// sector buffer, NBT bytes, and decoded sections all come from `arena` (the
/// caller's per-tile arena), freed together when the tile completes.
fn decodeChunkAt(arena: std.mem.Allocator, io: std.Io, file: std.Io.File, loc: region.Loc) !?chunk.Chunk {
    const size: usize = @as(usize, loc.sector_count) * region.SECTOR;
    if (size == 0) return null;
    const buf = try arena.alloc(u8, size);
    const n = try file.readPositionalAll(io, buf, @as(u64, loc.offset_sectors) * region.SECTOR);
    const raw = (try region.rawChunkFromSectors(buf[0..n])) orelse return null;
    const nbt_bytes = try region.decompress(arena, raw);
    var parser = nbt.Parser{ .buf = nbt_bytes, .arena = arena };
    const root = try parser.parseRoot();
    return try chunk.decode(arena, root);
}

test "parseRegionName" {
    try std.testing.expectEqual([2]i32{ -2, 1 }, parseRegionName("r.-2.1.mca").?);
    try std.testing.expectEqual([2]i32{ 0, 0 }, parseRegionName("r.0.0.mca").?);
    try std.testing.expect(parseRegionName("r.0.mca") == null);
    try std.testing.expect(parseRegionName("level.dat") == null);
    try std.testing.expect(parseRegionName("r.0.0.mcc") == null);
}

test "tileRevision invalidates only changed chunks in its seam apron" {
    const table = [_]u8{0} ** region.SECTOR;
    const empty_revisions = [_]u64{0} ** 1024;
    const base = [_]LoadedRegion{
        .{ .x = 0, .z = 0, .table = &table, .path = "r.0.0.mca", .revision = 10, .chunk_revisions = &empty_revisions },
        .{ .x = 1, .z = 0, .table = &table, .path = "r.1.0.mca", .revision = 20, .chunk_revisions = &empty_revisions },
    };
    var revisions_0 = empty_revisions;
    var revisions_1 = empty_revisions;
    revisions_0[0] = 101;
    revisions_1[0] = 202;
    var changed = base;
    changed[0].chunk_revisions = &revisions_0;
    changed[1].chunk_revisions = &revisions_1;

    // Tile 0 spans chunks 0..7 (+ apron -1..8), nowhere near region x=1.
    try std.testing.expectEqual(tileRevision(&base, 0, 0, 8), tileRevision(&changed, 0, 0, 8));
    // A change to chunk (0,0) remains local to the tiles that overlap it.
    var local_revisions = empty_revisions;
    local_revisions[0] = 101;
    var local_change = base;
    local_change[0].chunk_revisions = &local_revisions;
    try std.testing.expect(tileRevision(&base, 0, 0, 8) != tileRevision(&local_change, 0, 0, 8));
    try std.testing.expectEqual(tileRevision(&base, 4, 0, 8), tileRevision(&local_change, 4, 0, 8));
}

test "indexed tile revisions match catalog scans" {
    const table = [_]u8{0} ** region.SECTOR;
    var chunk_revisions_0 = [_]u64{0} ** 1024;
    var chunk_revisions_1 = [_]u64{0} ** 1024;
    var chunk_revisions_2 = [_]u64{0} ** 1024;
    chunk_revisions_0[0] = 11;
    chunk_revisions_1[31] = 12;
    chunk_revisions_2[32 * 8 + 7] = 13;
    const regions = [_]LoadedRegion{
        .{ .x = -1, .z = 0, .table = &table, .path = "r.-1.0.mca", .revision = 3, .chunk_revisions = &chunk_revisions_0 },
        .{ .x = 0, .z = 0, .table = &table, .path = "r.0.0.mca", .revision = 5, .chunk_revisions = &chunk_revisions_1 },
        .{ .x = 1, .z = 1, .table = &table, .path = "r.1.1.mca", .revision = 7, .chunk_revisions = &chunk_revisions_2 },
    };
    var index = try indexRegionRevisions(std.testing.allocator, &regions);
    defer index.deinit();
    for ([_]i32{ -4, -1, 0, 3, 4, 7 }) |tx| {
        for ([_]i32{ -1, 0, 3, 4, 8 }) |tz| {
            try std.testing.expectEqual(tileRevision(&regions, tx, tz, 8), tileRevisionIndexed(&index, &regions, tx, tz, 8));
        }
    }
}
