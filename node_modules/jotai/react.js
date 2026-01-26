'use client';
'use strict';

var React = require('react');
var vanilla = require('jotai/vanilla');
var internals = require('jotai/vanilla/internals');

var StoreContext = React.createContext(undefined);
function useStore(options) {
  var store = React.useContext(StoreContext);
  return (options == null ? void 0 : options.store) || store || vanilla.getDefaultStore();
}
function Provider(_ref) {
  var children = _ref.children,
    store = _ref.store;
  var storeRef = React.useRef(null);
  if (store) {
    return React.createElement(StoreContext.Provider, {
      value: store
    }, children);
  }
  if (storeRef.current === null) {
    storeRef.current = vanilla.createStore();
  }
  return React.createElement(StoreContext.Provider, {
    value: storeRef.current
  }, children);
}

var isPromiseLike = function isPromiseLike(x) {
  return typeof (x == null ? void 0 : x.then) === 'function';
};
var attachPromiseStatus = function attachPromiseStatus(promise) {
  if (!promise.status) {
    promise.status = 'pending';
    promise.then(function (v) {
      promise.status = 'fulfilled';
      promise.value = v;
    }, function (e) {
      promise.status = 'rejected';
      promise.reason = e;
    });
  }
};
var use = React.use || function (promise) {
  if (promise.status === 'pending') {
    throw promise;
  } else if (promise.status === 'fulfilled') {
    return promise.value;
  } else if (promise.status === 'rejected') {
    throw promise.reason;
  } else {
    attachPromiseStatus(promise);
    throw promise;
  }
};
var continuablePromiseMap = new WeakMap();
var createContinuablePromise = function createContinuablePromise(promise, getValue) {
  var continuablePromise = continuablePromiseMap.get(promise);
  if (!continuablePromise) {
    continuablePromise = new Promise(function (resolve, reject) {
      var curr = promise;
      var onFulfilled = function onFulfilled(me) {
        return function (v) {
          if (curr === me) {
            resolve(v);
          }
        };
      };
      var onRejected = function onRejected(me) {
        return function (e) {
          if (curr === me) {
            reject(e);
          }
        };
      };
      var _onAbort = function onAbort() {
        try {
          var nextValue = getValue();
          if (isPromiseLike(nextValue)) {
            continuablePromiseMap.set(nextValue, continuablePromise);
            curr = nextValue;
            nextValue.then(onFulfilled(nextValue), onRejected(nextValue));
            internals.INTERNAL_registerAbortHandler(nextValue, _onAbort);
          } else {
            resolve(nextValue);
          }
        } catch (e) {
          reject(e);
        }
      };
      promise.then(onFulfilled(promise), onRejected(promise));
      internals.INTERNAL_registerAbortHandler(promise, _onAbort);
    });
    continuablePromiseMap.set(promise, continuablePromise);
  }
  return continuablePromise;
};
function useAtomValue(atom, options) {
  var _ref = options || {},
    delay = _ref.delay,
    _ref$unstable_promise = _ref.unstable_promiseStatus,
    promiseStatus = _ref$unstable_promise === void 0 ? !React.use : _ref$unstable_promise;
  var store = useStore(options);
  var _useReducer = React.useReducer(function (prev) {
      var nextValue = store.get(atom);
      if (Object.is(prev[0], nextValue) && prev[1] === store && prev[2] === atom) {
        return prev;
      }
      return [nextValue, store, atom];
    }, undefined, function () {
      return [store.get(atom), store, atom];
    }),
    _useReducer$ = _useReducer[0],
    valueFromReducer = _useReducer$[0],
    storeFromReducer = _useReducer$[1],
    atomFromReducer = _useReducer$[2],
    rerender = _useReducer[1];
  var value = valueFromReducer;
  if (storeFromReducer !== store || atomFromReducer !== atom) {
    rerender();
    value = store.get(atom);
  }
  React.useEffect(function () {
    var unsub = store.sub(atom, function () {
      if (promiseStatus) {
        try {
          var _value = store.get(atom);
          if (isPromiseLike(_value)) {
            attachPromiseStatus(createContinuablePromise(_value, function () {
              return store.get(atom);
            }));
          }
        } catch (_unused) {}
      }
      if (typeof delay === 'number') {
        setTimeout(rerender, delay);
        return;
      }
      rerender();
    });
    rerender();
    return unsub;
  }, [store, atom, delay, promiseStatus]);
  React.useDebugValue(value);
  if (isPromiseLike(value)) {
    var promise = createContinuablePromise(value, function () {
      return store.get(atom);
    });
    if (promiseStatus) {
      attachPromiseStatus(promise);
    }
    return use(promise);
  }
  return value;
}

function useSetAtom(atom, options) {
  var store = useStore(options);
  var setAtom = React.useCallback(function () {
    if (process.env.NODE_ENV !== 'production' && !('write' in atom)) {
      throw new Error('not writable atom');
    }
    for (var _len = arguments.length, args = new Array(_len), _key = 0; _key < _len; _key++) {
      args[_key] = arguments[_key];
    }
    return store.set.apply(store, [atom].concat(args));
  }, [store, atom]);
  return setAtom;
}

function useAtom(atom, options) {
  return [useAtomValue(atom, options), useSetAtom(atom, options)];
}

exports.Provider = Provider;
exports.useAtom = useAtom;
exports.useAtomValue = useAtomValue;
exports.useSetAtom = useSetAtom;
exports.useStore = useStore;
