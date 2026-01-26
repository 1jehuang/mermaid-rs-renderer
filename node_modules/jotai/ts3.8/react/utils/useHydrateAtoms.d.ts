import { useStore } from 'jotai/react';
import type { WritableAtom } from 'jotai/vanilla';
type Options = Parameters<typeof useStore>[0] & {
    dangerouslyForceHydrate?: boolean;
};
type AnyWritableAtom = WritableAtom<any, any[], any>;
type InferAtomTuples<T> = {
    [K in keyof T]: T[K] extends readonly [
        infer A,
        ...infer Rest
    ] ? A extends WritableAtom<unknown, infer Args, unknown> ? Rest extends Args ? readonly [
        A,
        ...Rest
    ] : never : T[K] : never;
};
export type INTERNAL_InferAtomTuples<T> = InferAtomTuples<T>;
export declare function useHydrateAtoms<T extends (readonly [
    AnyWritableAtom,
    ...unknown[]
])[]>(values: InferAtomTuples<T>, options?: Options): void;
export declare function useHydrateAtoms<T extends Map<AnyWritableAtom, unknown>>(values: T, options?: Options): void;
export declare function useHydrateAtoms<T extends Iterable<readonly [
    AnyWritableAtom,
    ...unknown[]
]>>(values: InferAtomTuples<T>, options?: Options): void;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;