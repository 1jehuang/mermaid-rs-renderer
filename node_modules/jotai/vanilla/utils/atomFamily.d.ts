import { type Atom } from 'jotai/vanilla';
/**
 * in milliseconds
 */
type CreatedAt = number;
type ShouldRemove<Param> = (createdAt: CreatedAt, param: Param) => boolean;
type Cleanup = () => void;
type Callback<Param, AtomType> = (event: {
    type: 'CREATE' | 'REMOVE';
    param: Param;
    atom: AtomType;
}) => void;
/**
 * @deprecated atomFamily is deprecated and will be removed in v3.
 * Please use the `jotai-family` package instead: https://github.com/jotaijs/jotai-family
 *
 * Install: `npm install jotai-family`
 *
 * Migration:
 * ```ts
 * // Before
 * import { atomFamily } from 'jotai/utils'
 *
 * // After
 * import { atomFamily } from 'jotai-family'
 * ```
 */
export interface AtomFamily<Param, AtomType> {
    (param: Param): AtomType;
    getParams(): Iterable<Param>;
    remove(param: Param): void;
    setShouldRemove(shouldRemove: ShouldRemove<Param> | null): void;
    /**
     * fires when an atom is created or removed
     * This API is for advanced use cases, and can change without notice.
     */
    unstable_listen(callback: Callback<Param, AtomType>): Cleanup;
}
/**
 * @deprecated atomFamily is deprecated and will be removed in v3.
 * Please use the `jotai-family` package instead: https://github.com/jotaijs/jotai-family
 *
 * Install: `npm install jotai-family`
 *
 * Migration:
 * ```ts
 * // Before
 * import { atomFamily } from 'jotai/utils'
 *
 * // After
 * import { atomFamily } from 'jotai-family'
 * ```
 */
export declare function atomFamily<Param, AtomType extends Atom<unknown>>(initializeAtom: (param: Param) => AtomType, areEqual?: (a: Param, b: Param) => boolean): AtomFamily<Param, AtomType>;
export {};
