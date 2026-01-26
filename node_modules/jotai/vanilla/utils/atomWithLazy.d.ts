import type { PrimitiveAtom } from 'jotai/vanilla';
export declare function atomWithLazy<Value>(makeInitial: () => Value): PrimitiveAtom<Value>;
