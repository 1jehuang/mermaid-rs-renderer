import type { PrimitiveAtom } from 'jotai/vanilla';
export declare function atomWithLazy<Value>(makeInitial: () => Value): PrimitiveAtom<Value>;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;