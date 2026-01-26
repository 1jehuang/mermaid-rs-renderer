import { atom } from 'jotai/vanilla';

const RESET = /* @__PURE__ */ Symbol(
  (import.meta.env ? import.meta.env.MODE : void 0) !== "production" ? "RESET" : ""
);

function atomWithReset(initialValue) {
  const anAtom = atom(
    initialValue,
    (get, set, update) => {
      const nextValue = typeof update === "function" ? update(get(anAtom)) : update;
      set(anAtom, nextValue === RESET ? initialValue : nextValue);
    }
  );
  return anAtom;
}

function atomWithReducer(initialValue, reducer) {
  return atom(initialValue, function(get, set, action) {
    set(this, reducer(get(this), action));
  });
}

let didWarnDeprecation = false;
function atomFamily(initializeAtom, areEqual) {
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production" && !didWarnDeprecation) {
    console.warn(
      "[DEPRECATED] atomFamily is deprecated and will be removed in v3. Please use the `jotai-family` package instead: https://github.com/jotaijs/jotai-family"
    );
    didWarnDeprecation = true;
  }
  let shouldRemove = null;
  const atoms = /* @__PURE__ */ new Map();
  const listeners = /* @__PURE__ */ new Set();
  const createAtom = (param) => {
    let item;
    if (areEqual === void 0) {
      item = atoms.get(param);
    } else {
      for (const [key, value] of atoms) {
        if (areEqual(key, param)) {
          item = value;
          break;
        }
      }
    }
    if (item !== void 0) {
      if (shouldRemove == null ? void 0 : shouldRemove(item[1], param)) {
        createAtom.remove(param);
      } else {
        return item[0];
      }
    }
    const newAtom = initializeAtom(param);
    atoms.set(param, [newAtom, Date.now()]);
    notifyListeners("CREATE", param, newAtom);
    return newAtom;
  };
  const notifyListeners = (type, param, atom) => {
    for (const listener of listeners) {
      listener({ type, param, atom });
    }
  };
  createAtom.unstable_listen = (callback) => {
    listeners.add(callback);
    return () => {
      listeners.delete(callback);
    };
  };
  createAtom.getParams = () => atoms.keys();
  createAtom.remove = (param) => {
    if (areEqual === void 0) {
      if (!atoms.has(param)) return;
      const [atom] = atoms.get(param);
      atoms.delete(param);
      notifyListeners("REMOVE", param, atom);
    } else {
      for (const [key, [atom]] of atoms) {
        if (areEqual(key, param)) {
          atoms.delete(key);
          notifyListeners("REMOVE", key, atom);
          break;
        }
      }
    }
  };
  createAtom.setShouldRemove = (fn) => {
    shouldRemove = fn;
    if (!shouldRemove) return;
    for (const [key, [atom, createdAt]] of atoms) {
      if (shouldRemove(createdAt, key)) {
        atoms.delete(key);
        notifyListeners("REMOVE", key, atom);
      }
    }
  };
  return createAtom;
}

const getCached$2 = (c, m, k) => (m.has(k) ? m : m.set(k, c())).get(k);
const cache1$3 = /* @__PURE__ */ new WeakMap();
const memo3 = (create, dep1, dep2, dep3) => {
  const cache2 = getCached$2(() => /* @__PURE__ */ new WeakMap(), cache1$3, dep1);
  const cache3 = getCached$2(() => /* @__PURE__ */ new WeakMap(), cache2, dep2);
  return getCached$2(create, cache3, dep3);
};
function selectAtom(anAtom, selector, equalityFn = Object.is) {
  return memo3(
    () => {
      const EMPTY = /* @__PURE__ */ Symbol();
      const selectValue = ([value, prevSlice]) => {
        if (prevSlice === EMPTY) {
          return selector(value);
        }
        const slice = selector(value, prevSlice);
        return equalityFn(prevSlice, slice) ? prevSlice : slice;
      };
      const derivedAtom = atom((get) => {
        const prev = get(derivedAtom);
        const value = get(anAtom);
        return selectValue([value, prev]);
      });
      derivedAtom.init = EMPTY;
      return derivedAtom;
    },
    anAtom,
    selector,
    equalityFn
  );
}

