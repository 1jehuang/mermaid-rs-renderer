var $a333d78d58ab4731$exports = require("./isElementVisible.main.js");


function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

$parcel$export(module.exports, "isFocusable", () => $506b33fd893eab7d$export$4c063cf1350e6fed);
$parcel$export(module.exports, "isTabbable", () => $506b33fd893eab7d$export$bebd5a1431fec25d);
/*
 * Copyright 2025 Adobe. All rights reserved.
 * This file is licensed to you under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License. You may obtain a copy
 * of the License at http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
 * OF ANY KIND, either express or implied. See the License for the specific language
 * governing permissions and limitations under the License.
 */ 
const $506b33fd893eab7d$var$focusableElements = [
    'input:not([disabled]):not([type=hidden])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    'button:not([disabled])',
    'a[href]',
    'area[href]',
    'summary',
    'iframe',
    'object',
    'embed',
    'audio[controls]',
    'video[controls]',
    '[contenteditable]:not([contenteditable^="false"])',
    'permission'
];
const $506b33fd893eab7d$var$FOCUSABLE_ELEMENT_SELECTOR = $506b33fd893eab7d$var$focusableElements.join(':not([hidden]),') + ',[tabindex]:not([disabled]):not([hidden])';
$506b33fd893eab7d$var$focusableElements.push('[tabindex]:not([tabindex="-1"]):not([disabled])');
const $506b33fd893eab7d$var$TABBABLE_ELEMENT_SELECTOR = $506b33fd893eab7d$var$focusableElements.join(':not([hidden]):not([tabindex="-1"]),');
function $506b33fd893eab7d$export$4c063cf1350e6fed(element) {
    return element.matches($506b33fd893eab7d$var$FOCUSABLE_ELEMENT_SELECTOR) && (0, $a333d78d58ab4731$exports.isElementVisible)(element) && !$506b33fd893eab7d$var$isInert(element);
}
function $506b33fd893eab7d$export$bebd5a1431fec25d(element) {
    return element.matches($506b33fd893eab7d$var$TABBABLE_ELEMENT_SELECTOR) && (0, $a333d78d58ab4731$exports.isElementVisible)(element) && !$506b33fd893eab7d$var$isInert(element);
}
function $506b33fd893eab7d$var$isInert(element) {
    let node = element;
    while(node != null){
        if (node instanceof node.ownerDocument.defaultView.HTMLElement && node.inert) return true;
        node = node.parentElement;
    }
    return false;
}


//# sourceMappingURL=isFocusable.main.js.map
