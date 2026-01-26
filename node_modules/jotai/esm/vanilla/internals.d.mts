import type { Atom, WritableAtom } from './atom.mjs';
type AnyValue = unknown;
type AnyError = unknown;
type AnyAtom = Atom<AnyValue>;
type AnyWritableAtom = WritableAtom<AnyValue, unknown[], unknown>;
type OnUnmount = () => void;
type EpochNumber = number;
/**
 * Mutable atom state,
 * tracked for both mounted and unmounted atoms in a store.
 *
 * This should be garbage collectable.
 * We can mutate it during atom read. (except for fields with TODO)
 */
type AtomState<Value = AnyValue> = {
    /**
     * Map of atoms that the atom depends on.
     * The map value is the epoch number of the dependency.
     */
    readonly d: Map<AnyAtom, EpochNumber>;
    /**
     * Set of atoms with pending promise that depend on the atom.
     *
     * This may cause memory leaks, but it's for the capability to continue promises
     * TODO(daishi): revisit how to handle this
     */
    readonly p: Set<AnyAtom>;
    /** The epoch number of the atom. */
    n: EpochNumber;
    /** Atom value */
    v?: Value;
    /** Atom error */
    e?: AnyError;
};
/**
 * State tracked for mounted atoms. An atom is considered "mounted" if it has a
 * subscriber, or is a transitive dependency of another atom that has a
 * subscriber.
 * The mounted state of an atom is freed once it is no longer mounted.
 */
