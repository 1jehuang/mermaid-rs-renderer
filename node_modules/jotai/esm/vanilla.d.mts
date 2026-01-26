export { atom } from './vanilla/atom.mjs';
export type { Atom, WritableAtom, PrimitiveAtom } from './vanilla/atom.mjs';
export { createStore, getDefaultStore, INTERNAL_overrideCreateStore, } from './vanilla/store.mjs';
export type { Getter, Setter, ExtractAtomValue, ExtractAtomArgs, ExtractAtomResult, SetStateAction, } from './vanilla/typeUtils.mjs';
