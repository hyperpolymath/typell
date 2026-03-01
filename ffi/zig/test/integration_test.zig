// TYPELL Integration Tests
// SPDX-License-Identifier: PMPL-1.0-or-later
//
// These tests verify that the Zig FFI correctly implements the Idris2 ABI

const std = @import("std");
const testing = std.testing;

// Import FFI functions
extern fn typell_init() ?*opaque {};
extern fn typell_free(?*opaque {}) void;
extern fn typell_process(?*opaque {}, u32) c_int;
extern fn typell_get_string(?*opaque {}) ?[*:0]const u8;
extern fn typell_free_string(?[*:0]const u8) void;
extern fn typell_last_error() ?[*:0]const u8;
extern fn typell_version() [*:0]const u8;
extern fn typell_is_initialized(?*opaque {}) u32;

//==============================================================================
// Lifecycle Tests
//==============================================================================

test "create and destroy handle" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    try testing.expect(handle != null);
}

test "handle is initialized" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    const initialized = typell_is_initialized(handle);
    try testing.expectEqual(@as(u32, 1), initialized);
}

test "null handle is not initialized" {
    const initialized = typell_is_initialized(null);
    try testing.expectEqual(@as(u32, 0), initialized);
}

//==============================================================================
// Operation Tests
//==============================================================================

test "process with valid handle" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    const result = typell_process(handle, 42);
    try testing.expectEqual(@as(c_int, 0), result); // 0 = ok
}

test "process with null handle returns error" {
    const result = typell_process(null, 42);
    try testing.expectEqual(@as(c_int, 4), result); // 4 = null_pointer
}

//==============================================================================
// String Tests
//==============================================================================

test "get string result" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    const str = typell_get_string(handle);
    defer if (str) |s| typell_free_string(s);

    try testing.expect(str != null);
}

test "get string with null handle" {
    const str = typell_get_string(null);
    try testing.expect(str == null);
}

//==============================================================================
// Error Handling Tests
//==============================================================================

test "last error after null handle operation" {
    _ = typell_process(null, 0);

    const err = typell_last_error();
    try testing.expect(err != null);

    if (err) |e| {
        const err_str = std.mem.span(e);
        try testing.expect(err_str.len > 0);
    }
}

test "no error after successful operation" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    _ = typell_process(handle, 0);

    // Error should be cleared after successful operation
    // (This depends on implementation)
}

//==============================================================================
// Version Tests
//==============================================================================

test "version string is not empty" {
    const ver = typell_version();
    const ver_str = std.mem.span(ver);

    try testing.expect(ver_str.len > 0);
}

test "version string is semantic version format" {
    const ver = typell_version();
    const ver_str = std.mem.span(ver);

    // Should be in format X.Y.Z
    try testing.expect(std.mem.count(u8, ver_str, ".") >= 1);
}

//==============================================================================
// Memory Safety Tests
//==============================================================================

test "multiple handles are independent" {
    const h1 = typell_init() orelse return error.InitFailed;
    defer typell_free(h1);

    const h2 = typell_init() orelse return error.InitFailed;
    defer typell_free(h2);

    try testing.expect(h1 != h2);

    // Operations on h1 should not affect h2
    _ = typell_process(h1, 1);
    _ = typell_process(h2, 2);
}

test "double free is safe" {
    const handle = typell_init() orelse return error.InitFailed;

    typell_free(handle);
    typell_free(handle); // Should not crash
}

test "free null is safe" {
    typell_free(null); // Should not crash
}

//==============================================================================
// Thread Safety Tests (if applicable)
//==============================================================================

test "concurrent operations" {
    const handle = typell_init() orelse return error.InitFailed;
    defer typell_free(handle);

    const ThreadContext = struct {
        h: *opaque {},
        id: u32,
    };

    const thread_fn = struct {
        fn run(ctx: ThreadContext) void {
            _ = typell_process(ctx.h, ctx.id);
        }
    }.run;

    var threads: [4]std.Thread = undefined;
    for (&threads, 0..) |*thread, i| {
        thread.* = try std.Thread.spawn(.{}, thread_fn, .{
            ThreadContext{ .h = handle, .id = @intCast(i) },
        });
    }

    for (threads) |thread| {
        thread.join();
    }
}
