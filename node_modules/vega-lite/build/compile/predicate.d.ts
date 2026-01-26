import { LogicalComposition } from '../logical.js';
import { Predicate } from '../predicate.js';
import { DataFlowNode } from './data/dataflow.js';
import { Model } from './model.js';
/**
 * Converts a predicate into an expression.
 */
export declare function expression(model: Model, filterOp: LogicalComposition<Predicate>, node?: DataFlowNode): string;
//# sourceMappingURL=predicate.d.ts.map