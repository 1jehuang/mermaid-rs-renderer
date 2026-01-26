import type { WritableAtom } from 'jotai/vanilla';
import { RESET } from './constants';
type SetStateActionWithReset<Value> = Value | typeof RESET | ((prev: Value) => Value | typeof RESET);
type WithInitialValue<Value> = {
    init: Value;
};
export declare function atomWithReset<Value>(initialValue: Value): WritableAtom<Value, [
    SetStateActionWithReset<Value>
], void> & WithInitialValue<Value>;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;