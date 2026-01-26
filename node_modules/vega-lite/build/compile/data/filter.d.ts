import type { FilterTransform as VgFilterTransform } from 'vega';
import { LogicalComposition } from '../../logical.js';
import { Predicate } from '../../predicate.js';
import { Model } from '../model.js';
import { DataFlowNode } from './dataflow.js';
export declare class FilterNode extends DataFlowNode {
    private readonly model;
    private readonly filter;
    private expr;
    private _dependentFields;
    clone(): FilterNode;
    constructor(parent: DataFlowNode, model: Model, filter: LogicalComposition<Predicate>);
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    assemble(): VgFilterTransform;
    hash(): string;
}
//# sourceMappingURL=filter.d.ts.map