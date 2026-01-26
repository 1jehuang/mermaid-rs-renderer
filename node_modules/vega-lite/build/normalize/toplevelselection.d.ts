import { Field } from '../channeldef.js';
import { NormalizedLayerSpec, NormalizedSpec, NormalizedUnitSpec, TopLevel, UnitSpec } from '../spec/index.js';
import { SpecMapper } from '../spec/map.js';
import { NormalizerParams } from './base.js';
export declare class TopLevelSelectionsNormalizer extends SpecMapper<NormalizerParams, NormalizedUnitSpec> {
    map(spec: TopLevel<NormalizedSpec>, normParams: NormalizerParams): TopLevel<NormalizedSpec>;
    mapUnit(spec: UnitSpec<Field>, normParams: NormalizerParams): NormalizedUnitSpec | NormalizedLayerSpec;
}
//# sourceMappingURL=toplevelselection.d.ts.map