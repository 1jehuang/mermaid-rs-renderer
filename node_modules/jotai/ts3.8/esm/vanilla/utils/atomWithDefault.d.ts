import type { WritableAtom } from 'jotai/vanilla';
import { RESET } from './constants';
type Read<Value, Args extends unknown[], Result> = WritableAtom<Value, Args, Result>['read'];
type DefaultSetStateAction<Value> = Value | typeof RESET | ((prev: Value) => Value | typeof RESET);
export declare function atomWithDefault<Value>(getDefault: Read<Value, [
    DefaultSetStateAction<Value>
], void>): WritableAtom<Value, [
    DefaultSetStateAction<Value>
], void>;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;