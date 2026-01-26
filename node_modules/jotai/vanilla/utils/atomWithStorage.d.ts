import type { WritableAtom } from 'jotai/vanilla';
import { RESET } from './constants';
type Unsubscribe = () => void;
type Subscribe<Value> = (key: string, callback: (value: Value) => void, initialValue: Value) => Unsubscribe | undefined;
type StringSubscribe = (key: string, callback: (value: string | null) => void) => Unsubscribe | undefined;
type SetStateActionWithReset<Value> = Value | typeof RESET | ((prev: Value) => Value | typeof RESET);
export interface AsyncStorage<Value> {
    getItem: (key: string, initialValue: Value) => PromiseLike<Value>;
    setItem: (key: string, newValue: Value) => PromiseLike<void>;
    removeItem: (key: string) => PromiseLike<void>;
    subscribe?: Subscribe<Value>;
}
export interface SyncStorage<Value> {
    getItem: (key: string, initialValue: Value) => Value;
    setItem: (key: string, newValue: Value) => void;
    removeItem: (key: string) => void;
    subscribe?: Subscribe<Value>;
}
export interface AsyncStringStorage {
    getItem: (key: string) => PromiseLike<string | null>;
    setItem: (key: string, newValue: string) => PromiseLike<void>;
    removeItem: (key: string) => PromiseLike<void>;
    subscribe?: StringSubscribe;
}
export interface SyncStringStorage {
    getItem: (key: string) => string | null;
    setItem: (key: string, newValue: string) => void;
    removeItem: (key: string) => void;
    subscribe?: StringSubscribe;
}
export declare function withStorageValidator<Value>(validator: (value: unknown) => value is Value): {
    (storage: AsyncStorage<unknown>): AsyncStorage<Value>;
    (storage: SyncStorage<unknown>): SyncStorage<Value>;
};
type JsonStorageOptions = {
    reviver?: (key: string, value: unknown) => unknown;
    replacer?: (key: string, value: unknown) => unknown;
};
export declare function createJSONStorage<Value>(): SyncStorage<Value>;
export declare function createJSONStorage<Value>(getStringStorage: () => AsyncStringStorage, options?: JsonStorageOptions): AsyncStorage<Value>;
export declare function createJSONStorage<Value>(getStringStorage: () => SyncStringStorage, options?: JsonStorageOptions): SyncStorage<Value>;
export declare function atomWithStorage<Value>(key: string, initialValue: Value, storage: AsyncStorage<Value>, options?: {
    getOnInit?: boolean;
}): WritableAtom<Value | Promise<Value>, [
    SetStateActionWithReset<Value | Promise<Value>>
], Promise<void>>;
export declare function atomWithStorage<Value>(key: string, initialValue: Value, storage?: SyncStorage<Value>, options?: {
    getOnInit?: boolean;
}): WritableAtom<Value, [SetStateActionWithReset<Value>], void>;
export {};
