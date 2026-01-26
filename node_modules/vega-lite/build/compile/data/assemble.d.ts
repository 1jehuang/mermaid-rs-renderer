import { InlineDataset } from '../../data.js';
import { Dict } from '../../util.js';
import { VgData } from '../../vega.schema.js';
import { FacetNode } from './facet.js';
import { DataComponent } from './index.js';
/**
 * Assemble data sources that are derived from faceted data.
 */
export declare function assembleFacetData(root: FacetNode): VgData[];
/**
 * Create Vega data array from a given compiled model and append all of them to the given array
 *
 * @param  model
 * @param  data array
 * @return modified data array
 */
export declare function assembleRootData(dataComponent: DataComponent, datasets: Dict<InlineDataset>): VgData[];
//# sourceMappingURL=assemble.d.ts.map