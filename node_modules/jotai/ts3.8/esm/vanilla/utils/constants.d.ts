export declare const RESET: unique symbol;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;