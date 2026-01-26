import type { Atom, WritableAtom } from 'jotai/vanilla';
export declare function unwrap<Value, Args extends unknown[], Result>(anAtom: WritableAtom<Value, Args, Result>): WritableAtom<Awaited<Value> | undefined, Args, Result>;
export declare function unwrap<Value, Args extends unknown[], Result, PendingValue>(anAtom: WritableAtom<Value, Args, Result>, fallback: (prev?: Awaited<Value>) => PendingValue): WritableAtom<Awaited<Value> | PendingValue, Args, Result>;
export declare function unwrap<Value>(anAtom: Atom<Value>): Atom<Awaited<Value> | undefined>;
export declare function unwrap<Value, PendingValue>(anAtom: Atom<Value>, fallback: (prev?: Awaited<Value>) => PendingValue): Atom<Awaited<Value> | PendingValue>;
