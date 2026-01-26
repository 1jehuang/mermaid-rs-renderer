import { Field } from '../channeldef.js';
import { Encoding } from '../encoding.js';
import { GenericSpec } from '../spec/index.js';
import { GenericUnitSpec } from '../spec/unit.js';
import { NonFacetUnitNormalizer, NormalizeLayerOrUnit, NormalizerParams } from './base.js';
interface EncodingX2Mixins {
    x2: Encoding<Field>['x2'];
}
interface EncodingY2Mixins {
    y2: Encoding<Field>['y2'];
}
type RangedLineSpec = GenericUnitSpec<Encoding<Field> & (EncodingX2Mixins | EncodingY2Mixins), 'line' | {
    mark: 'line';
}>;
export declare class RuleForRangedLineNormalizer implements NonFacetUnitNormalizer<RangedLineSpec> {
    name: string;
    hasMatchingType(spec: GenericSpec<any, any, any, any>): spec is RangedLineSpec;
    run(spec: RangedLineSpec, params: NormalizerParams, normalize: NormalizeLayerOrUnit): import("../spec/unit.js").NormalizedUnitSpec | import("../spec/layer.js").NormalizedLayerSpec;
}
export {};
//# sourceMappingURL=ruleforrangedline.d.ts.map