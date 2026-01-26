import type { Atom } from 'jotai/vanilla';
export declare function freezeAtom<AtomType extends Atom<unknown>>(anAtom: AtomType): AtomType;
/**
 * @deprecated Define it on users end
 */
export declare function freezeAtomCreator<CreateAtom extends (...args: unknown[]) => Atom<unknown>>(createAtom: CreateAtom): CreateAtom;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;