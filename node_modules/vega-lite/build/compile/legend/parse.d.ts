import { NonPositionScaleChannel } from '../../channel.js';
import { Model } from '../model.js';
import { UnitModel } from '../unit.js';
import { LegendComponent } from './component.js';
export declare function parseLegend(model: Model): Partial<Record<"color" | "fill" | "stroke" | "strokeWidth" | "size" | "shape" | "fillOpacity" | "strokeOpacity" | "opacity" | "strokeDash" | "angle" | "time", LegendComponent>>;
export declare function parseLegendForChannel(model: UnitModel, channel: NonPositionScaleChannel): LegendComponent;
export declare function mergeLegendComponent(mergedLegend: LegendComponent, childLegend: LegendComponent): LegendComponent;
//# sourceMappingURL=parse.d.ts.map