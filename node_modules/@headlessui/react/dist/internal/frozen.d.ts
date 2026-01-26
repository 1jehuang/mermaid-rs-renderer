import React from 'react';
export declare const Frozen: React.ForwardRefExoticComponent<{
    children: React.ReactNode;
    freeze: boolean;
} & React.RefAttributes<HTMLElement>>;
export declare function useFrozenData<T>(freeze: boolean, data: T): T;
