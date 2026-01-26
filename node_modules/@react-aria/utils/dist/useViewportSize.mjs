import {willOpenKeyboard as $21f1aa98acb08317$export$c57958e35f31ed73} from "./keyboard.mjs";
import {useState as $fuDHA$useState, useEffect as $fuDHA$useEffect} from "react";
import {useIsSSR as $fuDHA$useIsSSR} from "@react-aria/ssr";

/*
 * Copyright 2020 Adobe. All rights reserved.
 * This file is licensed to you under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License. You may obtain a copy
 * of the License at http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
 * OF ANY KIND, either express or implied. See the License for the specific language
 * governing permissions and limitations under the License.
 */ 


let $5df64b3807dc15ee$var$visualViewport = typeof document !== 'undefined' && window.visualViewport;
function $5df64b3807dc15ee$export$d699905dd57c73ca() {
    let isSSR = (0, $fuDHA$useIsSSR)();
    let [size, setSize] = (0, $fuDHA$useState)(()=>isSSR ? {
            width: 0,
            height: 0
        } : $5df64b3807dc15ee$var$getViewportSize());
    (0, $fuDHA$useEffect)(()=>{
        // Use visualViewport api to track available height even on iOS virtual keyboard opening
        let onResize = ()=>{
            // Ignore updates when zoomed.
            if ($5df64b3807dc15ee$var$visualViewport && $5df64b3807dc15ee$var$visualViewport.scale > 1) return;
            setSize((size)=>{
                let newSize = $5df64b3807dc15ee$var$getViewportSize();
                if (newSize.width === size.width && newSize.height === size.height) return size;
                return newSize;
            });
        };
        // When closing the keyboard, iOS does not fire the visual viewport resize event until the animation is complete.
        // We can anticipate this and resize early by handling the blur event and using the layout size.
        let frame;
        let onBlur = (e)=>{
            if ($5df64b3807dc15ee$var$visualViewport && $5df64b3807dc15ee$var$visualViewport.scale > 1) return;
            if ((0, $21f1aa98acb08317$export$c57958e35f31ed73)(e.target)) // Wait one frame to see if a new element gets focused.
            frame = requestAnimationFrame(()=>{
                if (!document.activeElement || !(0, $21f1aa98acb08317$export$c57958e35f31ed73)(document.activeElement)) setSize((size)=>{
                    let newSize = {
                        width: window.innerWidth,
                        height: window.innerHeight
                    };
                    if (newSize.width === size.width && newSize.height === size.height) return size;
                    return newSize;
                });
            });
        };
        window.addEventListener('blur', onBlur, true);
        if (!$5df64b3807dc15ee$var$visualViewport) window.addEventListener('resize', onResize);
        else $5df64b3807dc15ee$var$visualViewport.addEventListener('resize', onResize);
        return ()=>{
            cancelAnimationFrame(frame);
            window.removeEventListener('blur', onBlur, true);
            if (!$5df64b3807dc15ee$var$visualViewport) window.removeEventListener('resize', onResize);
            else $5df64b3807dc15ee$var$visualViewport.removeEventListener('resize', onResize);
        };
    }, []);
    return size;
}
function $5df64b3807dc15ee$var$getViewportSize() {
    return {
        // Multiply by the visualViewport scale to get the "natural" size, unaffected by pinch zooming.
        width: $5df64b3807dc15ee$var$visualViewport ? $5df64b3807dc15ee$var$visualViewport.width * $5df64b3807dc15ee$var$visualViewport.scale : window.innerWidth,
        height: $5df64b3807dc15ee$var$visualViewport ? $5df64b3807dc15ee$var$visualViewport.height * $5df64b3807dc15ee$var$visualViewport.scale : window.innerHeight
    };
}


export {$5df64b3807dc15ee$export$d699905dd57c73ca as useViewportSize};
//# sourceMappingURL=useViewportSize.module.js.map
