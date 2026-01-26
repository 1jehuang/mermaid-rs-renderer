import babel from '@babel/core';
import type { PluginOptions } from './utils.mjs';
export default function jotaiPreset(_: typeof babel, options?: PluginOptions): {
    plugins: babel.PluginItem[];
};
