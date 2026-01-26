import { SelectionComponent } from './index.js';
import { SelectionParameter, ParameterExtent } from '../../selection.js';
import { Dict } from '../../util.js';
import { DataFlowNode, OutputNode } from '../data/dataflow.js';
import { Model } from '../model.js';
import { UnitModel } from '../unit.js';
import { ParameterPredicate } from '../../predicate.js';
export declare function parseUnitSelection(model: UnitModel, selDefs: SelectionParameter[]): Dict<SelectionComponent<any>>;
export declare function parseSelectionPredicate(model: Model, pred: ParameterPredicate, dfnode?: DataFlowNode, datum?: string): string;
export declare function parseSelectionExtent(model: Model, name: string, extent: ParameterExtent): string;
export declare function materializeSelections(model: UnitModel, main: OutputNode): void;
//# sourceMappingURL=parse.d.ts.map