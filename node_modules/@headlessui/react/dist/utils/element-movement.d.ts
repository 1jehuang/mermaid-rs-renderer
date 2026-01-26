export declare const ElementPositionState: {
    Idle: {
        kind: "Idle";
    };
    Tracked: (position: string) => {
        kind: "Tracked";
        position: string;
    };
    Moved: {
        kind: "Moved";
    };
};
type ResolvedStates<T extends Record<string, any>> = {
    [K in keyof T]: T[K] extends (...args: any[]) => infer R ? R : T[K];
}[keyof T];
export type ElementPositionState = ResolvedStates<typeof ElementPositionState>;
export declare function computeVisualPosition(element: HTMLElement): string;
export declare function detectMovement(target: HTMLElement, state: ResolvedStates<typeof ElementPositionState>, onMove: () => void): () => void;
export {};
