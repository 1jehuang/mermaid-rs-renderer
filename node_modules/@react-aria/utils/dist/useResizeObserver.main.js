var $1254e5bb94ac8761$exports = require("./useEffectEvent.main.js");
var $aM4zL$react = require("react");


function $parcel$export(e, n, v, s) {
  Object.defineProperty(e, n, {get: v, set: s, enumerable: true, configurable: true});
}

$parcel$export(module.exports, "useResizeObserver", () => $37733e1652f47193$export$683480f191c0e3ea);


function $37733e1652f47193$var$hasResizeObserver() {
    return typeof window.ResizeObserver !== 'undefined';
}
function $37733e1652f47193$export$683480f191c0e3ea(options) {
    // Only call onResize from inside the effect, otherwise we'll void our assumption that
    // useEffectEvents are safe to pass in.
    const { ref: ref, box: box, onResize: onResize } = options;
    let onResizeEvent = (0, $1254e5bb94ac8761$exports.useEffectEvent)(onResize);
    (0, $aM4zL$react.useEffect)(()=>{
        let element = ref === null || ref === void 0 ? void 0 : ref.current;
        if (!element) return;
        if (!$37733e1652f47193$var$hasResizeObserver()) {
            window.addEventListener('resize', onResizeEvent, false);
            return ()=>{
                window.removeEventListener('resize', onResizeEvent, false);
            };
        } else {
            const resizeObserverInstance = new window.ResizeObserver((entries)=>{
                if (!entries.length) return;
                onResizeEvent();
            });
            resizeObserverInstance.observe(element, {
                box: box
            });
            return ()=>{
                if (element) resizeObserverInstance.unobserve(element);
            };
        }
    }, [
        ref,
        box
    ]);
}


//# sourceMappingURL=useResizeObserver.main.js.map
