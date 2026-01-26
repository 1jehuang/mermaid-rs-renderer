import { Axis as VgAxis, Legend as VgLegend, NewSignal, Projection as VgProjection, Signal, SignalRef, Title as VgTitle } from 'vega';
import { Channel, ExtendedChannel, ScaleChannel, SingleDefChannel } from '../channel.js';
import { FieldDef, FieldRefOption } from '../channeldef.js';
import { Config } from '../config.js';
import { Data, DataSourceType } from '../data.js';
import { ExprRef } from '../expr.js';
import { Resolve } from '../resolve.js';
import { ScaleType } from '../scale.js';
import { GenericCompositionLayoutWithColumns, LayoutSizeMixins, SpecType, ViewBackground } from '../spec/base.js';
import { NormalizedSpec } from '../spec/index.js';
import { TitleParams } from '../title.js';
import { Transform } from '../transform.js';
import { Dict } from '../util.js';
import { VgData, VgEncodeEntry, VgLayout, VgMarkGroup } from '../vega.schema.js';
import { AxisComponentIndex } from './axis/component.js';
import { ConcatModel } from './concat.js';
import { DataComponent } from './data/index.js';
import { FacetModel } from './facet.js';
import { LayoutHeaderComponent } from './header/component.js';
import { LayerModel } from './layer.js';
import { LayoutSizeComponent, LayoutSizeType } from './layoutsize/component.js';
import { LegendComponentIndex } from './legend/component.js';
import { ProjectionComponent } from './projection/component.js';
import { ScaleComponent, ScaleComponentIndex } from './scale/component.js';
import { SelectionComponent } from './selection/index.js';
import { UnitModel } from './unit.js';
/**
 * Composable Components that are intermediate results of the parsing phase of the
 * compilations. The components represents parts of the specification in a form that
 * can be easily merged (during parsing for composite specs).
 * In addition, these components are easily transformed into Vega specifications
 * during the "assemble" phase, which is the last phase of the compilation step.
 */
