import type { MermaidConfig } from 'mermaid';
export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error' | 'fatal';
export declare const LEVELS: Record<LogLevel, number>;
export declare const log: Record<keyof typeof LEVELS, typeof console.log>;
export declare let setLogLevel: (level: keyof typeof LEVELS | number) => void;
export declare let getConfig: () => MermaidConfig;
export declare let sanitizeText: (str: string) => string;
export declare let setupGraphViewbox: (graph: any, svgElem: any, padding: any, useMaxWidth: boolean) => void;
export declare const injectUtils: (_log: Record<keyof typeof LEVELS, typeof console.log>, _setLogLevel: any, _getConfig: any, _sanitizeText: any, _setupGraphViewbox: any) => void;
