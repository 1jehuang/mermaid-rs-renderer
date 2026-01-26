import type { Atom, PrimitiveAtom, WritableAtom } from 'jotai/vanilla';
type SplitAtomAction<Item> = {
    type: 'remove';
    atom: PrimitiveAtom<Item>;
} | {
    type: 'insert';
    value: Item;
    before?: PrimitiveAtom<Item>;
} | {
    type: 'move';
    atom: PrimitiveAtom<Item>;
    before?: PrimitiveAtom<Item>;
};
export declare function splitAtom<Item, Key>(arrAtom: WritableAtom<Item[], [
    Item[]
], void>, keyExtractor?: (item: Item) => Key): WritableAtom<PrimitiveAtom<Item>[], [
    SplitAtomAction<Item>
], void>;
export declare function splitAtom<Item, Key>(arrAtom: Atom<Item[]>, keyExtractor?: (item: Item) => Key): Atom<Atom<Item>[]>;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;