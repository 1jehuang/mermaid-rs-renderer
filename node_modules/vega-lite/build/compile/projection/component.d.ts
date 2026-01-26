import { Projection as VgProjection, SignalRef } from 'vega';
import { Projection } from '../../projection.js';
import { Split } from '../split.js';
export declare class ProjectionComponent extends Split<VgProjection> {
    specifiedProjection: Projection<SignalRef>;
    size: SignalRef[];
    data: (string | SignalRef)[];
    merged: boolean;
    constructor(name: string, specifiedProjection: Projection<SignalRef>, size: SignalRef[], data: (string | SignalRef)[]);
    /**
     * Whether the projection parameters should fit provided data.
     */
    get isFit(): boolean;
}
//# sourceMappingURL=component.d.ts.map