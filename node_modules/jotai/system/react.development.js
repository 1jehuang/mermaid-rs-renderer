'use client';
System.register(['react', 'jotai/vanilla', 'jotai/vanilla/internals'], (function (exports) {
  'use strict';
  var createContext, useContext, useRef, createElement, React, useReducer, useEffect, useDebugValue, useCallback, getDefaultStore, createStore, INTERNAL_registerAbortHandler;
  return {
    setters: [function (module) {
      createContext = module.createContext;
      useContext = module.useContext;
      useRef = module.useRef;
      createElement = module.createElement;
      React = module.default;
      useReducer = module.useReducer;
      useEffect = module.useEffect;
      useDebugValue = module.useDebugValue;
      useCallback = module.useCallback;
    }, function (module) {
      getDefaultStore = module.getDefaultStore;
      createStore = module.createStore;
    }, function (module) {
      INTERNAL_registerAbortHandler = module.INTERNAL_registerAbortHandler;
    }],
    execute: (function () {

      exports({
        Provider: Provider,
        useAtom: useAtom,
        useAtomValue: useAtomValue,
        useSetAtom: useSetAtom,
        useStore: useStore
      });

      const StoreContext = createContext(
        void 0
      );
      function useStore(options) {
        const store = useContext(StoreContext);
        return (options == null ? void 0 : options.store) || store || getDefaultStore();
      }
      function Provider({
        children,
        store
      }) {
        const storeRef = useRef(null);
        if (store) {
          return createElement(StoreContext.Provider, { value: store }, children);
        }
        if (storeRef.current === null) {
          storeRef.current = createStore();
        }
        return createElement(
          StoreContext.Provider,
          {
            // TODO: If this is not a false positive, consider using useState instead of useRef like https://github.com/pmndrs/jotai/pull/2771
            // eslint-disable-next-line react-hooks/refs
            value: storeRef.current
          },
          children
        );
      }

      const isPromiseLike = (x) => typeof (x == null ? void 0 : x.then) === "function";
      const attachPromiseStatus = (promise) => {
        if (!promise.status) {
          promise.status = "pending";
          promise.then(
            (v) => {
              promise.status = "fulfilled";
              promise.value = v;
            },
            (e) => {
              promise.status = "rejected";
              promise.reason = e;
            }
          );
        }
      };
      const use = React.use || // A shim for older React versions
      ((promise) => {
        if (promise.status === "pending") {
          throw promise;
        } else if (promise.status === "fulfilled") {
          return promise.value;
        } else if (promise.status === "rejected") {
          throw promise.reason;
        } else {
          attachPromiseStatus(promise);
          throw promise;
        }
      });
      const continuablePromiseMap = /* @__PURE__ */ new WeakMap();
      const createContinuablePromise = (promise, getValue) => {
        let continuablePromise = continuablePromiseMap.get(promise);
        if (!continuablePromise) {
          continuablePromise = new Promise((resolve, reject) => {
            let curr = promise;
            const onFulfilled = (me) => (v) => {
              if (curr === me) {
                resolve(v);
              }
            };
            const onRejected = (me) => (e) => {
              if (curr === me) {
                reject(e);
              }
            };
            const onAbort = () => {
              try {
                const nextValue = getValue();
                if (isPromiseLike(nextValue)) {
                  continuablePromiseMap.set(nextValue, continuablePromise);
                  curr = nextValue;
                  nextValue.then(onFulfilled(nextValue), onRejected(nextValue));
                  INTERNAL_registerAbortHandler(nextValue, onAbort);
                } else {
                  resolve(nextValue);
                }
              } catch (e) {
                reject(e);
              }
            };
            promise.then(onFulfilled(promise), onRejected(promise));
            INTERNAL_registerAbortHandler(promise, onAbort);
          });
          continuablePromiseMap.set(promise, continuablePromise);
        }
        return continuablePromise;
      };
      function useAtomValue(atom, options) {
        const { delay, unstable_promiseStatus: promiseStatus = !React.use } = options || {};
        const store = useStore(options);
        const [[valueFromReducer, storeFromReducer, atomFromReducer], rerender] = useReducer(
          (prev) => {
            const nextValue = store.get(atom);
            if (Object.is(prev[0], nextValue) && prev[1] === store && prev[2] === atom) {
              return prev;
            }
            return [nextValue, store, atom];
          },
          void 0,
          () => [store.get(atom), store, atom]
        );
        let value = valueFromReducer;
        if (storeFromReducer !== store || atomFromReducer !== atom) {
          rerender();
          value = store.get(atom);
        }
        useEffect(() => {
          const unsub = store.sub(atom, () => {
            if (promiseStatus) {
              try {
                const value2 = store.get(atom);
                if (isPromiseLike(value2)) {
                  attachPromiseStatus(
                    createContinuablePromise(value2, () => store.get(atom))
                  );
                }
              } catch (e) {
              }
            }
            if (typeof delay === "number") {
              setTimeout(rerender, delay);
              return;
            }
            rerender();
          });
          rerender();
          return unsub;
        }, [store, atom, delay, promiseStatus]);
        useDebugValue(value);
        if (isPromiseLike(value)) {
          const promise = createContinuablePromise(value, () => store.get(atom));
          if (promiseStatus) {
            attachPromiseStatus(promise);
          }
          return use(promise);
        }
        return value;
      }

      function useSetAtom(atom, options) {
        const store = useStore(options);
        const setAtom = useCallback(
          (...args) => {
            if (!("write" in atom)) {
              throw new Error("not writable atom");
            }
            return store.set(atom, ...args);
          },
          [store, atom]
        );
        return setAtom;
      }

      function useAtom(atom, options) {
        return [
          useAtomValue(atom, options),
          // We do wrong type assertion here, which results in throwing an error.
          useSetAtom(atom, options)
        ];
      }

    })
  };
}));
