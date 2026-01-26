import { NewSignal, SignalRef } from 'vega';
import { ExtendedChannel } from '../channel.js';
import { FieldRefOption, TypedFieldDef } from '../channeldef.js';
import { Config } from '../config.js';
import { EncodingSortField } from '../sort.js';
import { NormalizedFacetSpec } from '../spec/index.js';
import { EncodingFacetMapping, FacetFieldDef } from '../spec/facet.js';
import { VgData, VgLayout, VgMarkGroup } from '../vega.schema.js';
import { Model, ModelWithField } from './model.js';
export declare function facetSortFieldName(fieldDef: FacetFieldDef<string>, sort: EncodingSortField<string>, opt?: FieldRefOption): string;
export declare class FacetModel extends ModelWithField {
    readonly facet: EncodingFacetMapping<string, SignalRef>;
    readonly child: Model;
    readonly children: Model[];
    constructor(spec: NormalizedFacetSpec, parent: Model, parentGivenName: string, config: Config<SignalRef>);
    private initFacet;
    private initFacetFieldDef;
    channelHasField(channel: ExtendedChannel): boolean;
    fieldDef(channel: ExtendedChannel): TypedFieldDef<string>;
    parseData(): void;
    parseLayoutSize(): void;
    parseSelections(): void;
    parseMarkGroup(): void;
    parseAxesAndHeaders(): void;
    assembleSelectionTopLevelSignals(signals: NewSignal[]): NewSignal[];
    assembleSignals(): NewSignal[];
    assembleSelectionData(data: readonly VgData[]): readonly VgData[];
    private getHeaderLayoutMixins;
    protected assembleDefaultLayout(): VgLayout;
    assembleLayoutSignals(): NewSignal[];
    private columnDistinctSignal;
    assembleGroupStyle(): string | string[];
    assembleGroup(signals: NewSignal[]): any;
    /**
     * Aggregate cardinality for calculating size
     */
    private getCardinalityAggregateForChild;
    private assembleFacet;
    private facetSortFields;
    private facetSortOrder;
    private assembleLabelTitle;
    assembleMarks(): VgMarkGroup[];
    protected getMapping(): EncodingFacetMapping<string, SignalRef>;
}
//# sourceMappingURL=facet.d.ts.map