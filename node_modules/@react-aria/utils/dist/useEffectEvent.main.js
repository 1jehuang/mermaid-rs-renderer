var $78605a5d7424e31b$exports = require("./useLayoutEffect.main.js");
var $gErir$react = require("react");


function $parcel$interopDefault(a) {
  return a && a.__esModule ? a.default : a;
}

function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

$parcel$export(module.exports, "useEffectEvent", () => $1254e5bb94ac8761$export$7f54fc3180508a52);
/*
 * Copyright 2023 Adobe. All rights reserved.
 * This file is licensed to you under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License. You may obtain a copy
 * of the License at http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
 * OF ANY KIND, either express or implied. See the License for the specific language
 * governing permissions and limitations under the License.
 */ 

var $1254e5bb94ac8761$var$_React_useInsertionEffect;
// Use the earliest effect type possible. useInsertionEffect runs during the mutation phase,
// before all layout effects, but is available only in React 18 and later.
const $1254e5bb94ac8761$var$useEarlyEffect = ($1254e5bb94ac8761$var$_React_useInsertionEffect = (0, ($parcel$interopDefault($gErir$react)))['useInsertionEffect']) !== null && $1254e5bb94ac8761$var$_React_useInsertionEffect !== void 0 ? $1254e5bb94ac8761$var$_React_useInsertionEffect : (0, $78605a5d7424e31b$exports.useLayoutEffect);
function $1254e5bb94ac8761$export$7f54fc3180508a52(fn) {
    const ref = (0, $gErir$react.useRef)(null);
    $1254e5bb94ac8761$var$useEarlyEffect(()=>{
        ref.current = fn;
    }, [
        fn
    ]);
    // @ts-ignore
    return (0, $gErir$react.useCallback)((...args)=>{
        const f = ref.current;
        return f === null || f === void 0 ? void 0 : f(...args);
    }, []);
}


//# sourceMappingURL=useEffectEvent.main.js.map
