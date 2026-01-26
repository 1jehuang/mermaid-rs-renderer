import { Legend as VgLegend } from 'vega';
import { NonPositionScaleChannel } from '../../channel.js';
import { LegendInternal } from '../../legend.js';
import { Split } from '../split.js';
export type LegendComponentProps = VgLegend & {
    labelExpr?: string;
    selections?: string[];
    disable?: boolean;
};
export declare const LEGEND_COMPONENT_PROPERTIES: ("labelExpr" | "disable" | keyof VgLegend | "selections")[];
export declare class LegendComponent extends Split<LegendComponentProps> {
}
export type LegendComponentIndex = Partial<Record<NonPositionScaleChannel, LegendComponent>>;
export type LegendInternalIndex = Partial<Record<NonPositionScaleChannel, LegendInternal>>;
//# sourceMappingURL=component.d.ts.map