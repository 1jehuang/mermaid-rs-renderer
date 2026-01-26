import { useSetAtom } from 'jotai/react';
import type { Getter, Setter } from 'jotai/vanilla';
type Options = Parameters<typeof useSetAtom>[1];
export declare function useAtomCallback<Result, Args extends unknown[]>(callback: (get: Getter, set: Setter, ...arg: Args) => Result, options?: Options): (...args: Args) => Result;
export {};
