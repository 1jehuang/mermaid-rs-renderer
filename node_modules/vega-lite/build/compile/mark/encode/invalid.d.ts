import { NonPositionScaleChannel, PolarPositionScaleChannel, PositionScaleChannel } from '../../../channel.js';
import { VgValueRef } from '../../../vega.schema.js';
import { MidPointParams } from './valueref.js';
export declare function getConditionalValueRefForIncludingInvalidValue<C extends PositionScaleChannel | PolarPositionScaleChannel | NonPositionScaleChannel>({ scaleChannel, channelDef, scale, scaleName, markDef, config, }: {
    scaleChannel: C;
} & Pick<MidPointParams, 'scale' | 'scaleName' | 'channelDef' | 'markDef' | 'config'>): VgValueRef | undefined;
//# sourceMappingURL=invalid.d.ts.map