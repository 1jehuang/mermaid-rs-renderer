import type { FunctionComponent, ReactElement, ReactNode } from 'react';
import { createStore } from 'jotai/vanilla';
type Store = ReturnType<typeof createStore>;
type Options = {
    store?: Store;
};
export declare function useStore(options?: Options): Store;
export declare function Provider({ children, store, }: {
    children?: ReactNode;
    store?: Store;
}): ReactElement<{
    value: Store | undefined;
}, FunctionComponent<{
    value: Store;
}>>;
export {};
