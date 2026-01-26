import { ScaleChannel } from '../../channel.js';
import { DatumDef, TypedFieldDef } from '../../channeldef.js';
import { MarkDef } from '../../mark.js';
import { Scale, ScaleType } from '../../scale.js';
export type RangeType = 'continuous' | 'discrete' | 'flexible' | undefined;
/**
 * Determine if there is a specified scale type and if it is appropriate,
 * or determine default type if type is unspecified or inappropriate.
 */
export declare function scaleType(specifiedScale: Scale, channel: ScaleChannel, fieldDef: TypedFieldDef<string> | DatumDef, mark: MarkDef, hasNestedOffsetScale?: boolean): ScaleType;
//# sourceMappingURL=type.d.ts.map