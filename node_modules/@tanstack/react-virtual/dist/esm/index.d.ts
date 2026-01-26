import { Virtualizer, PartialKeys, VirtualizerOptions } from '@tanstack/virtual-core';
export * from '@tanstack/virtual-core';
export type ReactVirtualizerOptions<TScrollElement extends Element | Window, TItemElement extends Element> = VirtualizerOptions<TScrollElement, TItemElement> & {
    useFlushSync?: boolean;
};
export declare function useVirtualizer<TScrollElement extends Element, TItemElement extends Element>(options: PartialKeys<ReactVirtualizerOptions<TScrollElement, TItemElement>, 'observeElementRect' | 'observeElementOffset' | 'scrollToFn'>): Virtualizer<TScrollElement, TItemElement>;
export declare function useWindowVirtualizer<TItemElement extends Element>(options: PartialKeys<ReactVirtualizerOptions<Window, TItemElement>, 'getScrollElement' | 'observeElementRect' | 'observeElementOffset' | 'scrollToFn'>): Virtualizer<Window, TItemElement>;
