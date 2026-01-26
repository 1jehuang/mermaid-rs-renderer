/**
 * Utility files for producing Vega ValueRef for marks
 */
import type { SignalRef } from 'vega';
import { PolarPositionChannel, PositionChannel } from '../../../channel.js';
import { Encoding } from '../../../encoding.js';
import { Mark, MarkDef } from '../../../mark.js';
import { VgValueRef } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
export interface Offset {
    offsetType?: 'visual' | 'encoding';
    offset?: number | VgValueRef;
}
export declare function positionOffset({ channel: baseChannel, markDef, encoding, model, bandPosition, }: {
    channel: PositionChannel | PolarPositionChannel;
    markDef: MarkDef<Mark, SignalRef>;
    encoding?: Encoding<string>;
    model?: UnitModel;
    bandPosition?: number;
}): Offset;
//# sourceMappingURL=offset.d.ts.map