import { Config } from '../config.js';
import { Encoding } from '../encoding.js';
import { Mark, MarkDef } from '../mark.js';
import { GenericUnitSpec, NormalizedUnitSpec } from '../spec/index.js';
import { NonFacetUnitNormalizer, NormalizeLayerOrUnit, NormalizerParams } from './base.js';
type UnitSpecWithPathOverlay = GenericUnitSpec<Encoding<string>, Mark | MarkDef<'line' | 'area' | 'rule' | 'trail'>>;
export declare class PathOverlayNormalizer implements NonFacetUnitNormalizer<UnitSpecWithPathOverlay> {
    name: string;
    hasMatchingType(spec: GenericUnitSpec<any, Mark | MarkDef>, config: Config): spec is UnitSpecWithPathOverlay;
    run(spec: UnitSpecWithPathOverlay, normParams: NormalizerParams, normalize: NormalizeLayerOrUnit): NormalizedUnitSpec | import("../spec/layer.js").NormalizedLayerSpec;
}
export {};
//# sourceMappingURL=pathoverlay.d.ts.map