const frozenAtoms = /* @__PURE__ */ new WeakSet();
const deepFreeze = (value) => {
  if (typeof value !== "object" || value === null) {
    return value;
  }
  Object.freeze(value);
  const propNames = Object.getOwnPropertyNames(value);
  for (const name of propNames) {
    deepFreeze(value[name]);
  }
  return value;
};
function freezeAtom(anAtom) {
  if (frozenAtoms.has(anAtom)) {
    return anAtom;
  }
  frozenAtoms.add(anAtom);
  const origRead = anAtom.read;
  anAtom.read = function(get, options) {
    return deepFreeze(origRead.call(this, get, options));
  };
  if ("write" in anAtom) {
    const origWrite = anAtom.write;
    anAtom.write = function(get, set, ...args) {
      return origWrite.call(
        this,
        get,
        (...setArgs) => {
          if (setArgs[0] === anAtom) {
            setArgs[1] = deepFreeze(setArgs[1]);
          }
          return set(...setArgs);
        },
        ...args
      );
    };
  }
  return anAtom;
}
function freezeAtomCreator(createAtom) {
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    console.warn(
      "[DEPRECATED] freezeAtomCreator is deprecated, define it on users end"
    );
  }
  return ((...args) => freezeAtom(createAtom(...args)));
}

