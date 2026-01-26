var $9e20cff0af27e8cc$exports = require("./platform.main.js");


function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

$parcel$export(module.exports, "isCtrlKeyPressed", () => $2308dc377e184bb0$export$16792effe837dba3);
$parcel$export(module.exports, "willOpenKeyboard", () => $2308dc377e184bb0$export$c57958e35f31ed73);
/*
 * Copyright 2024 Adobe. All rights reserved.
 * This file is licensed to you under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License. You may obtain a copy
 * of the License at http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
 * OF ANY KIND, either express or implied. See the License for the specific language
 * governing permissions and limitations under the License.
 */ 
function $2308dc377e184bb0$export$16792effe837dba3(e) {
    if ((0, $9e20cff0af27e8cc$exports.isMac)()) return e.metaKey;
    return e.ctrlKey;
}
// HTML input types that do not cause the software keyboard to appear.
const $2308dc377e184bb0$var$nonTextInputTypes = new Set([
    'checkbox',
    'radio',
    'range',
    'color',
    'file',
    'image',
    'button',
    'submit',
    'reset'
]);
function $2308dc377e184bb0$export$c57958e35f31ed73(target) {
    return target instanceof HTMLInputElement && !$2308dc377e184bb0$var$nonTextInputTypes.has(target.type) || target instanceof HTMLTextAreaElement || target instanceof HTMLElement && target.isContentEditable;
}


//# sourceMappingURL=keyboard.main.js.map
