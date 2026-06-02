// MermaidCacheTests.swift
// MermaidRenderKitTests

import Cmmdr
import XCTest

@testable import MermaidRenderKit

/// Bounded LRU cache (PRD §4.7 / PR7 / T12).
final class MermaidCacheTests: XCTestCase {

    private func opts(bg: UInt32) -> MmdrOptions {
        var o = MmdrOptions()
        o.abi_version = UInt32(MMDR_ABI_VERSION)
        o.struct_size = UInt32(MemoryLayout<MmdrOptions>.size)
        o.color_override_mask = 0b111
        o.background_rgba = bg
        return o
    }

    private let payload = MermaidRenderResult.success(payload: .svg("<svg/>"), intrinsicSize: .zero)

    func testGetMissThenSetThenHit() {
        let cache = MermaidRenderCache()
        let key = MermaidCacheKey(source: "flowchart LR\nA-->B", options: opts(bg: 1), format: .vectorSVG)
        XCTAssertNil(cache.get(key))
        cache.set(key, payload)
        guard case .success = cache.get(key) else {
            return XCTFail("expected a cached success")
        }
    }

    func testKeyIncludesResolvedRGBANotAdaptiveColor() {
        // Same source, different resolved background → different key → miss.
        let light = MermaidCacheKey(source: "x", options: opts(bg: 0xFFFF_FFFF), format: .vectorSVG)
        let dark = MermaidCacheKey(source: "x", options: opts(bg: 0x0000_00FF), format: .vectorSVG)
        XCTAssertNotEqual(light, dark)

        let cache = MermaidRenderCache()
        cache.set(light, payload)
        XCTAssertNotNil(cache.get(light))
        XCTAssertNil(cache.get(dark), "a color-scheme change (new RGBA) must miss")
    }

    func testFormatIsPartOfKey() {
        let svg = MermaidCacheKey(source: "x", options: opts(bg: 1), format: .vectorSVG)
        let png = MermaidCacheKey(source: "x", options: opts(bg: 1), format: .rasterPNG)
        XCTAssertNotEqual(svg, png)
    }

    func testLRUEvictsOldestBeyondCapacity() {
        let cache = MermaidRenderCache(maxEntries: 32)
        var keys: [MermaidCacheKey] = []
        for i in 0..<33 {
            let k = MermaidCacheKey(source: "n\(i)", options: opts(bg: UInt32(i)), format: .vectorSVG)
            keys.append(k)
            cache.set(k, payload)
        }
        XCTAssertEqual(cache.count, 32, "capacity is bounded at 32")
        XCTAssertNil(cache.get(keys[0]), "the 33rd insert evicts the oldest (key 0)")
        XCTAssertNotNil(cache.get(keys[32]), "the newest entry is retained")
    }

    func testGetMarksMostRecentlyUsedSoItSurvivesEviction() {
        let cache = MermaidRenderCache(maxEntries: 32)
        var keys: [MermaidCacheKey] = []
        for i in 0..<32 {
            let k = MermaidCacheKey(source: "n\(i)", options: opts(bg: UInt32(i)), format: .vectorSVG)
            keys.append(k)
            cache.set(k, payload)
        }
        _ = cache.get(keys[0]) // touch key 0 → most-recently-used
        let extra = MermaidCacheKey(source: "extra", options: opts(bg: 999), format: .vectorSVG)
        cache.set(extra, payload) // evicts the now-oldest (key 1), not key 0
        XCTAssertNotNil(cache.get(keys[0]), "recently-used key 0 must survive")
        XCTAssertNil(cache.get(keys[1]), "key 1 is now the oldest and is evicted")
    }

    // T12: a render through the guard is cached; the synchronous lookup returns
    // it without re-running the renderer (no placeholder transition).
    func testGuardCachesAndSyncLookupReturnsIt() async {
        MermaidRenderCache.shared.removeAll()
        let options = opts(bg: 0x1122_33FF)
        XCTAssertNil(MermaidRenderGuard.cachedResult(code: "flowchart LR\nA-->B", format: .vectorSVG, options: options))
        let first = await MermaidRenderGuard.render(code: "flowchart LR\nA-->B", format: .vectorSVG, options: options)
        guard case .success = first else { return XCTFail("expected success") }
        let cached = MermaidRenderGuard.cachedResult(code: "flowchart LR\nA-->B", format: .vectorSVG, options: options)
        XCTAssertNotNil(cached, "the result must be cached for a synchronous hit (T12)")
        MermaidRenderCache.shared.removeAll()
    }
}
