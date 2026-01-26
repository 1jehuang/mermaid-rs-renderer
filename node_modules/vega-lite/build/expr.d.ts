import { Dict } from './util.js';
import { MappedExclude } from './vega.schema.js';
export interface ExprRef {
    /**
     * Vega expression (which can refer to Vega-Lite parameters).
     */
    expr: string;
}
export declare function isExprRef(o: any): o is ExprRef;
export declare function replaceExprRef<T extends Dict<any>>(index: T, { level }?: {
    level: number;
}): MappedExclude<T, ExprRef>;
//# sourceMappingURL=expr.d.ts.map