import type { WritableAtom } from 'jotai/vanilla';
export declare function atomWithReducer<Value, Action>(initialValue: Value, reducer: (value: Value, action?: Action) => Value): WritableAtom<Value, [
    Action?
], void>;
export declare function atomWithReducer<Value, Action>(initialValue: Value, reducer: (value: Value, action: Action) => Value): WritableAtom<Value, [
    Action
], void>;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;