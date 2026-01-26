import type { WritableAtom } from 'jotai/vanilla';
type Read<Value, Args extends unknown[], Result> = WritableAtom<Value, Args, Result>['read'];
type Write<Value, Args extends unknown[], Result> = WritableAtom<Value, Args, Result>['write'];
export declare function atomWithRefresh<Value, Args extends unknown[], Result>(read: Read<Value, Args, Result>, write: Write<Value, Args, Result>): WritableAtom<Value, Args | [], Result | void>;
export declare function atomWithRefresh<Value>(read: Read<Value, [], void>): WritableAtom<Value, [], void>;
export {};
