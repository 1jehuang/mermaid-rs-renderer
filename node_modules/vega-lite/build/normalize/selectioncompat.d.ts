import { Field } from '../channeldef.js';
import { FacetedUnitSpec, GenericSpec, LayerSpec, RepeatSpec, UnitSpec } from '../spec/index.js';
import { SpecMapper } from '../spec/map.js';
import { NormalizerParams } from './base.js';
export declare class SelectionCompatibilityNormalizer extends SpecMapper<NormalizerParams, FacetedUnitSpec<Field>, LayerSpec<Field>, UnitSpec<Field>> {
    map(spec: GenericSpec<FacetedUnitSpec<Field>, LayerSpec<Field>, RepeatSpec, Field>, normParams: NormalizerParams): GenericSpec<UnitSpec<Field>, import("../spec/layer.js").GenericLayerSpec<UnitSpec<Field>>, never, string>;
    mapLayerOrUnit(spec: FacetedUnitSpec<Field> | LayerSpec<Field>, normParams: NormalizerParams): UnitSpec<Field> | import("../spec/layer.js").GenericLayerSpec<UnitSpec<Field>>;
    mapUnit(spec: UnitSpec<Field>, normParams: NormalizerParams): any;
}
//# sourceMappingURL=selectioncompat.d.ts.map