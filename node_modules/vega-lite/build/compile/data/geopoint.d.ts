import { GeoPointTransform as VgGeoPointTransform } from 'vega';
import { VgExprRef } from '../../vega.schema.js';
import { UnitModel } from '../unit.js';
import { DataFlowNode } from './dataflow.js';
export declare class GeoPointNode extends DataFlowNode {
    private projection;
    private fields;
    private as;
    clone(): GeoPointNode;
    constructor(parent: DataFlowNode, projection: string, fields: [string | VgExprRef, string | VgExprRef], as: [string, string]);
    static parseAll(parent: DataFlowNode, model: UnitModel): DataFlowNode;
    dependentFields(): Set<string>;
    producedFields(): Set<string>;
    hash(): string;
    assemble(): VgGeoPointTransform;
}
//# sourceMappingURL=geopoint.d.ts.map