type Mounted = {
    /** Set of listeners to notify when the atom value changes. */
    readonly l: Set<() => void>;
    /** Set of mounted atoms that the atom depends on. */
    readonly d: Set<AnyAtom>;
    /** Set of mounted atoms that depends on the atom. */
    readonly t: Set<AnyAtom>;
    /** Function to run when the atom is unmounted. */
    u?: () => void;
};
type WeakMapLike<K extends object, V> = {
    get(key: K): V | undefined;
    set(key: K, value: V): void;
    has(key: K): boolean;
    delete(key: K): boolean;
};
type SetLike<T> = {
    readonly size: number;
    add(value: T): void;
    has(value: T): boolean;
    delete(value: T): boolean;
    clear(): void;
    forEach(callback: (value: T) => void): void;
    [Symbol.iterator](): IterableIterator<T>;
};
type AtomStateMap = WeakMapLike<AnyAtom, AtomState>;
type MountedMap = WeakMapLike<AnyAtom, Mounted>;
type InvalidatedAtoms = WeakMapLike<AnyAtom, EpochNumber>;
type ChangedAtoms = SetLike<AnyAtom>;
type Callbacks = SetLike<() => void>;
type AtomRead = <Value>(store: Store, atom: Atom<Value>, ...params: Parameters<Atom<Value>['read']>) => Value;
type AtomWrite = <Value, Args extends unknown[], Result>(store: Store, atom: WritableAtom<Value, Args, Result>, ...params: Parameters<WritableAtom<Value, Args, Result>['write']>) => Result;
type AtomOnInit = <Value>(store: Store, atom: Atom<Value>) => void;
type AtomOnMount = <Value, Args extends unknown[], Result>(store: Store, atom: WritableAtom<Value, Args, Result>, setAtom: (...args: Args) => Result) => OnUnmount | void;
type EnsureAtomState = <Value>(store: Store, atom: Atom<Value>) => AtomState<Value>;
type FlushCallbacks = (store: Store) => void;
type RecomputeInvalidatedAtoms = (store: Store) => void;
type ReadAtomState = <Value>(store: Store, atom: Atom<Value>) => AtomState<Value>;
type InvalidateDependents = (store: Store, atom: AnyAtom) => void;
type WriteAtomState = <Value, Args extends unknown[], Result>(store: Store, atom: WritableAtom<Value, Args, Result>, ...args: Args) => Result;
type MountDependencies = (store: Store, atom: AnyAtom) => void;
type MountAtom = <Value>(store: Store, atom: Atom<Value>) => Mounted;
type UnmountAtom = <Value>(store: Store, atom: Atom<Value>) => Mounted | undefined;
type SetAtomStateValueOrPromise = <Value>(store: Store, atom: Atom<Value>, valueOrPromise: Value) => void;
type StoreGet = <Value>(store: Store, atom: Atom<Value>) => Value;
type StoreSet = <Value, Args extends unknown[], Result>(store: Store, atom: WritableAtom<Value, Args, Result>, ...args: Args) => Result;
type StoreSub = (store: Store, atom: AnyAtom, listener: () => void) => () => void;
type EnhanceBuildingBlocks = (buildingBlocks: Readonly<BuildingBlocks>) => Readonly<BuildingBlocks>;
type Store = {
    get: <Value>(atom: Atom<Value>) => Value;
    set: <Value, Args extends unknown[], Result>(atom: WritableAtom<Value, Args, Result>, ...args: Args) => Result;
    sub: (atom: AnyAtom, listener: () => void) => () => void;
};
type BuildingBlocks = [
    atomStateMap: AtomStateMap,
    mountedMap: MountedMap,
    invalidatedAtoms: InvalidatedAtoms,
    changedAtoms: ChangedAtoms,
    mountCallbacks: Callbacks,
    unmountCallbacks: Callbacks,
    storeHooks: StoreHooks,
    atomRead: AtomRead,
    atomWrite: AtomWrite,
    atomOnInit: AtomOnInit,
    atomOnMount: AtomOnMount,
    ensureAtomState: EnsureAtomState,
    flushCallbacks: FlushCallbacks,
    recomputeInvalidatedAtoms: RecomputeInvalidatedAtoms,
    readAtomState: ReadAtomState,
    invalidateDependents: InvalidateDependents,
    writeAtomState: WriteAtomState,
    mountDependencies: MountDependencies,
    mountAtom: MountAtom,
    unmountAtom: UnmountAtom,
    setAtomStateValueOrPromise: SetAtomStateValueOrPromise,
    storeGet: StoreGet,
    storeSet: StoreSet,
    storeSub: StoreSub,
    enhanceBuildingBlocks: EnhanceBuildingBlocks | undefined
];
export type { AtomState as INTERNAL_AtomState, Mounted as INTERNAL_Mounted, AtomStateMap as INTERNAL_AtomStateMap, MountedMap as INTERNAL_MountedMap, InvalidatedAtoms as INTERNAL_InvalidatedAtoms, ChangedAtoms as INTERNAL_ChangedAtoms, Callbacks as INTERNAL_Callbacks, AtomRead as INTERNAL_AtomRead, AtomWrite as INTERNAL_AtomWrite, AtomOnInit as INTERNAL_AtomOnInit, AtomOnMount as INTERNAL_AtomOnMount, EnsureAtomState as INTERNAL_EnsureAtomState, FlushCallbacks as INTERNAL_FlushCallbacks, RecomputeInvalidatedAtoms as INTERNAL_RecomputeInvalidatedAtoms, ReadAtomState as INTERNAL_ReadAtomState, InvalidateDependents as INTERNAL_InvalidateDependents, WriteAtomState as INTERNAL_WriteAtomState, MountDependencies as INTERNAL_MountDependencies, MountAtom as INTERNAL_MountAtom, UnmountAtom as INTERNAL_UnmountAtom, Store as INTERNAL_Store, BuildingBlocks as INTERNAL_BuildingBlocks, StoreHooks as INTERNAL_StoreHooks, };
declare function hasInitialValue<T extends Atom<AnyValue>>(atom: T): atom is T & (T extends Atom<infer Value> ? {
    init: Value;
} : never);
declare function isActuallyWritableAtom(atom: AnyAtom): atom is AnyWritableAtom;
declare function isAtomStateInitialized<Value>(atomState: AtomState<Value>): boolean;
declare function returnAtomValue<Value>(atomState: AtomState<Value>): Value;
declare const promiseStateMap: WeakMap<PromiseLike<unknown>, [
    pending: boolean,
    abortHandlers: Set<() => void>
]>;
declare function isPendingPromise(value: unknown): value is PromiseLike<unknown>;
declare function abortPromise<T>(promise: PromiseLike<T>): void;
declare function registerAbortHandler<T>(promise: PromiseLike<T>, abortHandler: () => void): void;
declare function isPromiseLike(p: unknown): p is PromiseLike<unknown>;
declare function addPendingPromiseToDependency(atom: AnyAtom, promise: PromiseLike<AnyValue>, dependencyAtomState: AtomState): void;
declare function getMountedOrPendingDependents(atom: AnyAtom, atomState: AtomState, mountedMap: MountedMap): Set<AnyAtom>;
type StoreHook = {
    (): void;
    add(callback: () => void): () => void;
};
type StoreHookForAtoms = {
    (atom: AnyAtom): void;
    add(atom: AnyAtom, callback: () => void): () => void;
    add(atom: undefined, callback: (atom: AnyAtom) => void): () => void;
};
/** StoreHooks are an experimental API. */
type StoreHooks = {
    /** Listener to notify when the atom state is created. */
    readonly i?: StoreHookForAtoms;
    /** Listener to notify when the atom is read. */
    readonly r?: StoreHookForAtoms;
    /** Listener to notify when the atom value is changed. */
    readonly c?: StoreHookForAtoms;
    /** Listener to notify when the atom is mounted. */
    readonly m?: StoreHookForAtoms;
    /** Listener to notify when the atom is unmounted. */
    readonly u?: StoreHookForAtoms;
    /** Listener to notify when callbacks are being flushed. */
    readonly f?: StoreHook;
};
declare function initializeStoreHooks(storeHooks: StoreHooks): Required<StoreHooks>;
declare function getBuildingBlocks(store: Store): Readonly<BuildingBlocks>;
declare function buildStore(...buildArgs: Partial<BuildingBlocks>): Store;
export { buildStore as INTERNAL_buildStoreRev2, getBuildingBlocks as INTERNAL_getBuildingBlocksRev2, initializeStoreHooks as INTERNAL_initializeStoreHooksRev2, hasInitialValue as INTERNAL_hasInitialValue, isActuallyWritableAtom as INTERNAL_isActuallyWritableAtom, isAtomStateInitialized as INTERNAL_isAtomStateInitialized, returnAtomValue as INTERNAL_returnAtomValue, promiseStateMap as INTERNAL_promiseStateMap, isPendingPromise as INTERNAL_isPendingPromise, abortPromise as INTERNAL_abortPromise, registerAbortHandler as INTERNAL_registerAbortHandler, isPromiseLike as INTERNAL_isPromiseLike, addPendingPromiseToDependency as INTERNAL_addPendingPromiseToDependency, getMountedOrPendingDependents as INTERNAL_getMountedOrPendingDependents, };
