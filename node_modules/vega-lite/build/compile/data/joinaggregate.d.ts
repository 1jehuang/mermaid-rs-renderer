import { JoinAggregateTransform } from '../../transform.js';
import { VgJoinAggregateTransform } from '../../vega.schema.js';
import { DataFlowNode } from './dataflow.js';
/**
 * A class for the join aggregate transform nodes.
 */
export declare class JoinAggregateTransformNode extends DataFlowNode {
    private readonly transform;
    clone(): JoinAggregateTransformNode;
    constructor(parent: DataFlowNode, transform: JoinAggregateTransform);
    addDimensions(fields: string[]): void;
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    private getDefaultName;
    hash(): string;
    assemble(): VgJoinAggregateTransform;
}
//# sourceMappingURL=joinaggregate.d.ts.map