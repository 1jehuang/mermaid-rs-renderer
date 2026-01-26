import type { Atom } from 'jotai/vanilla';
export type Loadable<Value> = {
    state: 'loading';
} | {
    state: 'hasError';
    error: unknown;
} | {
    state: 'hasData';
    data: Awaited<Value>;
};
export declare function loadable<Value>(anAtom: Atom<Value>): Atom<Loadable<Value>>;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;