const getCached$1 = (c, m, k) => (m.has(k) ? m : m.set(k, c())).get(k);
const cache1$2 = /* @__PURE__ */ new WeakMap();
const memo2$1 = (create, dep1, dep2) => {
  const cache2 = getCached$1(() => /* @__PURE__ */ new WeakMap(), cache1$2, dep1);
  return getCached$1(create, cache2, dep2);
};
const cacheKeyForEmptyKeyExtractor = {};
const isWritable = (atom2) => !!atom2.write;
const isFunction = (x) => typeof x === "function";
function splitAtom(arrAtom, keyExtractor) {
  return memo2$1(
    () => {
      const mappingCache = /* @__PURE__ */ new WeakMap();
      const getMapping = (arr, prev) => {
        let mapping = mappingCache.get(arr);
        if (mapping) {
          return mapping;
        }
        const prevMapping = prev && mappingCache.get(prev);
        const atomList = [];
        const keyList = [];
        arr.forEach((item, index) => {
          const key = keyExtractor ? keyExtractor(item) : index;
          keyList[index] = key;
          const cachedAtom = prevMapping && prevMapping.atomList[prevMapping.keyList.indexOf(key)];
          if (cachedAtom) {
            atomList[index] = cachedAtom;
            return;
          }
          const read = (get) => {
            const prev2 = get(mappingAtom);
            const currArr = get(arrAtom);
            const mapping2 = getMapping(currArr, prev2 == null ? void 0 : prev2.arr);
            const index2 = mapping2.keyList.indexOf(key);
            if (index2 < 0 || index2 >= currArr.length) {
              const prevItem = arr[getMapping(arr).keyList.indexOf(key)];
              if (prevItem) {
                return prevItem;
              }
              throw new Error("splitAtom: index out of bounds for read");
            }
            return currArr[index2];
          };
          const write = (get, set, update) => {
            const prev2 = get(mappingAtom);
            const arr2 = get(arrAtom);
            const mapping2 = getMapping(arr2, prev2 == null ? void 0 : prev2.arr);
            const index2 = mapping2.keyList.indexOf(key);
            if (index2 < 0 || index2 >= arr2.length) {
              throw new Error("splitAtom: index out of bounds for write");
            }
            const nextItem = isFunction(update) ? update(arr2[index2]) : update;
            if (!Object.is(arr2[index2], nextItem)) {
              set(arrAtom, [
                ...arr2.slice(0, index2),
                nextItem,
                ...arr2.slice(index2 + 1)
              ]);
            }
          };
          atomList[index] = isWritable(arrAtom) ? atom(read, write) : atom(read);
        });
        if (prevMapping && prevMapping.keyList.length === keyList.length && prevMapping.keyList.every((x, i) => x === keyList[i])) {
          mapping = prevMapping;
        } else {
          mapping = { arr, atomList, keyList };
        }
        mappingCache.set(arr, mapping);
        return mapping;
      };
      const mappingAtom = atom((get) => {
        const prev = get(mappingAtom);
        const arr = get(arrAtom);
        const mapping = getMapping(arr, prev == null ? void 0 : prev.arr);
        return mapping;
      });
      if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
        mappingAtom.debugPrivate = true;
      }
      mappingAtom.init = void 0;
      const splittedAtom = isWritable(arrAtom) ? atom(
        (get) => get(mappingAtom).atomList,
        (get, set, action) => {
          switch (action.type) {
            case "remove": {
              const index = get(splittedAtom).indexOf(action.atom);
              if (index >= 0) {
                const arr = get(arrAtom);
                set(arrAtom, [
                  ...arr.slice(0, index),
                  ...arr.slice(index + 1)
                ]);
              }
              break;
            }
            case "insert": {
              const index = action.before ? get(splittedAtom).indexOf(action.before) : get(splittedAtom).length;
              if (index >= 0) {
                const arr = get(arrAtom);
                set(arrAtom, [
                  ...arr.slice(0, index),
                  action.value,
                  ...arr.slice(index)
                ]);
              }
              break;
            }
            case "move": {
              const index1 = get(splittedAtom).indexOf(action.atom);
              const index2 = action.before ? get(splittedAtom).indexOf(action.before) : get(splittedAtom).length;
              if (index1 >= 0 && index2 >= 0) {
                const arr = get(arrAtom);
                if (index1 < index2) {
                  set(arrAtom, [
                    ...arr.slice(0, index1),
                    ...arr.slice(index1 + 1, index2),
                    arr[index1],
                    ...arr.slice(index2)
                  ]);
                } else {
                  set(arrAtom, [
                    ...arr.slice(0, index2),
                    arr[index1],
                    ...arr.slice(index2, index1),
                    ...arr.slice(index1 + 1)
                  ]);
                }
              }
              break;
            }
          }
        }
      ) : atom((get) => get(mappingAtom).atomList);
      return splittedAtom;
    },
    arrAtom,
    keyExtractor || cacheKeyForEmptyKeyExtractor
  );
}

function atomWithDefault(getDefault) {
  const EMPTY = /* @__PURE__ */ Symbol();
  const overwrittenAtom = atom(EMPTY);
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    overwrittenAtom.debugPrivate = true;
  }
  const anAtom = atom(
    (get, options) => {
      const overwritten = get(overwrittenAtom);
      if (overwritten !== EMPTY) {
        return overwritten;
      }
      return getDefault(get, options);
    },
    (get, set, update) => {
      const newValue = typeof update === "function" ? update(get(anAtom)) : update;
      set(overwrittenAtom, newValue === RESET ? EMPTY : newValue);
    }
  );
  return anAtom;
}

