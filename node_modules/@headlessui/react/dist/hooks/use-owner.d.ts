import { getOwnerDocument, getRootNode } from '../utils/owner.js';
export declare function useOwnerDocument(...args: Parameters<typeof getOwnerDocument>): Document | null;
export declare function useRootDocument(...args: Parameters<typeof getRootNode>): Document | ShadowRoot | null;
