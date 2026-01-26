import type { Atom } from 'jotai/vanilla';
export declare function selectAtom<Value, Slice>(anAtom: Atom<Value>, selector: (v: Value, prevSlice?: Slice) => Slice, equalityFn?: (a: Slice, b: Slice) => boolean): Atom<Slice>;