const isPromiseLike$3 = (x) => typeof (x == null ? void 0 : x.then) === "function";
function withStorageValidator(validator) {
  return (unknownStorage) => {
    const storage = {
      ...unknownStorage,
      getItem: (key, initialValue) => {
        const validate = (value2) => {
          if (!validator(value2)) {
            return initialValue;
          }
          return value2;
        };
        const value = unknownStorage.getItem(key, initialValue);
        if (isPromiseLike$3(value)) {
          return value.then(validate);
        }
        return validate(value);
      }
    };
    return storage;
  };
}
function createJSONStorage(getStringStorage = () => {
  try {
    return window.localStorage;
  } catch (e) {
    if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
      if (typeof window !== "undefined") {
        console.warn(e);
      }
    }
    return void 0;
  }
}, options) {
  var _a;
  let lastStr;
  let lastValue;
  const storage = {
    getItem: (key, initialValue) => {
      var _a2, _b;
      const parse = (str2) => {
        str2 = str2 || "";
        if (lastStr !== str2) {
          try {
            lastValue = JSON.parse(str2, options == null ? void 0 : options.reviver);
          } catch (e) {
            return initialValue;
          }
          lastStr = str2;
        }
        return lastValue;
      };
      const str = (_b = (_a2 = getStringStorage()) == null ? void 0 : _a2.getItem(key)) != null ? _b : null;
      if (isPromiseLike$3(str)) {
        return str.then(parse);
      }
      return parse(str);
    },
    setItem: (key, newValue) => {
      var _a2;
      return (_a2 = getStringStorage()) == null ? void 0 : _a2.setItem(
        key,
        JSON.stringify(newValue, options == null ? void 0 : options.replacer)
      );
    },
    removeItem: (key) => {
      var _a2;
      return (_a2 = getStringStorage()) == null ? void 0 : _a2.removeItem(key);
    }
  };
  const createHandleSubscribe = (subscriber2) => (key, callback, initialValue) => subscriber2(key, (v) => {
    let newValue;
    try {
      newValue = JSON.parse(v || "");
    } catch (e) {
      newValue = initialValue;
    }
    callback(newValue);
  });
  let subscriber;
  try {
    subscriber = (_a = getStringStorage()) == null ? void 0 : _a.subscribe;
  } catch (e) {
  }
  if (!subscriber && typeof window !== "undefined" && typeof window.addEventListener === "function" && window.Storage) {
    subscriber = (key, callback) => {
      if (!(getStringStorage() instanceof window.Storage)) {
        return () => {
        };
      }
      const storageEventCallback = (e) => {
        if (e.storageArea === getStringStorage() && e.key === key) {
          callback(e.newValue);
        }
      };
      window.addEventListener("storage", storageEventCallback);
      return () => {
        window.removeEventListener("storage", storageEventCallback);
      };
    };
  }
  if (subscriber) {
    storage.subscribe = createHandleSubscribe(subscriber);
  }
  return storage;
}
const defaultStorage = createJSONStorage();
function atomWithStorage(key, initialValue, storage = defaultStorage, options) {
  const getOnInit = options == null ? void 0 : options.getOnInit;
  const baseAtom = atom(
    getOnInit ? storage.getItem(key, initialValue) : initialValue
  );
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    baseAtom.debugPrivate = true;
  }
  baseAtom.onMount = (setAtom) => {
    setAtom(storage.getItem(key, initialValue));
    let unsub;
    if (storage.subscribe) {
      unsub = storage.subscribe(key, setAtom, initialValue);
    }
    return unsub;
  };
  const anAtom = atom(
    (get) => get(baseAtom),
    (get, set, update) => {
      const nextValue = typeof update === "function" ? update(get(baseAtom)) : update;
      if (nextValue === RESET) {
        set(baseAtom, initialValue);
        return storage.removeItem(key);
      }
      if (isPromiseLike$3(nextValue)) {
        return nextValue.then((resolvedValue) => {
          set(baseAtom, resolvedValue);
          return storage.setItem(key, resolvedValue);
        });
      }
      set(baseAtom, nextValue);
      return storage.setItem(key, nextValue);
    }
  );
  return anAtom;
}

