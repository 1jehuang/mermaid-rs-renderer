/**
 * @license
 * Copyright 2017 Google Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
export type { Protocol } from 'puppeteer-core';
export * from 'puppeteer-core/internal/puppeteer-core.js';
import * as PuppeteerCore from 'puppeteer-core/internal/puppeteer-core.js';
/**
 * @public
 */
declare const puppeteer: PuppeteerCore.PuppeteerNode;
export declare const 
/**
 * @public
 */
connect: (options: PuppeteerCore.ConnectOptions) => Promise<PuppeteerCore.Browser>, 
/**
 * @public
 */
defaultArgs: (options?: PuppeteerCore.LaunchOptions) => string[], 
/**
 * @public
 */
executablePath: {
    (channel: PuppeteerCore.ChromeReleaseChannel): string;
    (options: PuppeteerCore.LaunchOptions): string;
    (): string;
}, 
/**
 * @public
 */
launch: (options?: PuppeteerCore.LaunchOptions) => Promise<PuppeteerCore.Browser>, 
/**
 * @public
 */
trimCache: () => Promise<void>;
export default puppeteer;
//# sourceMappingURL=puppeteer.d.ts.map