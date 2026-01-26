import type { ExtractAtomArgs, ExtractAtomResult, WritableAtom } from 'jotai/vanilla';
import { useStore } from './Provider';
type SetAtom<Args extends unknown[], Result> = (...args: Args) => Result;
type Options = Parameters<typeof useStore>[0];
export declare function useSetAtom<Value, Args extends unknown[], Result>(atom: WritableAtom<Value, Args, Result>, options?: Options): SetAtom<Args, Result>;
export declare function useSetAtom<AtomType extends WritableAtom<unknown, never[], unknown>>(atom: AtomType, options?: Options): SetAtom<ExtractAtomArgs<AtomType>, ExtractAtomResult<AtomType>>;
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;