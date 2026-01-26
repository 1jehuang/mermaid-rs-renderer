/**
 * Options to pass to {@link parseMMD}
 */
export type ParseMDDOptions = {
    /**
     * - Puppeteer viewport (e.g. `width`, `height`, `deviceScaleFactor`)
     */
    viewport?: puppeteer.Viewport | undefined;
    /**
     * - Background color.
     */
    backgroundColor?: string | undefined;
    /**
     * - Mermaid config.
     */
    mermaidConfig?: import("mermaid").MermaidConfig | undefined;
    /**
     * - Optional CSS text.
     */
    myCSS?: string | undefined;
    /**
     * - If set, scale PDF to fit chart.
     */
    pdfFit?: boolean | undefined;
    /**
     * - The id attribute for the SVG element to be rendered.
     */
    svgId?: string | undefined;
    /**
     * - Icon packages to use.
     */
    iconPacks?: string[] | undefined;
    /**
     * - IconPack Json file name and url to use.
     *
     * /**
     * Render a mermaid diagram.
     */
    iconPacksNamesAndUrls?: string[] | undefined;
};
/**
 * Markdown image properties
 * Used to create a markdown image that looks like `![alt](url "title")`
 */
export type MarkdownImageProps = {
    /**
     * - Path to image.
     */
    url: string;
    /**
     * - Image alt text, required.
     */
    alt: string;
    /**
     * - Optional image title text.
     */
    title?: string | null | undefined;
};
/**
 * Renders a mermaid diagram or mermaid markdown file.
 *
 * @param {`${string}.${"md" | "markdown"}` | string | undefined} input - If this ends with `.md`/`.markdown`,
 * path to a markdown file containing mermaid.
 * If this is a string, loads the mermaid definition from the given file.
 * If this is `undefined`, loads the mermaid definition from stdin.
 * @param {`${string}.${"md" | "markdown" | "svg" | "png" | "pdf"}` | "/dev/stdout"} output - Path to the output file.
 * @param {Object} [opts] - Options
 * @param {import("puppeteer").LaunchOptions} [opts.puppeteerConfig] - Puppeteer launch options.
 * @param {boolean} [opts.quiet] - If set, suppress log output.
 * @param {"svg" | "png" | "pdf"} [opts.outputFormat] - Mermaid output format.
 * @param {string} [opts.artefacts] - Path to the artefacts directory.
 * Defaults to `output` extension. Overrides `output` extension if set.
 * @param {ParseMDDOptions} [opts.parseMMDOptions] - Options to pass to {@link parseMMDOptions}.
 */
export function run(input: `${string}.${"md" | "markdown"}` | string | undefined, output: `${string}.${"md" | "markdown" | "svg" | "png" | "pdf"}` | "/dev/stdout", { puppeteerConfig, quiet, outputFormat, parseMMDOptions, artefacts }?: {
    puppeteerConfig?: puppeteer.LaunchOptions | undefined;
    quiet?: boolean | undefined;
    outputFormat?: "svg" | "png" | "pdf" | undefined;
    artefacts?: string | undefined;
    parseMMDOptions?: ParseMDDOptions | undefined;
}): Promise<void>;
/**
 * @typedef {Object} ParseMDDOptions Options to pass to {@link parseMMD}
 * @property {import("puppeteer").Viewport} [viewport] - Puppeteer viewport (e.g. `width`, `height`, `deviceScaleFactor`)
 * @property {string | "transparent"} [backgroundColor] - Background color.
 * @property {Parameters<import("mermaid")["default"]["initialize"]>[0]} [mermaidConfig] - Mermaid config.
 * @property {CSSStyleDeclaration["cssText"]} [myCSS] - Optional CSS text.
 * @property {boolean} [pdfFit] - If set, scale PDF to fit chart.
 * @property {string} [svgId] - The id attribute for the SVG element to be rendered.
 * @property {string[]} [iconPacks] - Icon packages to use.
 * @property {string[]} [iconPacksNamesAndUrls] - IconPack Json file name and url to use.

/**
 * Render a mermaid diagram.
 *
 * @param {import("puppeteer").Browser | import("puppeteer").BrowserContext} browser - Puppeteer Browser
 * @param {string} definition - Mermaid diagram definition
 * @param {"svg" | "png" | "pdf"} outputFormat - Mermaid output format.
 * @param {ParseMDDOptions} [opt] - Options, see {@link ParseMDDOptions} for details.
 * @returns {Promise<{title: string | null, desc: string | null, data: Uint8Array}>} The output file in bytes,
 * with optional metadata.
 */
export function renderMermaid(browser: import("puppeteer").Browser | import("puppeteer").BrowserContext, definition: string, outputFormat: "svg" | "png" | "pdf", { viewport, backgroundColor, mermaidConfig, myCSS, pdfFit, svgId, iconPacks, iconPacksNamesAndUrls }?: ParseMDDOptions): Promise<{
    title: string | null;
    desc: string | null;
    data: Uint8Array;
}>;
export function cli(): Promise<void>;
/**
 * Prints an error to stderr, then closes with exit code 1
 *
 * @param {string} message - The message to print to `stderr`.
 * @returns {never} Quits Node.JS, so never returns.
 */
export function error(message: string): never;
import puppeteer from 'puppeteer';
//# sourceMappingURL=index.d.ts.map