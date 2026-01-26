import { LoessTransform as VgLoessTransform } from 'vega';
import { LoessTransform } from '../../transform.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for loess transform nodes
 */
export declare class LoessTransformNode extends DataFlowNode {
    private transform;
    clone(): LoessTransformNode;
    constructor(parent: DataFlowNode, transform: LoessTransform);
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    assemble(): VgLoessTransform;
}
//# sourceMappingURL=loess.d.ts.map