import { Field } from '../channeldef.js';
import { Encoding } from '../encoding.js';
import { NormalizerParams } from '../normalize/index.js';
import { GenericUnitSpec, NormalizedLayerSpec } from '../spec/index.js';
import { EncodingFacetMapping } from '../spec/facet.js';
import { NormalizedUnitSpec } from '../spec/unit.js';
import { BoxPlot, BoxPlotConfigMixins, BoxPlotDef } from './boxplot.js';
import { ErrorBand, ErrorBandConfigMixins, ErrorBandDef } from './errorband.js';
import { ErrorBar, ErrorBarConfigMixins, ErrorBarDef, ErrorExtraEncoding } from './errorbar.js';
export type { BoxPlotConfig } from './boxplot.js';
export type { ErrorBandConfigMixins } from './errorband.js';
export type { ErrorBarConfigMixins } from './errorbar.js';
export type CompositeMarkNormalizerRun = (spec: GenericUnitSpec<any, any>, params: NormalizerParams) => NormalizedLayerSpec | NormalizedUnitSpec;
export declare function add(mark: string, run: CompositeMarkNormalizerRun, parts: readonly string[]): void;
export declare function remove(mark: string): void;
export type CompositeEncoding<F extends Field> = Encoding<F> & ErrorExtraEncoding<F>;
export type PartialIndex<T extends Encoding<any>> = {
    [t in keyof T]?: Partial<T[t]>;
};
export type SharedCompositeEncoding<F extends Field> = PartialIndex<Omit<CompositeEncoding<F>, 'detail' | 'order' | 'tooltip'>> & Pick<Encoding<F>, 'detail' | 'order' | 'tooltip'>;
export type FacetedCompositeEncoding<F extends Field> = Encoding<F> & ErrorExtraEncoding<F> & EncodingFacetMapping<F>;
export type CompositeMark = BoxPlot | ErrorBar | ErrorBand;
export declare function getAllCompositeMarks(): string[];
export type CompositeMarkDef = BoxPlotDef | ErrorBarDef | ErrorBandDef;
export type CompositeAggregate = BoxPlot | ErrorBar | ErrorBand;
export interface CompositeMarkConfigMixins extends BoxPlotConfigMixins, ErrorBarConfigMixins, ErrorBandConfigMixins {
}
//# sourceMappingURL=index.d.ts.map