import { types } from '@babel/core';
export interface PluginOptions {
    customAtomNames?: string[];
}
export declare function isAtom(t: typeof types, callee: babel.types.Expression | babel.types.V8IntrinsicIdentifier, customAtomNames?: PluginOptions['customAtomNames']): boolean;
