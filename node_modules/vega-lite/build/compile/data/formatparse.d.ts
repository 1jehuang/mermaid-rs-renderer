import { FormulaTransform as VgFormulaTransform } from 'vega';
import { AncestorParse } from './index.js';
import { Parse } from '../../data.js';
import { FilterTransform } from '../../transform.js';
import { Dict } from '../../util.js';
import { Model } from '../model.js';
import { DataFlowNode } from './dataflow.js';
export declare function getImplicitFromFilterTransform(transform: FilterTransform): Dict<string>;
/**
 * Creates a parse node for implicit parsing from a model and updates ancestorParse.
 */
export declare function getImplicitFromEncoding(model: Model): Dict<string>;
/**
 * Creates a parse node for implicit parsing from a model and updates ancestorParse.
 */
export declare function getImplicitFromSelection(model: Model): Dict<string>;
export declare class ParseNode extends DataFlowNode {
    private _parse;
    clone(): ParseNode;
    constructor(parent: DataFlowNode, parse: Parse);
    hash(): string;
    /**
     * Creates a parse node from a data.format.parse and updates ancestorParse.
     */
    static makeExplicit(parent: DataFlowNode, model: Model, ancestorParse: AncestorParse): ParseNode;
    /**
     * Creates a parse node from "explicit" parse and "implicit" parse and updates ancestorParse.
     */
    static makeWithAncestors(parent: DataFlowNode, explicit: Parse, implicit: Parse, ancestorParse: AncestorParse): ParseNode;
    get parse(): Parse;
    merge(other: ParseNode): void;
    /**
     * Assemble an object for Vega's format.parse property.
     */
    assembleFormatParse(): Dict<string>;
    producedFields(): Set<string>;
    dependentFields(): Set<string>;
    assembleTransforms(onlyNested?: boolean): VgFormulaTransform[];
}
//# sourceMappingURL=formatparse.d.ts.map