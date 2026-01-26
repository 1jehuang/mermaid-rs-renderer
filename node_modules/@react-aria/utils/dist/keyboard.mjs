import {isMac as $c87311424ea30a05$export$9ac100e40613ea10} from "./platform.mjs";

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
function $21f1aa98acb08317$export$16792effe837dba3(e) {
    if ((0, $c87311424ea30a05$export$9ac100e40613ea10)()) return e.metaKey;
    return e.ctrlKey;
}
// HTML input types that do not cause the software keyboard to appear.
const $21f1aa98acb08317$var$nonTextInputTypes = new Set([
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
function $21f1aa98acb08317$export$c57958e35f31ed73(target) {
    return target instanceof HTMLInputElement && !$21f1aa98acb08317$var$nonTextInputTypes.has(target.type) || target instanceof HTMLTextAreaElement || target instanceof HTMLElement && target.isContentEditable;
}


export {$21f1aa98acb08317$export$16792effe837dba3 as isCtrlKeyPressed, $21f1aa98acb08317$export$c57958e35f31ed73 as willOpenKeyboard};
//# sourceMappingURL=keyboard.module.js.map
