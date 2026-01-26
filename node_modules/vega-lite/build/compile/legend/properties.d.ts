import { LabelOverlap, LegendOrient, LegendType, Orientation, SignalRef, SymbolShape } from 'vega';
import { DatumDef, MarkPropFieldOrDatumDef, TypedFieldDef } from '../../channeldef.js';
import { Config } from '../../config.js';
import { Encoding } from '../../encoding.js';
import { Legend, LegendConfig, LegendInternal } from '../../legend.js';
import { Mark, MarkDef } from '../../mark.js';
import { ScaleType } from '../../scale.js';
import { TimeUnit } from '../../timeunit.js';
import { Model } from '../model.js';
import { UnitModel } from '../unit.js';
import { NonPositionScaleChannel } from './../../channel.js';
import { LegendComponentProps } from './component.js';
export interface LegendRuleParams {
    legend: LegendInternal;
    channel: NonPositionScaleChannel;
    model: UnitModel;
    markDef: MarkDef<Mark, SignalRef>;
    encoding: Encoding<string>;
    fieldOrDatumDef: MarkPropFieldOrDatumDef<string>;
    legendConfig: LegendConfig<SignalRef>;
    config: Config<SignalRef>;
    scaleType: ScaleType;
    orient: LegendOrient;
    legendType: LegendType;
    direction: Orientation;
}
export declare const legendRules: {
    [k in keyof LegendComponentProps]?: (params: LegendRuleParams) => LegendComponentProps[k];
};
export declare function values(legend: LegendInternal, fieldOrDatumDef: TypedFieldDef<string> | DatumDef): SignalRef | (string | number | boolean | import("../../datetime.js").DateTime | {
    signal: string;
})[];
export declare function defaultSymbolType(mark: Mark, channel: NonPositionScaleChannel, shapeChannelDef: Encoding<string>['shape'], markShape: SymbolShape | SignalRef): SymbolShape | SignalRef;
export declare function clipHeight(legendType: LegendType): number;
export declare function getLegendType(params: {
    legend: LegendInternal;
    channel: NonPositionScaleChannel;
    timeUnit?: TimeUnit;
    scaleType: ScaleType;
}): LegendType;
export declare function defaultType({ channel, timeUnit, scaleType, }: {
    channel: NonPositionScaleChannel;
    timeUnit?: TimeUnit;
    scaleType: ScaleType;
}): LegendType;
export declare function getDirection({ legendConfig, legendType, orient, legend, }: {
    orient: LegendOrient;
    legendConfig: LegendConfig<SignalRef>;
    legendType: LegendType;
    legend: Legend<SignalRef>;
}): Orientation;
export declare function defaultDirection(orient: LegendOrient, legendType: LegendType): 'horizontal' | undefined;
export declare function defaultGradientLength({ legendConfig, model, direction, orient, scaleType, }: {
    scaleType: ScaleType;
    direction: Orientation;
    orient: LegendOrient;
    model: Model;
    legendConfig: LegendConfig<SignalRef>;
}): number | {
    signal: string;
};
export declare function defaultLabelOverlap(scaleType: ScaleType): LabelOverlap;
//# sourceMappingURL=properties.d.ts.map