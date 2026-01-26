import { SignalRef } from 'vega';
import { Config } from '../../config.js';
import { Encoding } from '../../encoding.js';
import { MarkDef } from '../../mark.js';
export declare function initMarkdef(originalMarkDef: MarkDef, encoding: Encoding<string>, config: Config<SignalRef>): MarkDef<"text" | "arc" | "area" | "image" | "line" | "rect" | "rule" | "trail" | "bar" | "point" | "tick" | "circle" | "square" | "geoshape", SignalRef>;
export declare const DEFAULT_REDUCED_OPACITY = 0.7;
export declare function defaultFilled(markDef: MarkDef, config: Config<SignalRef>, { graticule }: {
    graticule: boolean;
}): boolean;
//# sourceMappingURL=init.d.ts.map