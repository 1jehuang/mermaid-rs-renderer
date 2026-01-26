import { VgValueRef } from '../../../vega.schema.js';
import { ScaleComponent } from '../../scale/component.js';
export interface ScaledZeroOrMinOrMaxProps {
    scaleName: string;
    scale: ScaleComponent;
    mode: 'zeroOrMin' | {
        zeroOrMax: {
            widthSignal: string;
            heightSignal: string;
        };
    };
}
export declare function scaledZeroOrMinOrMax({ scaleName, scale, mode }: ScaledZeroOrMinOrMaxProps): VgValueRef | undefined;
//# sourceMappingURL=scaledZeroOrMinOrMax.d.ts.map