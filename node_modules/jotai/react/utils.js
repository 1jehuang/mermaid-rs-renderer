'use client';
'use strict';

var react$1 = require('react');
var react = require('jotai/react');
var utils = require('jotai/vanilla/utils');
var vanilla = require('jotai/vanilla');

function useResetAtom(anAtom, options) {
  var setAtom = react.useSetAtom(anAtom, options);
  var resetAtom = react$1.useCallback(function () {
    return setAtom(utils.RESET);
  }, [setAtom]);
  return resetAtom;
}

function useReducerAtom(anAtom, reducer, options) {
  if (process.env.NODE_ENV !== 'production') {
    console.warn('[DEPRECATED] useReducerAtom is deprecated and will be removed in the future. Please create your own version using the recipe. https://github.com/pmndrs/jotai/pull/2467');
  }
  var _useAtom = react.useAtom(anAtom, options),
    state = _useAtom[0],
    setState = _useAtom[1];
  var dispatch = react$1.useCallback(function (action) {
    setState(function (prev) {
      return reducer(prev, action);
    });
  }, [setState, reducer]);
  return [state, dispatch];
}

function useAtomCallback(callback, options) {
  var anAtom = react$1.useMemo(function () {
    return vanilla.atom(null, function (get, set) {
      for (var _len = arguments.length, args = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
        args[_key - 2] = arguments[_key];
      }
      return callback.apply(void 0, [get, set].concat(args));
    });
  }, [callback]);
  return react.useSetAtom(anAtom, options);
}

function _arrayLikeToArray(r, a) {
  (null == a || a > r.length) && (a = r.length);
  for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e];
  return n;
}
function _createForOfIteratorHelperLoose(r, e) {
  var t = "undefined" != typeof Symbol && r[Symbol.iterator] || r["@@iterator"];
  if (t) return (t = t.call(r)).next.bind(t);
  if (Array.isArray(r) || (t = _unsupportedIterableToArray(r)) || e) {
    t && (r = t);
    var o = 0;
    return function () {
      return o >= r.length ? {
        done: true
      } : {
        done: false,
        value: r[o++]
      };
    };
  }
  throw new TypeError("Invalid attempt to iterate non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _unsupportedIterableToArray(r, a) {
  if (r) {
    if ("string" == typeof r) return _arrayLikeToArray(r, a);
    var t = {}.toString.call(r).slice(8, -1);
    return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0;
  }
}

var hydratedMap = new WeakMap();
function useHydrateAtoms(values, options) {
  var store = react.useStore(options);
  var hydratedSet = getHydratedSet(store);
  for (var _iterator = _createForOfIteratorHelperLoose(values), _step; !(_step = _iterator()).done;) {
    var _step$value = _step.value,
      atom = _step$value[0],
      args = _arrayLikeToArray(_step$value).slice(1);
    if (!hydratedSet.has(atom) || options != null && options.dangerouslyForceHydrate) {
      hydratedSet.add(atom);
      store.set.apply(store, [atom].concat(args));
    }
  }
}
var getHydratedSet = function getHydratedSet(store) {
  var hydratedSet = hydratedMap.get(store);
  if (!hydratedSet) {
    hydratedSet = new WeakSet();
    hydratedMap.set(store, hydratedSet);
  }
  return hydratedSet;
};

exports.useAtomCallback = useAtomCallback;
exports.useHydrateAtoms = useHydrateAtoms;
exports.useReducerAtom = useReducerAtom;
exports.useResetAtom = useResetAtom;
