import type { Atom, ExtractAtomValue } from 'jotai/vanilla';
import { useStore } from './Provider';
type Options = Parameters<typeof useStore>[0] & {
    delay?: number;
    unstable_promiseStatus?: boolean;
};
export declare function useAtomValue<Value>(atom: Atom<Value>, options?: Options): Awaited<Value>;
export declare function useAtomValue<AtomType extends Atom<unknown>>(atom: AtomType, options?: Options): Awaited<ExtractAtomValue<AtomType>>;
export {};
