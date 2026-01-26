import { Config } from '../../config.js';
import { VgEncodeEntry } from '../../vega.schema.js';
import { UnitModel } from '../unit.js';
import { MarkCompiler } from './base.js';
export declare function shapeMixins(model: UnitModel, config: Config, fixedShape?: 'circle' | 'square'): VgEncodeEntry;
export declare const point: MarkCompiler;
export declare const circle: MarkCompiler;
export declare const square: MarkCompiler;
//# sourceMappingURL=point.d.ts.map