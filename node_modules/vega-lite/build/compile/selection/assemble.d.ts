import { Signal, SignalRef } from 'vega';
import { SelectionInit, SelectionInitInterval, ParameterExtent } from '../../selection.js';
import { VgData, VgDomain } from '../../vega.schema.js';
import { FacetModel } from '../facet.js';
import { LayerModel } from '../layer.js';
import { Model } from '../model.js';
import { ScaleComponent } from '../scale/component.js';
import { UnitModel } from '../unit.js';
import { SelectionProjection } from './project.js';
export declare function assembleProjection(proj: SelectionProjection): {
    type: import("./project.js").TupleStoreType;
    field: string;
    channel?: import("../../channel.js").SingleDefUnitChannel;
    geoChannel?: import("../../channel.js").GeoPositionChannel;
};
export declare function assembleInit(init: readonly (SelectionInit | readonly SelectionInit[] | SelectionInitInterval)[] | SelectionInit, isExpr?: boolean, wrap?: (str: string | number) => string | number): any;
export declare function assembleUnitSelectionSignals(model: UnitModel, signals: Signal[]): Signal[];
export declare function assembleFacetSignals(model: FacetModel, signals: Signal[]): Signal[];
export declare function assembleTopLevelSignals(model: UnitModel, signals: Signal[]): Signal[];
export declare function assembleUnitSelectionData(model: UnitModel, data: readonly VgData[]): VgData[];
export declare function assembleUnitSelectionMarks(model: UnitModel, marks: any[]): any[];
export declare function assembleLayerSelectionMarks(model: LayerModel, marks: any[]): any[];
export declare function assembleSelectionScaleDomain(model: Model, extent: ParameterExtent, scaleCmpt: ScaleComponent, domain: VgDomain): SignalRef;
//# sourceMappingURL=assemble.d.ts.map