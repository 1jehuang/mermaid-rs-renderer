import { useSetAtom } from 'jotai/react';
import { RESET } from 'jotai/vanilla/utils';
import type { WritableAtom } from 'jotai/vanilla';
type Options = Parameters<typeof useSetAtom>[1];
export declare function useResetAtom<T>(anAtom: WritableAtom<unknown, [
    typeof RESET
], T>, options?: Options): () => T;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;