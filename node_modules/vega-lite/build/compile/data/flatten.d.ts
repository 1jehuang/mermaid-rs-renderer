import type { FlattenTransform as VgFlattenTransform } from 'vega';
import { FlattenTransform } from '../../transform.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for flatten transform nodes
 */
export declare class FlattenTransformNode extends DataFlowNode {
    private transform;
    clone(): FlattenTransformNode;
    constructor(parent: DataFlowNode, transform: FlattenTransform);
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    assemble(): VgFlattenTransform;
}
//# sourceMappingURL=flatten.d.ts.map