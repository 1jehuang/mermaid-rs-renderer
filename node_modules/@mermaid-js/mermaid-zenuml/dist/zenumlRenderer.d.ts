/**
 * Draws a Zen UML in the tag with id: id based on the graph definition in text.
 *
 * @param text - The text of the diagram
 * @param id - The id of the diagram which will be used as a DOM element id¨
 */
export declare const draw: (text: string, id: string) => Promise<void>;
declare const _default: {
    draw: (text: string, id: string) => Promise<void>;
};
export default _default;
