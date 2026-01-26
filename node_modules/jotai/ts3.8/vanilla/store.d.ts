import type { INTERNAL_Store } from 'jotai/vanilla/internals';
export type Store = INTERNAL_Store;
export declare function INTERNAL_overrideCreateStore(fn: (prev: typeof createStore | undefined) => typeof createStore): void;
export declare function createStore(): Store;
export declare function getDefaultStore(): Store;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;