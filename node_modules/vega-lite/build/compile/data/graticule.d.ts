import { GraticuleTransform as VgGraticuleTransform } from 'vega';
import { GraticuleParams } from '../../data.js';
import { DataFlowNode } from './dataflow.js';
export declare class GraticuleNode extends DataFlowNode {
    private params;
    clone(): GraticuleNode;
    constructor(parent: DataFlowNode, params: true | GraticuleParams);
    dependentFields(): Set<string>;
    producedFields(): undefined;
    hash(): string;
    assemble(): VgGraticuleTransform;
}
//# sourceMappingURL=graticule.d.ts.map