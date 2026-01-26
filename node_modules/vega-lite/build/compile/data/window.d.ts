import { WindowTransform as VgWindowTransform } from 'vega';
import { WindowTransform } from '../../transform.js';
import { VgJoinAggregateTransform } from '../../vega.schema.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for the window transform nodes
 */
export declare class WindowTransformNode extends DataFlowNode {
    private readonly transform;
    clone(): WindowTransformNode;
    constructor(parent: DataFlowNode, transform: WindowTransform);
    addDimensions(fields: string[]): void;
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    private getDefaultName;
    hash(): string;
    assemble(): VgWindowTransform | VgJoinAggregateTransform;
}
//# sourceMappingURL=window.d.ts.map