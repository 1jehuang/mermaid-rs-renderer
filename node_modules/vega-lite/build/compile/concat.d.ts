import { NewSignal, SignalRef } from 'vega';
import { Config } from '../config.js';
import { NormalizedConcatSpec } from '../spec/index.js';
import { VgData, VgLayout } from '../vega.schema.js';
import { Model } from './model.js';
export declare class ConcatModel extends Model {
    readonly children: Model[];
    constructor(spec: NormalizedConcatSpec, parent: Model, parentGivenName: string, config: Config<SignalRef>);
    parseData(): void;
    parseSelections(): void;
    parseMarkGroup(): void;
    parseAxesAndHeaders(): void;
    private getChildren;
    parseLayoutSize(): void;
    parseAxisGroup(): void;
    assembleSelectionTopLevelSignals(signals: NewSignal[]): NewSignal[];
    assembleSignals(): NewSignal[];
    assembleLayoutSignals(): NewSignal[];
    assembleSelectionData(data: readonly VgData[]): readonly VgData[];
    assembleMarks(): any[];
    assembleGroupStyle(): string | string[];
    protected assembleDefaultLayout(): VgLayout;
}
//# sourceMappingURL=concat.d.ts.map