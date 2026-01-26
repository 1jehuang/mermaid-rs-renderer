import babel from '@babel/core';
import type { PluginObj } from '@babel/core';
import type { PluginOptions } from './utils';
export default function debugLabelPlugin({ types: t }: typeof babel, options?: PluginOptions): PluginObj;
declare type Awaited<T> = T extends Promise<infer V> ? V : T;