import { ExtentTransform as VgExtentTransform } from 'vega';
import { ExtentTransform } from '../../transform.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for flatten transform nodes
 */
export declare class ExtentTransformNode extends DataFlowNode {
    private transform;
    clone(): ExtentTransformNode;
    constructor(parent: DataFlowNode, transform: ExtentTransform);
    dependentFields(): Set<string>;
    producedFields(): Set<any>;
    hash(): string;
    assemble(): VgExtentTransform;
}
//# sourceMappingURL=extent.d.ts.map