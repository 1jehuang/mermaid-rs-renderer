import { Value } from '../../../channeldef.js';
import { VgEncodeEntry } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
/**
 * Create Vega's "defined" encoding to break paths in a path mark for invalid values.
 */
export declare function defined(model: UnitModel): VgEncodeEntry;
export declare function valueIfDefined(prop: string, value: Value): VgEncodeEntry;
//# sourceMappingURL=defined.d.ts.map