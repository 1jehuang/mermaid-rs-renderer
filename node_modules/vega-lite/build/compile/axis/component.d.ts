import { Axis as VgAxis, SignalRef, Text } from 'vega';
import { AxisInternal, AxisPart, AxisPropsWithCondition, ConditionalAxisProp } from '../../axis.js';
import { FieldDefBase } from '../../channeldef.js';
import { Split } from '../split.js';
export type AxisComponentProps = Omit<VgAxis, 'title' | ConditionalAxisProp> & Omit<AxisPropsWithCondition<SignalRef>, 'title'> & {
    title: Text | FieldDefBase<string>[] | SignalRef;
    labelExpr: string;
    disable: boolean;
};
export declare const AXIS_COMPONENT_PROPERTIES: ("values" | "scale" | "domain" | "orient" | "aria" | "description" | "ticks" | "offset" | "zindex" | "labelAlign" | "labelBaseline" | "labelColor" | "labelFont" | "labelFontSize" | "labelFontStyle" | "labelFontWeight" | "labelOpacity" | "labelOffset" | "labelPadding" | "gridColor" | "gridDash" | "gridDashOffset" | "gridOpacity" | "gridWidth" | "tickColor" | "tickDash" | "tickDashOffset" | "tickOpacity" | "tickSize" | "tickWidth" | "title" | "gridScale" | "format" | "formatType" | "position" | "tickCount" | "tickMinStep" | "encode" | "translate" | "minExtent" | "maxExtent" | "bandPosition" | "titlePadding" | "titleAlign" | "titleAnchor" | "titleAngle" | "titleX" | "titleY" | "titleBaseline" | "titleColor" | "titleFont" | "titleFontSize" | "titleFontStyle" | "titleFontWeight" | "titleLimit" | "titleLineHeight" | "titleOpacity" | "domainCap" | "domainDash" | "domainDashOffset" | "domainColor" | "domainOpacity" | "domainWidth" | "tickBand" | "tickCap" | "tickExtra" | "tickOffset" | "tickRound" | "grid" | "gridCap" | "labels" | "labelBound" | "labelFlush" | "labelFlushOffset" | "labelLineHeight" | "labelOverlap" | "labelSeparation" | "labelAngle" | "labelLimit" | "labelExpr" | "disable")[];
export declare class AxisComponent extends Split<AxisComponentProps> {
    readonly explicit: Partial<AxisComponentProps>;
    readonly implicit: Partial<AxisComponentProps>;
    mainExtracted: boolean;
    constructor(explicit?: Partial<AxisComponentProps>, implicit?: Partial<AxisComponentProps>, mainExtracted?: boolean);
    clone(): AxisComponent;
    hasAxisPart(part: AxisPart): boolean;
    hasOrientSignalRef(): boolean;
}
export interface AxisComponentIndex {
    x?: AxisComponent[];
    y?: AxisComponent[];
}
export interface AxisInternalIndex {
    x?: AxisInternal;
    y?: AxisInternal;
}
//# sourceMappingURL=component.d.ts.map