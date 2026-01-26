import { Encoding } from '../encoding.js';
import { GenericMarkDef } from '../mark.js';
import { NonFacetUnitNormalizer, Normalize, NormalizerParams } from '../normalize/base.js';
import { GenericSpec } from '../spec/index.js';
import { GenericLayerSpec, NormalizedLayerSpec } from '../spec/layer.js';
import { GenericUnitSpec, NormalizedUnitSpec } from '../spec/unit.js';
import { FieldName } from '../channeldef.js';
export type CompositeMarkUnitSpec<M extends string> = GenericUnitSpec<any, M | GenericMarkDef<M>>;
export declare class CompositeMarkNormalizer<M extends string> implements NonFacetUnitNormalizer<CompositeMarkUnitSpec<M>> {
    name: string;
    run: (spec: CompositeMarkUnitSpec<M>, params: NormalizerParams, normalize: Normalize<GenericUnitSpec<Encoding<FieldName>, M> | GenericLayerSpec<any>, NormalizedLayerSpec | NormalizedUnitSpec>) => NormalizedLayerSpec | NormalizedUnitSpec;
    constructor(name: string, run: (spec: CompositeMarkUnitSpec<M>, params: NormalizerParams, normalize: Normalize<GenericUnitSpec<Encoding<FieldName>, M> | GenericLayerSpec<any>, NormalizedLayerSpec | NormalizedUnitSpec>) => NormalizedLayerSpec | NormalizedUnitSpec);
    hasMatchingType(spec: GenericSpec<any, any, any, any>): spec is CompositeMarkUnitSpec<M>;
}
//# sourceMappingURL=base.d.ts.map