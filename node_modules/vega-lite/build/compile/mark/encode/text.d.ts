import { Config } from '../../../config.js';
import { Encoding } from '../../../encoding.js';
import { VgValueRef } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
export declare function text(model: UnitModel, channel?: 'text' | 'href' | 'url' | 'description'): Partial<Record<import("../../../vega.schema.js").VgEncodeChannel, VgValueRef | (VgValueRef & {
    test?: string;
})[]>>;
export declare function textRef(channelDef: Encoding<string>['text' | 'tooltip'], config: Config, expr?: 'datum' | 'datum.datum'): VgValueRef;
//# sourceMappingURL=text.d.ts.map