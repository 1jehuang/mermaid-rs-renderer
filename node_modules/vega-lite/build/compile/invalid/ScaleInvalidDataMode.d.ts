import { SignalRef } from 'vega';
import { ScaleChannel } from '../../channel.js';
import { Config } from '../../config.js';
import { MarkInvalidDataMode } from '../../invalid.js';
import { MarkDef } from '../../mark.js';
import { ScaleType } from '../../scale.js';
export type ScaleInvalidDataMode = Omit<MarkInvalidDataMode, 'break-paths-show-path-domains'> | 'always-valid';
export declare function getScaleInvalidDataMode<C extends ScaleChannel>({ markDef, config, scaleChannel, scaleType, isCountAggregate, }: {
    markDef: MarkDef;
    config: Config<SignalRef>;
    scaleChannel: C;
    scaleType: ScaleType;
    isCountAggregate: boolean;
}): ScaleInvalidDataMode;
export declare function shouldBreakPath(mode: ScaleInvalidDataMode): boolean;
//# sourceMappingURL=ScaleInvalidDataMode.d.ts.map