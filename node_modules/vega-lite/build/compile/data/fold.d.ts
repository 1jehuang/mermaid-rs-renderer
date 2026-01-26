import type { FoldTransform as VgFoldTransform } from 'vega';
import { FoldTransform } from '../../transform.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for flatten transform nodes
 */
export declare class FoldTransformNode extends DataFlowNode {
    private transform;
    clone(): FoldTransformNode;
    constructor(parent: DataFlowNode, transform: FoldTransform);
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    assemble(): VgFoldTransform;
}
//# sourceMappingURL=fold.d.ts.map