const isPromiseLike$2 = (x) => typeof (x == null ? void 0 : x.then) === "function";
function atomWithObservable(getObservable, options) {
  const returnResultData = (result) => {
    if ("e" in result) {
      throw result.e;
    }
    return result.d;
  };
  const observableResultAtom = atom((get) => {
    var _a;
    let observable = getObservable(get);
    const itself = (_a = observable[Symbol.observable]) == null ? void 0 : _a.call(observable);
    if (itself) {
      observable = itself;
    }
    let resolve;
    const makePending = () => new Promise((r) => {
      resolve = r;
    });
    const initialResult = options && "initialValue" in options ? {
      d: typeof options.initialValue === "function" ? options.initialValue() : options.initialValue
    } : makePending();
    let setResult;
    let lastResult;
    const listener = (result) => {
      lastResult = result;
      resolve == null ? void 0 : resolve(result);
      setResult == null ? void 0 : setResult(result);
    };
    let subscription;
    let timer;
    const isNotMounted = () => !setResult;
    const unsubscribe = () => {
      if (subscription) {
        subscription.unsubscribe();
        subscription = void 0;
      }
    };
    const start = () => {
      if (subscription) {
        clearTimeout(timer);
        subscription.unsubscribe();
      }
      subscription = observable.subscribe({
        next: (d) => listener({ d }),
        error: (e) => listener({ e }),
        complete: () => {
        }
      });
      if (isNotMounted() && (options == null ? void 0 : options.unstable_timeout)) {
        timer = setTimeout(unsubscribe, options.unstable_timeout);
      }
    };
    start();
    const resultAtom = atom(lastResult || initialResult);
    if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
      resultAtom.debugPrivate = true;
    }
    resultAtom.onMount = (update) => {
      setResult = update;
      if (lastResult) {
        update(lastResult);
      }
      if (subscription) {
        clearTimeout(timer);
      } else {
        start();
      }
      return () => {
        setResult = void 0;
        if (options == null ? void 0 : options.unstable_timeout) {
          timer = setTimeout(unsubscribe, options.unstable_timeout);
        } else {
          unsubscribe();
        }
      };
    };
    return [resultAtom, observable, makePending, start, isNotMounted];
  });
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    observableResultAtom.debugPrivate = true;
  }
  const observableAtom = atom(
    (get) => {
      const [resultAtom] = get(observableResultAtom);
      const result = get(resultAtom);
      if (isPromiseLike$2(result)) {
        return result.then(returnResultData);
      }
      return returnResultData(result);
    },
    (get, set, data) => {
      const [resultAtom, observable, makePending, start, isNotMounted] = get(observableResultAtom);
      if ("next" in observable) {
        if (isNotMounted()) {
          set(resultAtom, makePending());
          start();
        }
        observable.next(data);
      } else {
        throw new Error("observable is not subject");
      }
    }
  );
  return observableAtom;
}

const cache1$1 = /* @__PURE__ */ new WeakMap();
const memo1 = (create, dep1) => (cache1$1.has(dep1) ? cache1$1 : cache1$1.set(dep1, create())).get(dep1);
const isPromiseLike$1 = (p) => typeof (p == null ? void 0 : p.then) === "function";
const LOADING = { state: "loading" };
function loadable(anAtom) {
  return memo1(() => {
    const loadableCache = /* @__PURE__ */ new WeakMap();
    const refreshAtom = atom(0);
    const triggerRefreshAtom = atom([]);
    triggerRefreshAtom.INTERNAL_onInit = (store) => {
      store.set(triggerRefreshAtom, [
        () => store.set(refreshAtom, (c) => c + 1)
      ]);
    };
    if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
      refreshAtom.debugPrivate = true;
      triggerRefreshAtom.debugPrivate = true;
    }
    const derivedAtom = atom((get) => {
      get(refreshAtom);
      let value;
      try {
        value = get(anAtom);
      } catch (error) {
        return { state: "hasError", error };
      }
      if (!isPromiseLike$1(value)) {
        return { state: "hasData", data: value };
      }
      const promise = value;
      const cached1 = loadableCache.get(promise);
      if (cached1) {
        return cached1;
      }
      promise.then(
        (data) => {
          loadableCache.set(promise, { state: "hasData", data });
          const [triggerRefresh] = get(triggerRefreshAtom);
          triggerRefresh();
        },
        (error) => {
          loadableCache.set(promise, { state: "hasError", error });
          const [triggerRefresh] = get(triggerRefreshAtom);
          triggerRefresh();
        }
      );
      const cached2 = loadableCache.get(promise);
      if (cached2) {
        return cached2;
      }
      loadableCache.set(promise, LOADING);
      return LOADING;
    });
    if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
      derivedAtom.debugPrivate = true;
    }
    return atom((get) => get(derivedAtom));
  }, anAtom);
}

