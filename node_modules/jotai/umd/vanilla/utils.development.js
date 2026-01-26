(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports, require('jotai/vanilla')) :
  typeof define === 'function' && define.amd ? define(['exports', 'jotai/vanilla'], factory) :
  (global = typeof globalThis !== 'undefined' ? globalThis : global || self, factory(global.jotaiVanillaUtils = {}, global.jotaiVanilla));
})(this, (function (exports, vanilla) { 'use strict';

  var RESET = Symbol('RESET' );

  function atomWithReset(initialValue) {
    var anAtom = vanilla.atom(initialValue, function (get, set, update) {
      var nextValue = typeof update === 'function' ? update(get(anAtom)) : update;
      set(anAtom, nextValue === RESET ? initialValue : nextValue);
    });
    return anAtom;
  }

  function atomWithReducer(initialValue, reducer) {
    return vanilla.atom(initialValue, function (get, set, action) {
      set(this, reducer(get(this), action));
    });
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
  function _extends() {
    return _extends = Object.assign ? Object.assign.bind() : function (n) {
      for (var e = 1; e < arguments.length; e++) {
        var t = arguments[e];
        for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]);
      }
      return n;
    }, _extends.apply(null, arguments);
  }
  function _unsupportedIterableToArray(r, a) {
    if (r) {
      if ("string" == typeof r) return _arrayLikeToArray(r, a);
      var t = {}.toString.call(r).slice(8, -1);
      return "Object" === t && r.constructor && (t = r.constructor.name), "Map" === t || "Set" === t ? Array.from(r) : "Arguments" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0;
    }
  }

  var didWarnDeprecation = false;
  function atomFamily(initializeAtom, areEqual) {
    if (!didWarnDeprecation) {
      console.warn('[DEPRECATED] atomFamily is deprecated and will be removed in v3. ' + 'Please use the `jotai-family` package instead: https://github.com/jotaijs/jotai-family');
      didWarnDeprecation = true;
    }
    var shouldRemove = null;
    var atoms = new Map();
    var listeners = new Set();
    var _createAtom = function createAtom(param) {
      var item;
      if (areEqual === undefined) {
        item = atoms.get(param);
      } else {
        for (var _iterator = _createForOfIteratorHelperLoose(atoms), _step; !(_step = _iterator()).done;) {
          var _step$value = _step.value,
            key = _step$value[0],
            value = _step$value[1];
          if (areEqual(key, param)) {
            item = value;
            break;
          }
        }
      }
      if (item !== undefined) {
        if (shouldRemove != null && shouldRemove(item[1], param)) {
          _createAtom.remove(param);
        } else {
          return item[0];
        }
      }
      var newAtom = initializeAtom(param);
      atoms.set(param, [newAtom, Date.now()]);
      notifyListeners('CREATE', param, newAtom);
      return newAtom;
    };
    var notifyListeners = function notifyListeners(type, param, atom) {
      for (var _iterator2 = _createForOfIteratorHelperLoose(listeners), _step2; !(_step2 = _iterator2()).done;) {
        var listener = _step2.value;
        listener({
          type: type,
          param: param,
          atom: atom
        });
      }
    };
    _createAtom.unstable_listen = function (callback) {
      listeners.add(callback);
      return function () {
        listeners.delete(callback);
      };
    };
    _createAtom.getParams = function () {
      return atoms.keys();
    };
    _createAtom.remove = function (param) {
      if (areEqual === undefined) {
        if (!atoms.has(param)) return;
        var _ref = atoms.get(param),
          atom = _ref[0];
        atoms.delete(param);
        notifyListeners('REMOVE', param, atom);
      } else {
        for (var _iterator3 = _createForOfIteratorHelperLoose(atoms), _step3; !(_step3 = _iterator3()).done;) {
          var _step3$value = _step3.value,
            key = _step3$value[0],
            _step3$value$ = _step3$value[1],
            _atom = _step3$value$[0];
          if (areEqual(key, param)) {
            atoms.delete(key);
            notifyListeners('REMOVE', key, _atom);
            break;
          }
        }
      }
    };
    _createAtom.setShouldRemove = function (fn) {
      shouldRemove = fn;
      if (!shouldRemove) return;
      for (var _iterator4 = _createForOfIteratorHelperLoose(atoms), _step4; !(_step4 = _iterator4()).done;) {
        var _step4$value = _step4.value,
          key = _step4$value[0],
          _step4$value$ = _step4$value[1],
          atom = _step4$value$[0],
          _createdAt = _step4$value$[1];
        if (shouldRemove(_createdAt, key)) {
          atoms.delete(key);
          notifyListeners('REMOVE', key, atom);
        }
      }
    };
    return _createAtom;
  }

  var getCached$2 = function getCached(c, m, k) {
    return (m.has(k) ? m : m.set(k, c())).get(k);
  };
  var cache1$3 = new WeakMap();
  var memo3 = function memo3(create, dep1, dep2, dep3) {
    var cache2 = getCached$2(function () {
      return new WeakMap();
    }, cache1$3, dep1);
    var cache3 = getCached$2(function () {
      return new WeakMap();
    }, cache2, dep2);
    return getCached$2(create, cache3, dep3);
  };
  function selectAtom(anAtom, selector, equalityFn) {
    if (equalityFn === void 0) {
      equalityFn = Object.is;
    }
    return memo3(function () {
      var EMPTY = Symbol();
      var selectValue = function selectValue(_ref) {
        var value = _ref[0],
          prevSlice = _ref[1];
        if (prevSlice === EMPTY) {
          return selector(value);
        }
        var slice = selector(value, prevSlice);
        return equalityFn(prevSlice, slice) ? prevSlice : slice;
      };
      var derivedAtom = vanilla.atom(function (get) {
        var prev = get(derivedAtom);
        var value = get(anAtom);
        return selectValue([value, prev]);
      });
      derivedAtom.init = EMPTY;
      return derivedAtom;
    }, anAtom, selector, equalityFn);
  }

  var frozenAtoms = new WeakSet();
  var _deepFreeze = function deepFreeze(value) {
    if (typeof value !== 'object' || value === null) {
      return value;
    }
    Object.freeze(value);
    var propNames = Object.getOwnPropertyNames(value);
    for (var _iterator = _createForOfIteratorHelperLoose(propNames), _step; !(_step = _iterator()).done;) {
      var name = _step.value;
      _deepFreeze(value[name]);
    }
    return value;
  };
  function freezeAtom(anAtom) {
    if (frozenAtoms.has(anAtom)) {
      return anAtom;
    }
    frozenAtoms.add(anAtom);
    var origRead = anAtom.read;
    anAtom.read = function (get, options) {
      return _deepFreeze(origRead.call(this, get, options));
    };
    if ('write' in anAtom) {
      var origWrite = anAtom.write;
      anAtom.write = function (get, set) {
        for (var _len = arguments.length, args = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
          args[_key - 2] = arguments[_key];
        }
        return origWrite.call.apply(origWrite, [this, get, function () {
          for (var _len2 = arguments.length, setArgs = new Array(_len2), _key2 = 0; _key2 < _len2; _key2++) {
            setArgs[_key2] = arguments[_key2];
          }
          if (setArgs[0] === anAtom) {
            setArgs[1] = _deepFreeze(setArgs[1]);
          }
          return set.apply(void 0, setArgs);
        }].concat(args));
      };
    }
    return anAtom;
  }
  function freezeAtomCreator(createAtom) {
    {
      console.warn('[DEPRECATED] freezeAtomCreator is deprecated, define it on users end');
    }
    return function () {
      return freezeAtom(createAtom.apply(void 0, arguments));
    };
  }

  var getCached$1 = function getCached(c, m, k) {
    return (m.has(k) ? m : m.set(k, c())).get(k);
  };
  var cache1$2 = new WeakMap();
  var memo2$1 = function memo2(create, dep1, dep2) {
    var cache2 = getCached$1(function () {
      return new WeakMap();
    }, cache1$2, dep1);
    return getCached$1(create, cache2, dep2);
  };
  var cacheKeyForEmptyKeyExtractor = {};
  var isWritable = function isWritable(atom) {
    return !!atom.write;
  };
  var isFunction = function isFunction(x) {
    return typeof x === 'function';
  };
  function splitAtom(arrAtom, keyExtractor) {
    return memo2$1(function () {
      var mappingCache = new WeakMap();
      var _getMapping = function getMapping(arr, prev) {
        var mapping = mappingCache.get(arr);
        if (mapping) {
          return mapping;
        }
        var prevMapping = prev && mappingCache.get(prev);
        var atomList = [];
        var keyList = [];
        arr.forEach(function (item, index) {
          var key = keyExtractor ? keyExtractor(item) : index;
          keyList[index] = key;
          var cachedAtom = prevMapping && prevMapping.atomList[prevMapping.keyList.indexOf(key)];
          if (cachedAtom) {
            atomList[index] = cachedAtom;
            return;
          }
          var read = function read(get) {
            var prev = get(mappingAtom);
            var currArr = get(arrAtom);
            var mapping = _getMapping(currArr, prev == null ? void 0 : prev.arr);
            var index = mapping.keyList.indexOf(key);
            if (index < 0 || index >= currArr.length) {
              var prevItem = arr[_getMapping(arr).keyList.indexOf(key)];
              if (prevItem) {
                return prevItem;
              }
              throw new Error('splitAtom: index out of bounds for read');
            }
            return currArr[index];
          };
          var write = function write(get, set, update) {
            var prev = get(mappingAtom);
            var arr = get(arrAtom);
            var mapping = _getMapping(arr, prev == null ? void 0 : prev.arr);
            var index = mapping.keyList.indexOf(key);
            if (index < 0 || index >= arr.length) {
              throw new Error('splitAtom: index out of bounds for write');
            }
            var nextItem = isFunction(update) ? update(arr[index]) : update;
            if (!Object.is(arr[index], nextItem)) {
              set(arrAtom, [].concat(arr.slice(0, index), [nextItem], arr.slice(index + 1)));
            }
          };
          atomList[index] = isWritable(arrAtom) ? vanilla.atom(read, write) : vanilla.atom(read);
        });
        if (prevMapping && prevMapping.keyList.length === keyList.length && prevMapping.keyList.every(function (x, i) {
          return x === keyList[i];
        })) {
          mapping = prevMapping;
        } else {
          mapping = {
            arr: arr,
            atomList: atomList,
            keyList: keyList
          };
        }
        mappingCache.set(arr, mapping);
        return mapping;
      };
      var mappingAtom = vanilla.atom(function (get) {
        var prev = get(mappingAtom);
        var arr = get(arrAtom);
        var mapping = _getMapping(arr, prev == null ? void 0 : prev.arr);
        return mapping;
      });
      {
        mappingAtom.debugPrivate = true;
      }
      mappingAtom.init = undefined;
      var splittedAtom = isWritable(arrAtom) ? vanilla.atom(function (get) {
        return get(mappingAtom).atomList;
      }, function (get, set, action) {
        switch (action.type) {
          case 'remove':
            {
              var index = get(splittedAtom).indexOf(action.atom);
              if (index >= 0) {
                var arr = get(arrAtom);
                set(arrAtom, [].concat(arr.slice(0, index), arr.slice(index + 1)));
              }
              break;
            }
          case 'insert':
            {
              var _index = action.before ? get(splittedAtom).indexOf(action.before) : get(splittedAtom).length;
              if (_index >= 0) {
                var _arr = get(arrAtom);
                set(arrAtom, [].concat(_arr.slice(0, _index), [action.value], _arr.slice(_index)));
              }
              break;
            }
          case 'move':
            {
              var index1 = get(splittedAtom).indexOf(action.atom);
              var index2 = action.before ? get(splittedAtom).indexOf(action.before) : get(splittedAtom).length;
              if (index1 >= 0 && index2 >= 0) {
                var _arr2 = get(arrAtom);
                if (index1 < index2) {
                  set(arrAtom, [].concat(_arr2.slice(0, index1), _arr2.slice(index1 + 1, index2), [_arr2[index1]], _arr2.slice(index2)));
                } else {
                  set(arrAtom, [].concat(_arr2.slice(0, index2), [_arr2[index1]], _arr2.slice(index2, index1), _arr2.slice(index1 + 1)));
                }
              }
              break;
            }
        }
      }) : vanilla.atom(function (get) {
        return get(mappingAtom).atomList;
      });
      return splittedAtom;
    }, arrAtom, keyExtractor || cacheKeyForEmptyKeyExtractor);
  }

  function atomWithDefault(getDefault) {
    var EMPTY = Symbol();
    var overwrittenAtom = vanilla.atom(EMPTY);
    {
      overwrittenAtom.debugPrivate = true;
    }
    var anAtom = vanilla.atom(function (get, options) {
      var overwritten = get(overwrittenAtom);
      if (overwritten !== EMPTY) {
        return overwritten;
      }
      return getDefault(get, options);
    }, function (get, set, update) {
      var newValue = typeof update === 'function' ? update(get(anAtom)) : update;
      set(overwrittenAtom, newValue === RESET ? EMPTY : newValue);
    });
    return anAtom;
  }

  var isPromiseLike$3 = function isPromiseLike(x) {
    return typeof (x == null ? void 0 : x.then) === 'function';
  };
  function withStorageValidator(validator) {
    return function (unknownStorage) {
      var storage = _extends({}, unknownStorage, {
        getItem: function getItem(key, initialValue) {
          var validate = function validate(value) {
            if (!validator(value)) {
              return initialValue;
            }
            return value;
          };
          var value = unknownStorage.getItem(key, initialValue);
          if (isPromiseLike$3(value)) {
            return value.then(validate);
          }
          return validate(value);
        }
      });
      return storage;
    };
  }
  function createJSONStorage(getStringStorage, options) {
    if (getStringStorage === void 0) {
      getStringStorage = function getStringStorage() {
        try {
          return window.localStorage;
        } catch (e) {
          {
            if (typeof window !== 'undefined') {
              console.warn(e);
            }
          }
          return undefined;
        }
      };
    }
    var lastStr;
    var lastValue;
    var storage = {
      getItem: function getItem(key, initialValue) {
        var _getStringStorage$get, _getStringStorage;
        var parse = function parse(str) {
          str = str || '';
          if (lastStr !== str) {
            try {
              lastValue = JSON.parse(str, options == null ? void 0 : options.reviver);
            } catch (_unused) {
              return initialValue;
            }
            lastStr = str;
          }
          return lastValue;
        };
        var str = (_getStringStorage$get = (_getStringStorage = getStringStorage()) == null ? void 0 : _getStringStorage.getItem(key)) != null ? _getStringStorage$get : null;
        if (isPromiseLike$3(str)) {
          return str.then(parse);
        }
        return parse(str);
      },
      setItem: function setItem(key, newValue) {
        var _getStringStorage2;
        return (_getStringStorage2 = getStringStorage()) == null ? void 0 : _getStringStorage2.setItem(key, JSON.stringify(newValue, options == null ? void 0 : options.replacer));
      },
      removeItem: function removeItem(key) {
        var _getStringStorage3;
        return (_getStringStorage3 = getStringStorage()) == null ? void 0 : _getStringStorage3.removeItem(key);
      }
    };
    var createHandleSubscribe = function createHandleSubscribe(subscriber) {
      return function (key, callback, initialValue) {
        return subscriber(key, function (v) {
          var newValue;
          try {
            newValue = JSON.parse(v || '');
          } catch (_unused2) {
            newValue = initialValue;
          }
          callback(newValue);
        });
      };
    };
    var subscriber;
    try {
      var _getStringStorage4;
      subscriber = (_getStringStorage4 = getStringStorage()) == null ? void 0 : _getStringStorage4.subscribe;
    } catch (_unused3) {}
    if (!subscriber && typeof window !== 'undefined' && typeof window.addEventListener === 'function' && window.Storage) {
      subscriber = function subscriber(key, callback) {
        if (!(getStringStorage() instanceof window.Storage)) {
          return function () {};
        }
        var storageEventCallback = function storageEventCallback(e) {
          if (e.storageArea === getStringStorage() && e.key === key) {
            callback(e.newValue);
          }
        };
        window.addEventListener('storage', storageEventCallback);
        return function () {
          window.removeEventListener('storage', storageEventCallback);
        };
      };
    }
    if (subscriber) {
      storage.subscribe = createHandleSubscribe(subscriber);
    }
    return storage;
  }
  var defaultStorage = createJSONStorage();
  function atomWithStorage(key, initialValue, storage, options) {
    if (storage === void 0) {
      storage = defaultStorage;
    }
    var getOnInit = options == null ? void 0 : options.getOnInit;
    var baseAtom = vanilla.atom(getOnInit ? storage.getItem(key, initialValue) : initialValue);
    {
      baseAtom.debugPrivate = true;
    }
    baseAtom.onMount = function (setAtom) {
      setAtom(storage.getItem(key, initialValue));
      var unsub;
      if (storage.subscribe) {
        unsub = storage.subscribe(key, setAtom, initialValue);
      }
      return unsub;
    };
    var anAtom = vanilla.atom(function (get) {
      return get(baseAtom);
    }, function (get, set, update) {
      var nextValue = typeof update === 'function' ? update(get(baseAtom)) : update;
      if (nextValue === RESET) {
        set(baseAtom, initialValue);
        return storage.removeItem(key);
      }
      if (isPromiseLike$3(nextValue)) {
        return nextValue.then(function (resolvedValue) {
          set(baseAtom, resolvedValue);
          return storage.setItem(key, resolvedValue);
        });
      }
      set(baseAtom, nextValue);
      return storage.setItem(key, nextValue);
    });
    return anAtom;
  }

  var isPromiseLike$2 = function isPromiseLike(x) {
    return typeof (x == null ? void 0 : x.then) === 'function';
  };
  function atomWithObservable(getObservable, options) {
    var returnResultData = function returnResultData(result) {
      if ('e' in result) {
        throw result.e;
      }
      return result.d;
    };
    var observableResultAtom = vanilla.atom(function (get) {
      var _Symbol$observable, _ref;
      var observable = getObservable(get);
      var itself = (_Symbol$observable = (_ref = observable)[Symbol.observable]) == null ? void 0 : _Symbol$observable.call(_ref);
      if (itself) {
        observable = itself;
      }
      var resolve;
      var makePending = function makePending() {
        return new Promise(function (r) {
          resolve = r;
        });
      };
      var initialResult = options && 'initialValue' in options ? {
        d: typeof options.initialValue === 'function' ? options.initialValue() : options.initialValue
      } : makePending();
      var setResult;
      var lastResult;
      var listener = function listener(result) {
        lastResult = result;
        resolve == null || resolve(result);
        setResult == null || setResult(result);
      };
      var subscription;
      var timer;
      var isNotMounted = function isNotMounted() {
        return !setResult;
      };
      var unsubscribe = function unsubscribe() {
        if (subscription) {
          subscription.unsubscribe();
          subscription = undefined;
        }
      };
      var start = function start() {
        if (subscription) {
          clearTimeout(timer);
          subscription.unsubscribe();
        }
        subscription = observable.subscribe({
          next: function next(d) {
            return listener({
              d: d
            });
          },
          error: function error(e) {
            return listener({
              e: e
            });
          },
          complete: function complete() {}
        });
        if (isNotMounted() && options != null && options.unstable_timeout) {
          timer = setTimeout(unsubscribe, options.unstable_timeout);
        }
      };
      start();
      var resultAtom = vanilla.atom(lastResult || initialResult);
      {
        resultAtom.debugPrivate = true;
      }
      resultAtom.onMount = function (update) {
        setResult = update;
        if (lastResult) {
          update(lastResult);
        }
        if (subscription) {
          clearTimeout(timer);
        } else {
          start();
        }
        return function () {
          setResult = undefined;
          if (options != null && options.unstable_timeout) {
            timer = setTimeout(unsubscribe, options.unstable_timeout);
          } else {
            unsubscribe();
          }
        };
      };
      return [resultAtom, observable, makePending, start, isNotMounted];
    });
    {
      observableResultAtom.debugPrivate = true;
    }
    var observableAtom = vanilla.atom(function (get) {
      var _get = get(observableResultAtom),
        resultAtom = _get[0];
      var result = get(resultAtom);
      if (isPromiseLike$2(result)) {
        return result.then(returnResultData);
      }
      return returnResultData(result);
    }, function (get, set, data) {
      var _get2 = get(observableResultAtom),
        resultAtom = _get2[0],
        observable = _get2[1],
        makePending = _get2[2],
        start = _get2[3],
        isNotMounted = _get2[4];
      if ('next' in observable) {
        if (isNotMounted()) {
          set(resultAtom, makePending());
          start();
        }
        observable.next(data);
      } else {
        throw new Error('observable is not subject');
      }
    });
    return observableAtom;
  }

  var cache1$1 = new WeakMap();
  var memo1 = function memo1(create, dep1) {
    return (cache1$1.has(dep1) ? cache1$1 : cache1$1.set(dep1, create())).get(dep1);
  };
  var isPromiseLike$1 = function isPromiseLike(p) {
    return typeof (p == null ? void 0 : p.then) === 'function';
  };
  var LOADING = {
    state: 'loading'
  };
  function loadable(anAtom) {
    return memo1(function () {
      var loadableCache = new WeakMap();
      var refreshAtom = vanilla.atom(0);
      var triggerRefreshAtom = vanilla.atom([]);
      triggerRefreshAtom.INTERNAL_onInit = function (store) {
        store.set(triggerRefreshAtom, [function () {
          return store.set(refreshAtom, function (c) {
            return c + 1;
          });
        }]);
      };
      {
        refreshAtom.debugPrivate = true;
        triggerRefreshAtom.debugPrivate = true;
      }
      var derivedAtom = vanilla.atom(function (get) {
        get(refreshAtom);
        var value;
        try {
          value = get(anAtom);
        } catch (error) {
          return {
            state: 'hasError',
            error: error
          };
        }
        if (!isPromiseLike$1(value)) {
          return {
            state: 'hasData',
            data: value
          };
        }
        var promise = value;
        var cached1 = loadableCache.get(promise);
        if (cached1) {
          return cached1;
        }
        promise.then(function (data) {
          loadableCache.set(promise, {
            state: 'hasData',
            data: data
          });
          var _get = get(triggerRefreshAtom),
            triggerRefresh = _get[0];
          triggerRefresh();
        }, function (error) {
          loadableCache.set(promise, {
            state: 'hasError',
            error: error
          });
          var _get2 = get(triggerRefreshAtom),
            triggerRefresh = _get2[0];
          triggerRefresh();
        });
        var cached2 = loadableCache.get(promise);
        if (cached2) {
          return cached2;
        }
        loadableCache.set(promise, LOADING);
        return LOADING;
      });
      {
        derivedAtom.debugPrivate = true;
      }
      return vanilla.atom(function (get) {
        return get(derivedAtom);
      });
    }, anAtom);
  }

  var getCached = function getCached(c, m, k) {
    return (m.has(k) ? m : m.set(k, c())).get(k);
  };
  var cache1 = new WeakMap();
  var memo2 = function memo2(create, dep1, dep2) {
    var cache2 = getCached(function () {
      return new WeakMap();
    }, cache1, dep1);
    return getCached(create, cache2, dep2);
  };
  var isPromiseLike = function isPromiseLike(p) {
    return typeof (p == null ? void 0 : p.then) === 'function';
  };
  var defaultFallback = function defaultFallback() {
    return undefined;
  };
  function unwrap(anAtom, fallback) {
    if (fallback === void 0) {
      fallback = defaultFallback;
    }
    return memo2(function () {
      var promiseErrorCache = new WeakMap();
      var promiseResultCache = new WeakMap();
      var refreshAtom = vanilla.atom(0);
      var triggerRefreshAtom = vanilla.atom([]);
      triggerRefreshAtom.INTERNAL_onInit = function (store) {
        store.set(triggerRefreshAtom, [function () {
          return store.set(refreshAtom, function (c) {
            return c + 1;
          });
        }]);
      };
      {
        refreshAtom.debugPrivate = true;
        triggerRefreshAtom.debugPrivate = true;
      }
      var promiseAndValueAtom = vanilla.atom(function (get) {
        var _prev;
        get(refreshAtom);
        var prev;
        try {
          prev = get(promiseAndValueAtom);
        } catch (_unused) {}
        var promise = get(anAtom);
        if (!isPromiseLike(promise)) {
          return {
            v: promise
          };
        }
        if (promise !== ((_prev = prev) == null ? void 0 : _prev.p)) {
          promise.then(function (v) {
            promiseResultCache.set(promise, v);
            var _get2 = get(triggerRefreshAtom),
              triggerRefresh = _get2[0];
            triggerRefresh();
          }, function (e) {
            promiseErrorCache.set(promise, e);
            var _get3 = get(triggerRefreshAtom),
              triggerRefresh = _get3[0];
            triggerRefresh();
          });
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
        if (prev && 'v' in prev) {
          return {
            p: promise,
            f: fallback(prev.v),
            v: prev.v
          };
        }
        return {
          p: promise,
          f: fallback()
        };
      });
      promiseAndValueAtom.init = undefined;
      {
        promiseAndValueAtom.debugPrivate = true;
      }
      return vanilla.atom(function (get) {
        var state = get(promiseAndValueAtom);
        if ('f' in state) {
          return state.f;
        }
        return state.v;
      }, function (_get, set) {
        for (var _len = arguments.length, args = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
          args[_key - 2] = arguments[_key];
        }
        return set.apply(void 0, [anAtom].concat(args));
      });
    }, anAtom, fallback);
  }

  function atomWithRefresh(read, write) {
    var refreshAtom = vanilla.atom(0);
    {
      refreshAtom.debugPrivate = true;
    }
    return vanilla.atom(function (get, options) {
      get(refreshAtom);
      return read(get, options);
    }, function (get, set) {
      for (var _len = arguments.length, args = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
        args[_key - 2] = arguments[_key];
      }
      if (args.length === 0) {
        set(refreshAtom, function (c) {
          return c + 1;
        });
      } else if (write) {
        return write.apply(void 0, [get, set].concat(args));
      } else {
        throw new Error('refresh must be called without arguments');
      }
    });
  }

  function atomWithLazy(makeInitial) {
    var a = vanilla.atom(undefined);
    delete a.init;
    Object.defineProperty(a, 'init', {
      get: function get() {
        return makeInitial();
      }
    });
    return a;
  }

  exports.RESET = RESET;
  exports.atomFamily = atomFamily;
  exports.atomWithDefault = atomWithDefault;
  exports.atomWithLazy = atomWithLazy;
  exports.atomWithObservable = atomWithObservable;
  exports.atomWithReducer = atomWithReducer;
  exports.atomWithRefresh = atomWithRefresh;
  exports.atomWithReset = atomWithReset;
  exports.atomWithStorage = atomWithStorage;
  exports.createJSONStorage = createJSONStorage;
  exports.freezeAtom = freezeAtom;
  exports.freezeAtomCreator = freezeAtomCreator;
  exports.loadable = loadable;
  exports.selectAtom = selectAtom;
  exports.splitAtom = splitAtom;
  exports.unstable_withStorageValidator = withStorageValidator;
  exports.unwrap = unwrap;

}));
