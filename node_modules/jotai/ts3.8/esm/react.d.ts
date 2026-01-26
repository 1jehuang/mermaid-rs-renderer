export { Provider, useStore } from './react/Provider';
export { useAtomValue } from './react/useAtomValue';
export { useSetAtom } from './react/useSetAtom';
export { useAtom } from './react/useAtom';
declare type Awaited<T> = T extends Promise<infer V> ? V : T;