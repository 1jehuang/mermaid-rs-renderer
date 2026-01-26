import type { Atom, ExtractAtomArgs, ExtractAtomResult, ExtractAtomValue, PrimitiveAtom, SetStateAction, WritableAtom } from 'jotai/vanilla';
import { useAtomValue } from './useAtomValue.mjs';
type SetAtom<Args extends unknown[], Result> = (...args: Args) => Result;
type Options = Parameters<typeof useAtomValue>[1];
export declare function useAtom<Value, Args extends unknown[], Result>(atom: WritableAtom<Value, Args, Result>, options?: Options): [Awaited<Value>, SetAtom<Args, Result>];
export declare function useAtom<Value>(atom: PrimitiveAtom<Value>, options?: Options): [Awaited<Value>, SetAtom<[SetStateAction<Value>], void>];
export declare function useAtom<Value>(atom: Atom<Value>, options?: Options): [Awaited<Value>, never];
export declare function useAtom<AtomType extends WritableAtom<unknown, never[], unknown>>(atom: AtomType, options?: Options): [
    Awaited<ExtractAtomValue<AtomType>>,
    SetAtom<ExtractAtomArgs<AtomType>, ExtractAtomResult<AtomType>>
];
export declare function useAtom<AtomType extends Atom<unknown>>(atom: AtomType, options?: Options): [Awaited<ExtractAtomValue<AtomType>>, never];
export {};
