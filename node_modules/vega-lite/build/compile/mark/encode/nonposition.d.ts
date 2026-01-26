import type { SignalRef } from 'vega';
import { NonPositionScaleChannel } from '../../../channel.js';
import { Value } from '../../../channeldef.js';
import { VgEncodeChannel, VgEncodeEntry, VgValueRef } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
/**
 * Return encode for non-positional channels with scales. (Text doesn't have scale.)
 */
export declare function nonPosition(channel: NonPositionScaleChannel, model: UnitModel, opt?: {
    defaultValue?: Value | SignalRef;
    vgChannel?: VgEncodeChannel;
    defaultRef?: VgValueRef;
}): VgEncodeEntry;
//# sourceMappingURL=nonposition.d.ts.map