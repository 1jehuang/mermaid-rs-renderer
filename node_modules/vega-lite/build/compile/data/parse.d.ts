import { AncestorParse, DataComponent } from './index.js';
import { Data } from '../../data.js';
import { Model } from '../model.js';
import { DataFlowNode } from './dataflow.js';
import { SourceNode } from './source.js';
export declare function findSource(data: Data, sources: SourceNode[]): SourceNode;
/**
 * Parses a transform array into a chain of connected dataflow nodes.
 */
export declare function parseTransformArray(head: DataFlowNode, model: Model, ancestorParse: AncestorParse): DataFlowNode;
export declare function parseData(model: Model): DataComponent;
//# sourceMappingURL=parse.d.ts.map