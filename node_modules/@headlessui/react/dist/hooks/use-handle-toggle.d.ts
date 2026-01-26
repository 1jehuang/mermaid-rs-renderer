import { type PointerEvent as ReactPointerEvent } from 'react';
export declare function useHandleToggle(cb: (event: ReactPointerEvent) => void): {
    onPointerDown: (event: ReactPointerEvent<HTMLButtonElement>) => void;
    onClick: (event: ReactPointerEvent<HTMLButtonElement>) => void;
};
