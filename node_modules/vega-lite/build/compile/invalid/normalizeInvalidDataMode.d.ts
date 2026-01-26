import { MarkInvalidDataMode } from '../../invalid.js';
type NormalizedMarkInvalidDataMode = Exclude<MarkInvalidDataMode, 'break-paths-show-path-domains'>;
export declare function normalizeInvalidDataMode(mode: MarkInvalidDataMode | null | undefined, { isPath }: {
    isPath: boolean;
}): NormalizedMarkInvalidDataMode;
export {};
//# sourceMappingURL=normalizeInvalidDataMode.d.ts.map