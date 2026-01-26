export * from 'jotai/vanilla/utils';
export * from 'jotai/react/utils';
declare type Awaited<T> = T extends Promise<infer V> ? V : T;