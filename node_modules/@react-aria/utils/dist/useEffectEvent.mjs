import {useLayoutEffect as $f0a04ccd8dbdd83b$export$e5c5a5f917a5871c} from "./useLayoutEffect.mjs";
import $lmaYr$react, {useRef as $lmaYr$useRef, useCallback as $lmaYr$useCallback} from "react";

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

var $8ae05eaa5c114e9c$var$_React_useInsertionEffect;
// Use the earliest effect type possible. useInsertionEffect runs during the mutation phase,
// before all layout effects, but is available only in React 18 and later.
const $8ae05eaa5c114e9c$var$useEarlyEffect = ($8ae05eaa5c114e9c$var$_React_useInsertionEffect = (0, $lmaYr$react)['useInsertionEffect']) !== null && $8ae05eaa5c114e9c$var$_React_useInsertionEffect !== void 0 ? $8ae05eaa5c114e9c$var$_React_useInsertionEffect : (0, $f0a04ccd8dbdd83b$export$e5c5a5f917a5871c);
function $8ae05eaa5c114e9c$export$7f54fc3180508a52(fn) {
    const ref = (0, $lmaYr$useRef)(null);
    $8ae05eaa5c114e9c$var$useEarlyEffect(()=>{
        ref.current = fn;
    }, [
        fn
    ]);
    // @ts-ignore
    return (0, $lmaYr$useCallback)((...args)=>{
        const f = ref.current;
        return f === null || f === void 0 ? void 0 : f(...args);
    }, []);
}


export {$8ae05eaa5c114e9c$export$7f54fc3180508a52 as useEffectEvent};
//# sourceMappingURL=useEffectEvent.module.js.map