export interface Component {
    data: DataComponent;
    layoutSize: LayoutSizeComponent;
    layoutHeaders: {
        row?: LayoutHeaderComponent;
        column?: LayoutHeaderComponent;
        facet?: LayoutHeaderComponent;
    };
    mark: VgMarkGroup[];
    scales: ScaleComponentIndex;
    projection: ProjectionComponent;
    selection: Dict<SelectionComponent>;
    /** Dictionary mapping channel to VgAxis definition */
    axes: AxisComponentIndex;
    /** Dictionary mapping channel to VgLegend definition */
    legends: LegendComponentIndex;
    resolve: Resolve;
}
export interface NameMapInterface {
    rename(oldname: string, newName: string): void;
    has(name: string): boolean;
    get(name: string): string;
}
export declare class NameMap implements NameMapInterface {
    private nameMap;
    constructor();
    rename(oldName: string, newName: string): void;
    has(name: string): boolean;
    get(name: string): string;
}
export declare function isUnitModel(model: Model): model is UnitModel;
export declare function isFacetModel(model: Model): model is FacetModel;
export declare function isConcatModel(model: Model): model is ConcatModel;
export declare function isLayerModel(model: Model): model is LayerModel;
export declare abstract class Model {
    readonly type: SpecType;
    readonly parent: Model;
    readonly config: Config<SignalRef>;
    readonly name: string;
    size: LayoutSizeMixins;
    readonly title: TitleParams<SignalRef>;
    readonly description: string;
    readonly data: Data | null;
    readonly transforms: Transform[];
    readonly layout: GenericCompositionLayoutWithColumns;
    /** Name map for scales, which can be renamed by a model's parent. */
    protected scaleNameMap: NameMapInterface;
    /** Name map for projections, which can be renamed by a model's parent. */
    protected projectionNameMap: NameMapInterface;
    /** Name map for signals, which can be renamed by a model's parent. */
    protected signalNameMap: NameMapInterface;
    readonly component: Component;
    readonly view?: ViewBackground<SignalRef>;
    abstract readonly children: Model[];
    constructor(spec: NormalizedSpec, type: SpecType, parent: Model, parentGivenName: string, config: Config<SignalRef>, resolve: Resolve, view?: ViewBackground<ExprRef | SignalRef>);
    get width(): SignalRef;
    get height(): SignalRef;
    parse(): void;
    abstract parseData(): void;
    abstract parseSelections(): void;
    parseScale(): void;
    parseProjection(): void;
    abstract parseLayoutSize(): void;
    /**
     * Rename top-level spec's size to be just width / height, ignoring model name.
     * This essentially merges the top-level spec's width/height signals with the width/height signals
     * to help us reduce redundant signals declaration.
     */
    private renameTopLevelLayoutSizeSignal;
    abstract parseMarkGroup(): void;
    abstract parseAxesAndHeaders(): void;
    parseLegends(): void;
    abstract assembleSelectionTopLevelSignals(signals: NewSignal[]): NewSignal[];
    abstract assembleSignals(): NewSignal[];
    abstract assembleSelectionData(data: readonly VgData[]): readonly VgData[];
    abstract assembleGroupStyle(): string | string[];
    private assembleEncodeFromView;
    assembleGroupEncodeEntry(isTopLevel: boolean): VgEncodeEntry;
    assembleLayout(): VgLayout;
    protected assembleDefaultLayout(): VgLayout;
    abstract assembleLayoutSignals(): NewSignal[];
    assembleHeaderMarks(): VgMarkGroup[];
    abstract assembleMarks(): VgMarkGroup[];
    assembleAxes(): VgAxis[];
    assembleLegends(): VgLegend[];
    assembleProjections(): VgProjection[];
    assembleTitle(): VgTitle;
    /**
     * Assemble the mark group for this model. We accept optional `signals` so that we can include concat top-level signals with the top-level model's local signals.
     */
    assembleGroup(signals?: Signal[]): any;
    getName(text: string): string;
    getDataName(type: DataSourceType): string;
    /**
     * Request a data source name for the given data source type and mark that data source as required.
     * This method should be called in parse, so that all used data source can be correctly instantiated in assembleData().
     * You can lookup the correct dataset name in assemble with `lookupDataSource`.
     */
    requestDataName(name: DataSourceType): string;
    getSizeSignalRef(layoutSizeType: LayoutSizeType): SignalRef;
    /**
     * Lookup the name of the datasource for an output node. You probably want to call this in assemble.
     */
    lookupDataSource(name: string): string;
    getSignalName(oldSignalName: string): string;
    renameSignal(oldName: string, newName: string): void;
    renameScale(oldName: string, newName: string): void;
    renameProjection(oldName: string, newName: string): void;
    /**
     * @return scale name for a given channel after the scale has been parsed and named.
     */
    scaleName(originalScaleName: ScaleChannel | string, parse?: boolean): string;
    /**
     * @return projection name after the projection has been parsed and named.
     */
    projectionName(parse?: boolean): string;
    /**
     * Traverse a model's hierarchy to get the scale component for a particular channel.
     */
    getScaleComponent(channel: ScaleChannel): ScaleComponent;
    getScaleType(channel: ScaleChannel): ScaleType;
    /**
     * Traverse a model's hierarchy to get a particular selection component.
     */
    getSelectionComponent(variableName: string, origName: string): SelectionComponent;
    /**
     * Returns true if the model has a signalRef for an axis orient.
     */
    hasAxisOrientSignalRef(): boolean;
}
/** Abstract class for UnitModel and FacetModel. Both of which can contain fieldDefs as a part of its own specification. */
export declare abstract class ModelWithField extends Model {
    abstract fieldDef(channel: SingleDefChannel): FieldDef<any>;
    /** Get "field" reference for Vega */
    vgField(channel: SingleDefChannel, opt?: FieldRefOption): string;
    protected abstract getMapping(): Partial<Record<ExtendedChannel, any>>;
    reduceFieldDef<T, U>(f: (acc: U, fd: FieldDef<string>, c: Channel) => U, init: T): T;
    forEachFieldDef(f: (fd: FieldDef<string>, c: ExtendedChannel) => void, t?: any): void;
    abstract channelHasField(channel: Channel): boolean;
}
//# sourceMappingURL=model.d.ts.map