const getCached = (c, m, k) => (m.has(k) ? m : m.set(k, c())).get(k);
const cache1 = /* @__PURE__ */ new WeakMap();
const memo2 = (create, dep1, dep2) => {
  const cache2 = getCached(() => /* @__PURE__ */ new WeakMap(), cache1, dep1);
  return getCached(create, cache2, dep2);
};
const isPromiseLike = (p) => typeof (p == null ? void 0 : p.then) === "function";
const defaultFallback = () => void 0;
function unwrap(anAtom, fallback = defaultFallback) {
  return memo2(
    () => {
      const promiseErrorCache = /* @__PURE__ */ new WeakMap();
      const promiseResultCache = /* @__PURE__ */ new WeakMap();
      const refreshAtom = atom(0);
      const triggerRefreshAtom = atom([]);
      triggerRefreshAtom.INTERNAL_onInit = (store) => {
        store.set(triggerRefreshAtom, [
          () => store.set(refreshAtom, (c) => c + 1)
        ]);
      };
      if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
        refreshAtom.debugPrivate = true;
        triggerRefreshAtom.debugPrivate = true;
      }
      const promiseAndValueAtom = atom((get) => {
        get(refreshAtom);
        let prev;
        try {
          prev = get(promiseAndValueAtom);
        } catch (e) {
        }
        const promise = get(anAtom);
        if (!isPromiseLike(promise)) {
          return { v: promise };
        }
        if (promise !== (prev == null ? void 0 : prev.p)) {
          promise.then(
            (v) => {
              promiseResultCache.set(promise, v);
              const [triggerRefresh] = get(triggerRefreshAtom);
              triggerRefresh();
            },
            (e) => {
              promiseErrorCache.set(promise, e);
              const [triggerRefresh] = get(triggerRefreshAtom);
              triggerRefresh();
            }
          );
        }
        if (promiseErrorCache.has(promise)) {
          throw promiseErrorCache.get(promise);
        }
        if (promiseResultCache.has(promise)) {
          return {
            p: promise,
            v: promiseResultCache.get(promise)
          };
        }
        if (prev && "v" in prev) {
          return { p: promise, f: fallback(prev.v), v: prev.v };
        }
        return { p: promise, f: fallback() };
      });
      promiseAndValueAtom.init = void 0;
      if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
        promiseAndValueAtom.debugPrivate = true;
      }
      return atom(
        (get) => {
          const state = get(promiseAndValueAtom);
          if ("f" in state) {
            return state.f;
          }
          return state.v;
        },
        (_get, set, ...args) => set(anAtom, ...args)
      );
    },
    anAtom,
    fallback
  );
}

function atomWithRefresh(read, write) {
  const refreshAtom = atom(0);
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    refreshAtom.debugPrivate = true;
  }
  return atom(
    (get, options) => {
      get(refreshAtom);
      return read(get, options);
    },
    (get, set, ...args) => {
      if (args.length === 0) {
        set(refreshAtom, (c) => c + 1);
      } else if (write) {
        return write(get, set, ...args);
      } else if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
        throw new Error("refresh must be called without arguments");
      }
    }
  );
}

function atomWithLazy(makeInitial) {
  const a = atom(void 0);
  delete a.init;
  Object.defineProperty(a, "init", {
    get() {
      return makeInitial();
    }
  });
  return a;
}

export { RESET, atomFamily, atomWithDefault, atomWithLazy, atomWithObservable, atomWithReducer, atomWithRefresh, atomWithReset, atomWithStorage, createJSONStorage, freezeAtom, freezeAtomCreator, loadable, selectAtom, splitAtom, withStorageValidator as unstable_withStorageValidator, unwrap };
