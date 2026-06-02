// MermaidCache.swift
// MermaidRenderKit

import Cmmdr
import Foundation
import os

/// Cache key for a native render (PRD §4.7 cache / PR7). Keyed on the source and
/// the RESOLVED render inputs — concrete RGBA, NOT an adaptive `Color`, and NOT
/// a `ColorScheme` directly (the scheme already determines the RGBA). A scheme
/// change therefore changes the resolved RGBA and so produces a cache miss.
public struct MermaidCacheKey: Hashable, Sendable {
    public let source: String
    public let backgroundRGBA: UInt32
    public let foregroundRGBA: UInt32
    public let accentRGBA: UInt32
    public let colorOverrideMask: UInt32
    public let baseTheme: Int32
    public let nodeSpacing: Float
    public let rankSpacing: Float
    public let format: MermaidDisplayFormat

    /// Build a key from the source, the resolved options, and the format.
    public init(source: String, options: MmdrOptions?, format: MermaidDisplayFormat) {
        self.source = source
        self.backgroundRGBA = options?.background_rgba ?? 0
        self.foregroundRGBA = options?.foreground_rgba ?? 0
        self.accentRGBA = options?.accent_rgba ?? 0
        self.colorOverrideMask = options?.color_override_mask ?? 0
        self.baseTheme = options?.base_theme ?? 0
        self.nodeSpacing = options?.node_spacing ?? 0
        self.rankSpacing = options?.rank_spacing ?? 0
        self.format = format
    }
}

/// A bounded LRU cache of native render results (PRD §4.7 / PR7).
///
/// Caches the `MermaidRenderResult` so a repeat of the same source+inputs avoids
/// re-running the Rust renderer (the FFI-level cache — MF-5). A manual LRU is
/// used, NOT `NSCache`, for deterministic, testable eviction. Thread-safe behind
/// an `OSAllocatedUnfairLock`.
public final class MermaidRenderCache: @unchecked Sendable {

    /// Process-wide cache consulted by the render guard.
    public static let shared = MermaidRenderCache()

    private struct State {
        var entries: [MermaidCacheKey: MermaidRenderResult] = [:]
        var order: [MermaidCacheKey] = [] // least-recently-used first
    }

    private let maxEntries: Int
    private let state: OSAllocatedUnfairLock<State>

    // Internal: render calls use the `shared` singleton. A standalone instance
    // (constructed and then discarded) would never be consulted, so the
    // initializer is not public. Tests construct isolated caches via @testable.
    init(maxEntries: Int = 32) {
        self.maxEntries = max(1, maxEntries)
        self.state = OSAllocatedUnfairLock(initialState: State())
    }

    /// Return the cached result for `key`, marking it most-recently-used.
    public func get(_ key: MermaidCacheKey) -> MermaidRenderResult? {
        state.withLock { state in
            guard let value = state.entries[key] else { return nil }
            touch(&state, key)
            return value
        }
    }

    /// Store `value` for `key`, evicting the least-recently-used entry when full.
    public func set(_ key: MermaidCacheKey, _ value: MermaidRenderResult) {
        state.withLock { state in
            if state.entries[key] == nil {
                while state.entries.count >= maxEntries, let oldest = state.order.first {
                    state.order.removeFirst()
                    state.entries.removeValue(forKey: oldest)
                }
            }
            state.entries[key] = value
            touch(&state, key)
        }
    }

    /// Current entry count (for tests/introspection).
    public var count: Int {
        state.withLock { $0.entries.count }
    }

    /// Empty the cache.
    public func removeAll() {
        state.withLock { $0 = State() }
    }

    private func touch(_ state: inout State, _ key: MermaidCacheKey) {
        state.order.removeAll { $0 == key }
        state.order.append(key)
    }
}
