import { Field, FieldName } from '../channeldef.js';
import { FacetedUnitSpec, GenericSpec, LayerSpec, UnitSpec } from '../spec/index.js';
import { GenericConcatSpec } from '../spec/concat.js';
import { GenericFacetSpec, NormalizedFacetSpec } from '../spec/facet.js';
import { NormalizedSpec } from '../spec/index.js';
import { NormalizedLayerSpec } from '../spec/layer.js';
import { SpecMapper } from '../spec/map.js';
import { RepeatSpec } from '../spec/repeat.js';
import { NormalizedUnitSpec } from '../spec/unit.js';
import { NormalizerParams } from './base.js';
export declare class CoreNormalizer extends SpecMapper<NormalizerParams, FacetedUnitSpec<Field>, LayerSpec<Field>> {
    private nonFacetUnitNormalizers;
    map(spec: GenericSpec<FacetedUnitSpec<Field>, LayerSpec<Field>, RepeatSpec, Field>, params: NormalizerParams): NormalizedUnitSpec | NormalizedFacetSpec | import("../spec/layer.js").GenericLayerSpec<NormalizedUnitSpec> | GenericConcatSpec<GenericSpec<NormalizedUnitSpec, import("../spec/layer.js").GenericLayerSpec<NormalizedUnitSpec>, never, string>> | import("../spec/concat.js").GenericVConcatSpec<GenericSpec<NormalizedUnitSpec, import("../spec/layer.js").GenericLayerSpec<NormalizedUnitSpec>, never, string>> | import("../spec/concat.js").GenericHConcatSpec<GenericSpec<NormalizedUnitSpec, import("../spec/layer.js").GenericLayerSpec<NormalizedUnitSpec>, never, string>>;
    mapUnit(spec: UnitSpec<Field>, params: NormalizerParams): NormalizedUnitSpec | NormalizedLayerSpec;
    protected mapRepeat(spec: RepeatSpec, params: NormalizerParams): GenericConcatSpec<NormalizedSpec> | NormalizedLayerSpec;
    private mapLayerRepeat;
    private mapNonLayerRepeat;
    protected mapFacet(spec: GenericFacetSpec<UnitSpec<Field>, LayerSpec<Field>, Field>, params: NormalizerParams): GenericFacetSpec<NormalizedUnitSpec, NormalizedLayerSpec, FieldName>;
    private mapUnitWithParentEncodingOrProjection;
    private mapFacetedUnit;
    private getFacetMappingAndLayout;
    mapLayer(spec: LayerSpec<Field>, { parentEncoding, parentProjection, ...otherParams }: NormalizerParams): NormalizedLayerSpec;
}
//# sourceMappingURL=core.d.ts.map