export declare const diagram: {
    db: {
        clear: () => void;
    };
    renderer: {
        draw: (text: string, id: string) => Promise<void>;
    };
    parser: {
        parse: () => void;
    };
    styles: () => void;
    injectUtils: (_log: Record<keyof typeof import("./mermaidUtils.js").LEVELS, typeof console.log>, _setLogLevel: any, _getConfig: any, _sanitizeText: any, _setupGraphViewbox: any) => void;
};
