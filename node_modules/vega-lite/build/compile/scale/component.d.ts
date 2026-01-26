import type { SignalRef } from 'vega';
import { ScaleChannel } from '../../channel.js';
import { Scale, ScaleType } from '../../scale.js';
import { ParameterExtent } from '../../selection.js';
import { VgNonUnionDomain, VgScale } from '../../vega.schema.js';
import { Explicit, Split } from '../split.js';
/**
 * All VgDomain property except domain.
 * (We exclude domain as we have a special "domains" array that allow us merge them all at once in assemble.)
 */
export type ScaleComponentProps = Omit<VgScale, 'domain' | 'reverse'> & {
    domains: VgNonUnionDomain[];
    selectionExtent?: ParameterExtent;
    reverse?: boolean | SignalRef;
};
export type Range = ScaleComponentProps['range'];
export declare class ScaleComponent extends Split<ScaleComponentProps> {
    merged: boolean;
    constructor(name: string, typeWithExplicit: Explicit<ScaleType>);
    /**
     * Whether the scale definitely includes or not include zero in the domain
     */
    domainHasZero(): 'definitely' | 'definitely-not' | 'maybe';
}
export type ScaleComponentIndex = Partial<Record<ScaleChannel, ScaleComponent>>;
export type ScaleIndex = Partial<Record<ScaleChannel, Scale<SignalRef>>>;
//# sourceMappingURL=component.d.ts.map