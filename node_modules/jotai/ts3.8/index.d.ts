export * from 'jotai/vanilla';
export * from 'jotai/react';
declare type Awaited<T> = T extends Promise<infer V> ? V : T;