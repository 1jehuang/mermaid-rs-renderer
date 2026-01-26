import type { SignalRef } from 'vega';
import { DatumDef, FieldDef, Format } from '../channeldef.js';
import { Config } from '../config.js';
import { ScaleType } from '../scale.js';
import { Type } from '../type.js';
import { TimeUnit } from './../timeunit.js';
export declare function isCustomFormatType(formatType: string): boolean;
export declare function formatSignalRef({ fieldOrDatumDef, format, formatType, expr, normalizeStack, config, }: {
    fieldOrDatumDef: FieldDef<string> | DatumDef<string>;
    format: Format;
    formatType: string;
    expr?: 'datum' | 'parent' | 'datum.datum';
    normalizeStack?: boolean;
    config: Config;
}): {
    signal: string;
};
export declare function formatCustomType({ fieldOrDatumDef, format, formatType, expr, normalizeStack, config, field, }: {
    fieldOrDatumDef: FieldDef<string> | DatumDef<string>;
    format: Format;
    formatType: string;
    expr?: 'datum' | 'parent' | 'datum.datum';
    normalizeStack?: boolean;
    config: Config;
    field?: string;
}): {
    signal: string;
};
export declare function guideFormat(fieldOrDatumDef: FieldDef<string> | DatumDef<string>, type: Type, format: Format, formatType: string | SignalRef, config: Config, omitTimeFormatConfig: boolean): string | {
    signal: string;
};
export declare function guideFormatType(formatType: string | SignalRef, fieldOrDatumDef: FieldDef<string> | DatumDef<string>, scaleType: ScaleType): "number" | SignalRef | "time" | "utc";
/**
 * Returns number format for a fieldDef.
 */
export declare function numberFormat({ type, specifiedFormat, config, normalizeStack, }: {
    type: Type;
    specifiedFormat?: Format;
    config: Config;
    normalizeStack?: boolean;
}): string;
/**
 * Returns time format for a fieldDef for use in guides.
 */
export declare function timeFormat({ specifiedFormat, timeUnit, config, omitTimeFormatConfig, }: {
    specifiedFormat?: string;
    timeUnit?: TimeUnit;
    config: Config;
    omitTimeFormatConfig?: boolean;
}): string | {
    signal: string;
};
export declare function binFormatExpression(startField: string, endField: string, format: Format, formatType: string, config: Config): string;
/**
 * Returns the time expression used for axis/legend labels or text mark for a temporal field
 */
export declare function timeFormatExpression({ field, timeUnit, format, formatType, rawTimeFormat, isUTCScale, }: {
    field: string;
    timeUnit?: TimeUnit;
    format?: Format;
    formatType?: string;
    rawTimeFormat?: string;
    isUTCScale?: boolean;
}): string;
//# sourceMappingURL=format.d.ts.map