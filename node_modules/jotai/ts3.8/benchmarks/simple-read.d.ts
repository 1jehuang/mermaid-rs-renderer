#!/usr/bin/env npx tsx
export {};
declare type Awaited<T> = T extends Promise<infer V> ? V : T;