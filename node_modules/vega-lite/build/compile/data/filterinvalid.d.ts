import type { FilterTransform as VgFilterTransform } from 'vega';
import { TypedFieldDef } from '../../channeldef.js';
import { Dict } from '../../util.js';
import { DataSourcesForHandlingInvalidValues } from '../invalid/datasources.js';
import { UnitModel } from '../unit.js';
import { DataFlowNode } from './dataflow.js';
export declare class FilterInvalidNode extends DataFlowNode {
    readonly filter: Dict<TypedFieldDef<string>>;
    clone(): FilterInvalidNode;
    constructor(parent: DataFlowNode, filter: Dict<TypedFieldDef<string>>);
    static make(parent: DataFlowNode, model: UnitModel, dataSourcesForHandlingInvalidValues: DataSourcesForHandlingInvalidValues): FilterInvalidNode;
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    /**
     * Create the VgTransforms for each of the filtered fields.
     */
    assemble(): VgFilterTransform;
}
export declare function isValidFiniteNumberExpr(ref: string): string;
//# sourceMappingURL=filterinvalid.d.ts.map