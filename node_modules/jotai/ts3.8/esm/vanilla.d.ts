export { atom } from './vanilla/atom';
export type { Atom, WritableAtom, PrimitiveAtom } from './vanilla/atom';
export { createStore, getDefaultStore, INTERNAL_overrideCreateStore, } from './vanilla/store';
export type { Getter, Setter, ExtractAtomValue, ExtractAtomArgs, ExtractAtomResult, SetStateAction, } from './vanilla/typeUtils';
declare type Awaited<T> = T extends Promise<infer V> ? V : T;