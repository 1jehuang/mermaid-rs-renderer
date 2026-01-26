import { LookupTransform as VgLookupTransform } from 'vega';
import { LookupTransform } from '../../transform.js';
import { Model } from '../model.js';
import { DataFlowNode } from './dataflow.js';
export declare class LookupNode extends DataFlowNode {
    readonly transform: LookupTransform;
    readonly secondary: string;
    clone(): LookupNode;
    constructor(parent: DataFlowNode, transform: LookupTransform, secondary: string);
    static make(parent: DataFlowNode, model: Model, transform: LookupTransform, counter: number): LookupNode;
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    assemble(): VgLookupTransform;
}
//# sourceMappingURL=lookup.d.ts.map