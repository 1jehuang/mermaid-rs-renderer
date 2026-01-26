import { Config } from '../../../config.js';
import { Encoding } from '../../../encoding.js';
import { StackProperties } from '../../../stack.js';
import { Dict } from '../../../util.js';
import { VgValueRef } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
export declare function tooltip(model: UnitModel, opt?: {
    reactiveGeom?: boolean;
}): Partial<Record<import("../../../vega.schema.js").VgEncodeChannel, VgValueRef | (VgValueRef & {
    test?: string;
})[]>> | {
    tooltip: {
        signal: string;
    };
};
export declare function tooltipData(encoding: Encoding<string>, stack: StackProperties, config: Config, { reactiveGeom }?: {
    reactiveGeom?: boolean;
}): Dict<string>;
export declare function tooltipRefForEncoding(encoding: Encoding<string>, stack: StackProperties, config: Config, { reactiveGeom }?: {
    reactiveGeom?: boolean;
}): {
    signal: string;
};
//# sourceMappingURL=tooltip.d.ts.map