import { VgValueRef } from '../../../vega.schema.js';
import { UnitModel } from '../../unit.js';
export { color } from './color.js';
export { nonPosition } from './nonposition.js';
export { pointPosition } from './position-point.js';
export { pointOrRangePosition, rangePosition } from './position-range.js';
export { rectPosition } from './position-rect.js';
export { text } from './text.js';
export { tooltip } from './tooltip.js';
export type Ignore = Record<'color' | 'size' | 'orient' | 'align' | 'baseline' | 'theta', 'ignore' | 'include'>;
export declare function baseEncodeEntry(model: UnitModel, ignore: Ignore): {
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: unknown;
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRoleDescription: unknown;
    aria: unknown;
    ariaRole: unknown;
    description: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    description: import("vega").SignalRef | {
        value: string;
    };
    ariaRoleDescription: unknown;
    aria: unknown;
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: unknown;
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRole: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: unknown;
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRoleDescription: {
        value: string | import("vega").SignalRef;
    };
    aria: unknown;
    ariaRole: unknown;
    description: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    description: import("vega").SignalRef | {
        value: string;
    };
    ariaRoleDescription: {
        value: string | import("vega").SignalRef;
    };
    aria: unknown;
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: unknown;
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRole: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: VgValueRef | (VgValueRef & {
        test?: string;
    })[] | {
        signal: string;
    };
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRoleDescription: unknown;
    aria: unknown;
    ariaRole: unknown;
    description: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    description: import("vega").SignalRef | {
        value: string;
    };
    ariaRoleDescription: unknown;
    aria: unknown;
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: VgValueRef | (VgValueRef & {
        test?: string;
    })[] | {
        signal: string;
    };
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRole: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: VgValueRef | (VgValueRef & {
        test?: string;
    })[] | {
        signal: string;
    };
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRoleDescription: {
        value: string | import("vega").SignalRef;
    };
    aria: unknown;
    ariaRole: unknown;
    description: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
} | {
    description: import("vega").SignalRef | {
        value: string;
    };
    ariaRoleDescription: {
        value: string | import("vega").SignalRef;
    };
    aria: unknown;
    x?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    fill: unknown;
    stroke: unknown;
    strokeWidth: unknown;
    size: unknown;
    shape: unknown;
    fillOpacity: unknown;
    strokeOpacity: unknown;
    opacity: unknown;
    text: unknown;
    width: unknown;
    height: unknown;
    interpolate: unknown;
    align: unknown;
    x2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    xc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    y2?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    yc?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    strokeCap: unknown;
    strokeDash: unknown;
    strokeDashOffset: unknown;
    strokeMiterLimit: unknown;
    strokeJoin: unknown;
    strokeOffset: unknown;
    strokeForeground?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cursor: unknown;
    clip?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    path?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    innerRadius: unknown;
    outerRadius: unknown;
    startAngle: unknown;
    endAngle: unknown;
    tension: unknown;
    orient: unknown;
    url: unknown;
    baseline: unknown;
    dir: unknown;
    ellipsis: unknown;
    limit: unknown;
    dx: unknown;
    dy: unknown;
    radius: unknown;
    theta: unknown;
    angle: unknown;
    font: unknown;
    fontSize: unknown;
    fontWeight: unknown;
    fontStyle: unknown;
    tooltip: VgValueRef | (VgValueRef & {
        test?: string;
    })[] | {
        signal: string;
    };
    href: unknown;
    defined?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    cornerRadius: unknown;
    cornerRadiusTopLeft: unknown;
    cornerRadiusTopRight: unknown;
    cornerRadiusBottomRight: unknown;
    cornerRadiusBottomLeft: unknown;
    scaleX?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    scaleY?: VgValueRef | (VgValueRef & {
        test?: string;
    })[];
    ariaRole: unknown;
    aspect: unknown;
    smooth: unknown;
    blend: unknown;
    padAngle: unknown;
    lineBreak: unknown;
    lineHeight: unknown;
};
//# sourceMappingURL=base.d.ts.map