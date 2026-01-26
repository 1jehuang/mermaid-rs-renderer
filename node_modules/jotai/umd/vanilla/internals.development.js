(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports) :
  typeof define === 'function' && define.amd ? define(['exports'], factory) :
  (global = typeof globalThis !== 'undefined' ? globalThis : global || self, factory(global.jotaiVanillaInternals = {}));
})(this, (function (exports) { 'use strict';

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

  function hasInitialValue(atom) {
    return 'init' in atom;
  }
  function isActuallyWritableAtom(atom) {
    return !!atom.write;
  }
  function isAtomStateInitialized(atomState) {
    return 'v' in atomState || 'e' in atomState;
  }
  function returnAtomValue(atomState) {
    if ('e' in atomState) {
      throw atomState.e;
    }
    if (!('v' in atomState)) {
      throw new Error('[Bug] atom state is not initialized');
    }
    return atomState.v;
  }
  var promiseStateMap = new WeakMap();
  function isPendingPromise(value) {
    var _promiseStateMap$get;
    return isPromiseLike(value) && !!((_promiseStateMap$get = promiseStateMap.get(value)) != null && _promiseStateMap$get[0]);
  }
  function abortPromise(promise) {
    var promiseState = promiseStateMap.get(promise);
    if (promiseState != null && promiseState[0]) {
      promiseState[0] = false;
      promiseState[1].forEach(function (fn) {
        return fn();
      });
    }
  }
  function registerAbortHandler(promise, abortHandler) {
    var promiseState = promiseStateMap.get(promise);
    if (!promiseState) {
      promiseState = [true, new Set()];
      promiseStateMap.set(promise, promiseState);
      var settle = function settle() {
        promiseState[0] = false;
      };
      promise.then(settle, settle);
    }
    promiseState[1].add(abortHandler);
  }
  function isPromiseLike(p) {
    return typeof (p == null ? void 0 : p.then) === 'function';
  }
  function addPendingPromiseToDependency(atom, promise, dependencyAtomState) {
    if (!dependencyAtomState.p.has(atom)) {
      dependencyAtomState.p.add(atom);
      var cleanup = function cleanup() {
        return dependencyAtomState.p.delete(atom);
      };
      promise.then(cleanup, cleanup);
    }
  }
  function getMountedOrPendingDependents(atom, atomState, mountedMap) {
    var dependents = new Set();
    for (var _iterator = _createForOfIteratorHelperLoose(((_mountedMap$get = mountedMap.get(atom)) == null ? void 0 : _mountedMap$get.t) || []), _step; !(_step = _iterator()).done;) {
      var _mountedMap$get;
      var a = _step.value;
      dependents.add(a);
    }
    for (var _iterator2 = _createForOfIteratorHelperLoose(atomState.p), _step2; !(_step2 = _iterator2()).done;) {
      var atomWithPendingPromise = _step2.value;
      dependents.add(atomWithPendingPromise);
    }
    return dependents;
  }
  var createStoreHook = function createStoreHook() {
    var callbacks = new Set();
    var notify = function notify() {
      return callbacks.forEach(function (fn) {
        return fn();
      });
    };
    notify.add = function (fn) {
      callbacks.add(fn);
      return function () {
        return callbacks.delete(fn);
      };
    };
    return notify;
  };
  var createStoreHookForAtoms = function createStoreHookForAtoms() {
    var all = {};
    var callbacks = new WeakMap();
    var notify = function notify(atom) {
      var _callbacks$get, _callbacks$get2;
      (_callbacks$get = callbacks.get(all)) == null || _callbacks$get.forEach(function (fn) {
        return fn(atom);
      });
      (_callbacks$get2 = callbacks.get(atom)) == null || _callbacks$get2.forEach(function (fn) {
        return fn();
      });
    };
    notify.add = function (atom, fn) {
      var key = atom || all;
      var fns = callbacks.get(key);
      if (!fns) {
        fns = new Set();
        callbacks.set(key, fns);
      }
      fns.add(fn);
      return function () {
        fns.delete(fn);
        if (!fns.size) {
          callbacks.delete(key);
        }
      };
    };
    return notify;
  };
  function initializeStoreHooks(storeHooks) {
    var _ref, _ref2, _ref3, _ref4, _ref5, _ref6;
    (_ref = storeHooks).i || (_ref.i = createStoreHookForAtoms());
    (_ref2 = storeHooks).r || (_ref2.r = createStoreHookForAtoms());
    (_ref3 = storeHooks).c || (_ref3.c = createStoreHookForAtoms());
    (_ref4 = storeHooks).m || (_ref4.m = createStoreHookForAtoms());
    (_ref5 = storeHooks).u || (_ref5.u = createStoreHookForAtoms());
    (_ref6 = storeHooks).f || (_ref6.f = createStoreHook());
    return storeHooks;
  }
  var BUILDING_BLOCK_atomRead = function BUILDING_BLOCK_atomRead(_store, atom) {
    for (var _len = arguments.length, params = new Array(_len > 2 ? _len - 2 : 0), _key = 2; _key < _len; _key++) {
      params[_key - 2] = arguments[_key];
    }
    return atom.read.apply(atom, params);
  };
  var BUILDING_BLOCK_atomWrite = function BUILDING_BLOCK_atomWrite(_store, atom) {
    for (var _len2 = arguments.length, params = new Array(_len2 > 2 ? _len2 - 2 : 0), _key2 = 2; _key2 < _len2; _key2++) {
      params[_key2 - 2] = arguments[_key2];
    }
    return atom.write.apply(atom, params);
  };
  var BUILDING_BLOCK_atomOnInit = function BUILDING_BLOCK_atomOnInit(store, atom) {
    if (atom.INTERNAL_onInit) {
      return atom.INTERNAL_onInit(store);
    }
    if (atom.unstable_onInit) {
      console.warn('[DEPRECATED] atom.unstable_onInit is renamed to atom.INTERNAL_onInit.');
      return atom.unstable_onInit(store);
    }
  };
  var BUILDING_BLOCK_atomOnMount = function BUILDING_BLOCK_atomOnMount(_store, atom, setAtom) {
    return atom.onMount == null ? void 0 : atom.onMount(setAtom);
  };
  var BUILDING_BLOCK_ensureAtomState = function BUILDING_BLOCK_ensureAtomState(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var atomStateMap = buildingBlocks[0];
    var storeHooks = buildingBlocks[6];
    var atomOnInit = buildingBlocks[9];
    if (!atom) {
      throw new Error('Atom is undefined or null');
    }
    var atomState = atomStateMap.get(atom);
    if (!atomState) {
      atomState = {
        d: new Map(),
        p: new Set(),
        n: 0
      };
      atomStateMap.set(atom, atomState);
      storeHooks.i == null || storeHooks.i(atom);
      atomOnInit == null || atomOnInit(store, atom);
    }
    return atomState;
  };
  var BUILDING_BLOCK_flushCallbacks = function BUILDING_BLOCK_flushCallbacks(store) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var changedAtoms = buildingBlocks[3];
    var mountCallbacks = buildingBlocks[4];
    var unmountCallbacks = buildingBlocks[5];
    var storeHooks = buildingBlocks[6];
    var recomputeInvalidatedAtoms = buildingBlocks[13];
    var errors = [];
    var call = function call(fn) {
      try {
        fn();
      } catch (e) {
        errors.push(e);
      }
    };
    var _loop = function _loop() {
      if (storeHooks.f) {
        call(storeHooks.f);
      }
      var callbacks = new Set();
      var add = callbacks.add.bind(callbacks);
      changedAtoms.forEach(function (atom) {
        var _mountedMap$get2;
        return (_mountedMap$get2 = mountedMap.get(atom)) == null ? void 0 : _mountedMap$get2.l.forEach(add);
      });
      changedAtoms.clear();
      unmountCallbacks.forEach(add);
      unmountCallbacks.clear();
      mountCallbacks.forEach(add);
      mountCallbacks.clear();
      callbacks.forEach(call);
      if (changedAtoms.size) {
        recomputeInvalidatedAtoms(store);
      }
    };
    do {
      _loop();
    } while (changedAtoms.size || unmountCallbacks.size || mountCallbacks.size);
    if (errors.length) {
      throw new AggregateError(errors);
    }
  };
  var BUILDING_BLOCK_recomputeInvalidatedAtoms = function BUILDING_BLOCK_recomputeInvalidatedAtoms(store) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var invalidatedAtoms = buildingBlocks[2];
    var changedAtoms = buildingBlocks[3];
    var ensureAtomState = buildingBlocks[11];
    var readAtomState = buildingBlocks[14];
    var mountDependencies = buildingBlocks[17];
    var topSortedReversed = [];
    var visiting = new WeakSet();
    var visited = new WeakSet();
    var stack = Array.from(changedAtoms);
    while (stack.length) {
      var a = stack[stack.length - 1];
      var aState = ensureAtomState(store, a);
      if (visited.has(a)) {
        stack.pop();
        continue;
      }
      if (visiting.has(a)) {
        if (invalidatedAtoms.get(a) === aState.n) {
          topSortedReversed.push([a, aState]);
        } else if (invalidatedAtoms.has(a)) {
          throw new Error('[Bug] invalidated atom exists');
        }
        visited.add(a);
        stack.pop();
        continue;
      }
      visiting.add(a);
      for (var _iterator3 = _createForOfIteratorHelperLoose(getMountedOrPendingDependents(a, aState, mountedMap)), _step3; !(_step3 = _iterator3()).done;) {
        var d = _step3.value;
        if (!visiting.has(d)) {
          stack.push(d);
        }
      }
    }
    for (var i = topSortedReversed.length - 1; i >= 0; --i) {
      var _ref7 = topSortedReversed[i],
        _a = _ref7[0],
        _aState = _ref7[1];
      var hasChangedDeps = false;
      for (var _iterator4 = _createForOfIteratorHelperLoose(_aState.d.keys()), _step4; !(_step4 = _iterator4()).done;) {
        var dep = _step4.value;
        if (dep !== _a && changedAtoms.has(dep)) {
          hasChangedDeps = true;
          break;
        }
      }
      if (hasChangedDeps) {
        readAtomState(store, _a);
        mountDependencies(store, _a);
      }
      invalidatedAtoms.delete(_a);
    }
  };
  var storeMutationSet = new WeakSet();
  var BUILDING_BLOCK_readAtomState = function BUILDING_BLOCK_readAtomState(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var invalidatedAtoms = buildingBlocks[2];
    var changedAtoms = buildingBlocks[3];
    var storeHooks = buildingBlocks[6];
    var atomRead = buildingBlocks[7];
    var ensureAtomState = buildingBlocks[11];
    var flushCallbacks = buildingBlocks[12];
    var recomputeInvalidatedAtoms = buildingBlocks[13];
    var readAtomState = buildingBlocks[14];
    var writeAtomState = buildingBlocks[16];
    var mountDependencies = buildingBlocks[17];
    var setAtomStateValueOrPromise = buildingBlocks[20];
    var atomState = ensureAtomState(store, atom);
    if (isAtomStateInitialized(atomState)) {
      if (mountedMap.has(atom) && invalidatedAtoms.get(atom) !== atomState.n) {
        return atomState;
      }
      var hasChangedDeps = false;
      for (var _iterator5 = _createForOfIteratorHelperLoose(atomState.d), _step5; !(_step5 = _iterator5()).done;) {
        var _step5$value = _step5.value,
          a = _step5$value[0],
          n = _step5$value[1];
        if (readAtomState(store, a).n !== n) {
          hasChangedDeps = true;
          break;
        }
      }
      if (!hasChangedDeps) {
        return atomState;
      }
    }
    atomState.d.clear();
    var isSync = true;
    function mountDependenciesIfAsync() {
      if (mountedMap.has(atom)) {
        mountDependencies(store, atom);
        recomputeInvalidatedAtoms(store);
        flushCallbacks(store);
      }
    }
    function getter(a) {
      if (a === atom) {
        var _aState2 = ensureAtomState(store, a);
        if (!isAtomStateInitialized(_aState2)) {
          if (hasInitialValue(a)) {
            setAtomStateValueOrPromise(store, a, a.init);
          } else {
            throw new Error('no atom init');
          }
        }
        return returnAtomValue(_aState2);
      }
      var aState = readAtomState(store, a);
      try {
        return returnAtomValue(aState);
      } finally {
        atomState.d.set(a, aState.n);
        if (isPendingPromise(atomState.v)) {
          addPendingPromiseToDependency(atom, atomState.v, aState);
        }
        if (mountedMap.has(atom)) {
          var _mountedMap$get3;
          (_mountedMap$get3 = mountedMap.get(a)) == null || _mountedMap$get3.t.add(atom);
        }
        if (!isSync) {
          mountDependenciesIfAsync();
        }
      }
    }
    var controller;
    var setSelf;
    var options = {
      get signal() {
        if (!controller) {
          controller = new AbortController();
        }
        return controller.signal;
      },
      get setSelf() {
        if (!isActuallyWritableAtom(atom)) {
          console.warn('setSelf function cannot be used with read-only atom');
        }
        if (!setSelf && isActuallyWritableAtom(atom)) {
          setSelf = function setSelf() {
            if (isSync) {
              console.warn('setSelf function cannot be called in sync');
            }
            if (!isSync) {
              try {
                for (var _len3 = arguments.length, args = new Array(_len3), _key3 = 0; _key3 < _len3; _key3++) {
                  args[_key3] = arguments[_key3];
                }
                return writeAtomState.apply(void 0, [store, atom].concat(args));
              } finally {
                recomputeInvalidatedAtoms(store);
                flushCallbacks(store);
              }
            }
          };
        }
        return setSelf;
      }
    };
    var prevEpochNumber = atomState.n;
    try {
      if ("development" !== 'production') {
        storeMutationSet.delete(store);
      }
      var _valueOrPromise = atomRead(store, atom, getter, options);
      if ("development" !== 'production' && storeMutationSet.has(store)) {
        console.warn('Detected store mutation during atom read. This is not supported.');
      }
      setAtomStateValueOrPromise(store, atom, _valueOrPromise);
      if (isPromiseLike(_valueOrPromise)) {
        registerAbortHandler(_valueOrPromise, function () {
          var _controller;
          return (_controller = controller) == null ? void 0 : _controller.abort();
        });
        _valueOrPromise.then(mountDependenciesIfAsync, mountDependenciesIfAsync);
      }
      storeHooks.r == null || storeHooks.r(atom);
      return atomState;
    } catch (error) {
      delete atomState.v;
      atomState.e = error;
      ++atomState.n;
      return atomState;
    } finally {
      isSync = false;
      if (prevEpochNumber !== atomState.n && invalidatedAtoms.get(atom) === prevEpochNumber) {
        invalidatedAtoms.set(atom, atomState.n);
        changedAtoms.add(atom);
        storeHooks.c == null || storeHooks.c(atom);
      }
    }
  };
  var BUILDING_BLOCK_invalidateDependents = function BUILDING_BLOCK_invalidateDependents(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var invalidatedAtoms = buildingBlocks[2];
    var ensureAtomState = buildingBlocks[11];
    var stack = [atom];
    while (stack.length) {
      var a = stack.pop();
      var aState = ensureAtomState(store, a);
      for (var _iterator6 = _createForOfIteratorHelperLoose(getMountedOrPendingDependents(a, aState, mountedMap)), _step6; !(_step6 = _iterator6()).done;) {
        var d = _step6.value;
        var dState = ensureAtomState(store, d);
        invalidatedAtoms.set(d, dState.n);
        stack.push(d);
      }
    }
  };
  var BUILDING_BLOCK_writeAtomState = function BUILDING_BLOCK_writeAtomState(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var changedAtoms = buildingBlocks[3];
    var storeHooks = buildingBlocks[6];
    var atomWrite = buildingBlocks[8];
    var ensureAtomState = buildingBlocks[11];
    var flushCallbacks = buildingBlocks[12];
    var recomputeInvalidatedAtoms = buildingBlocks[13];
    var readAtomState = buildingBlocks[14];
    var invalidateDependents = buildingBlocks[15];
    var writeAtomState = buildingBlocks[16];
    var mountDependencies = buildingBlocks[17];
    var setAtomStateValueOrPromise = buildingBlocks[20];
    var isSync = true;
    var getter = function getter(a) {
      return returnAtomValue(readAtomState(store, a));
    };
    var setter = function setter(a) {
      var aState = ensureAtomState(store, a);
      try {
        for (var _len5 = arguments.length, args = new Array(_len5 > 1 ? _len5 - 1 : 0), _key5 = 1; _key5 < _len5; _key5++) {
          args[_key5 - 1] = arguments[_key5];
        }
        if (a === atom) {
          if (!hasInitialValue(a)) {
            throw new Error('atom not writable');
          }
          if ("development" !== 'production') {
            storeMutationSet.add(store);
          }
          var prevEpochNumber = aState.n;
          var v = args[0];
          setAtomStateValueOrPromise(store, a, v);
          mountDependencies(store, a);
          if (prevEpochNumber !== aState.n) {
            changedAtoms.add(a);
            invalidateDependents(store, a);
            storeHooks.c == null || storeHooks.c(a);
          }
          return undefined;
        } else {
          return writeAtomState.apply(void 0, [store, a].concat(args));
        }
      } finally {
        if (!isSync) {
          recomputeInvalidatedAtoms(store);
          flushCallbacks(store);
        }
      }
    };
    try {
      for (var _len4 = arguments.length, args = new Array(_len4 > 2 ? _len4 - 2 : 0), _key4 = 2; _key4 < _len4; _key4++) {
        args[_key4 - 2] = arguments[_key4];
      }
      return atomWrite.apply(void 0, [store, atom, getter, setter].concat(args));
    } finally {
      isSync = false;
    }
  };
  var BUILDING_BLOCK_mountDependencies = function BUILDING_BLOCK_mountDependencies(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var changedAtoms = buildingBlocks[3];
    var storeHooks = buildingBlocks[6];
    var ensureAtomState = buildingBlocks[11];
    var invalidateDependents = buildingBlocks[15];
    var mountAtom = buildingBlocks[18];
    var unmountAtom = buildingBlocks[19];
    var atomState = ensureAtomState(store, atom);
    var mounted = mountedMap.get(atom);
    if (mounted && !isPendingPromise(atomState.v)) {
      for (var _iterator7 = _createForOfIteratorHelperLoose(atomState.d), _step7; !(_step7 = _iterator7()).done;) {
        var _step7$value = _step7.value,
          a = _step7$value[0],
          n = _step7$value[1];
        if (!mounted.d.has(a)) {
          var aState = ensureAtomState(store, a);
          var aMounted = mountAtom(store, a);
          aMounted.t.add(atom);
          mounted.d.add(a);
          if (n !== aState.n) {
            changedAtoms.add(a);
            invalidateDependents(store, a);
            storeHooks.c == null || storeHooks.c(a);
          }
        }
      }
      for (var _iterator8 = _createForOfIteratorHelperLoose(mounted.d), _step8; !(_step8 = _iterator8()).done;) {
        var _a2 = _step8.value;
        if (!atomState.d.has(_a2)) {
          mounted.d.delete(_a2);
          var _aMounted = unmountAtom(store, _a2);
          _aMounted == null || _aMounted.t.delete(atom);
        }
      }
    }
  };
  var BUILDING_BLOCK_mountAtom = function BUILDING_BLOCK_mountAtom(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var mountCallbacks = buildingBlocks[4];
    var storeHooks = buildingBlocks[6];
    var atomOnMount = buildingBlocks[10];
    var ensureAtomState = buildingBlocks[11];
    var flushCallbacks = buildingBlocks[12];
    var recomputeInvalidatedAtoms = buildingBlocks[13];
    var readAtomState = buildingBlocks[14];
    var writeAtomState = buildingBlocks[16];
    var mountAtom = buildingBlocks[18];
    var atomState = ensureAtomState(store, atom);
    var mounted = mountedMap.get(atom);
    if (!mounted) {
      readAtomState(store, atom);
      for (var _iterator9 = _createForOfIteratorHelperLoose(atomState.d.keys()), _step9; !(_step9 = _iterator9()).done;) {
        var a = _step9.value;
        var aMounted = mountAtom(store, a);
        aMounted.t.add(atom);
      }
      mounted = {
        l: new Set(),
        d: new Set(atomState.d.keys()),
        t: new Set()
      };
      mountedMap.set(atom, mounted);
      if (isActuallyWritableAtom(atom)) {
        var processOnMount = function processOnMount() {
          var isSync = true;
          var setAtom = function setAtom() {
            try {
              for (var _len6 = arguments.length, args = new Array(_len6), _key6 = 0; _key6 < _len6; _key6++) {
                args[_key6] = arguments[_key6];
              }
              return writeAtomState.apply(void 0, [store, atom].concat(args));
            } finally {
              if (!isSync) {
                recomputeInvalidatedAtoms(store);
                flushCallbacks(store);
              }
            }
          };
          try {
            var onUnmount = atomOnMount(store, atom, setAtom);
            if (onUnmount) {
              mounted.u = function () {
                isSync = true;
                try {
                  onUnmount();
                } finally {
                  isSync = false;
                }
              };
            }
          } finally {
            isSync = false;
          }
        };
        mountCallbacks.add(processOnMount);
      }
      storeHooks.m == null || storeHooks.m(atom);
    }
    return mounted;
  };
  var BUILDING_BLOCK_unmountAtom = function BUILDING_BLOCK_unmountAtom(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var mountedMap = buildingBlocks[1];
    var unmountCallbacks = buildingBlocks[5];
    var storeHooks = buildingBlocks[6];
    var ensureAtomState = buildingBlocks[11];
    var unmountAtom = buildingBlocks[19];
    var atomState = ensureAtomState(store, atom);
    var mounted = mountedMap.get(atom);
    if (!mounted || mounted.l.size) {
      return mounted;
    }
    var isDependent = false;
    for (var _iterator0 = _createForOfIteratorHelperLoose(mounted.t), _step0; !(_step0 = _iterator0()).done;) {
      var _mountedMap$get4;
      var _a3 = _step0.value;
      if ((_mountedMap$get4 = mountedMap.get(_a3)) != null && _mountedMap$get4.d.has(atom)) {
        isDependent = true;
        break;
      }
    }
    if (!isDependent) {
      if (mounted.u) {
        unmountCallbacks.add(mounted.u);
      }
      mounted = undefined;
      mountedMap.delete(atom);
      for (var _iterator1 = _createForOfIteratorHelperLoose(atomState.d.keys()), _step1; !(_step1 = _iterator1()).done;) {
        var a = _step1.value;
        var aMounted = unmountAtom(store, a);
        aMounted == null || aMounted.t.delete(atom);
      }
      storeHooks.u == null || storeHooks.u(atom);
      return undefined;
    }
    return mounted;
  };
  var BUILDING_BLOCK_setAtomStateValueOrPromise = function BUILDING_BLOCK_setAtomStateValueOrPromise(store, atom, valueOrPromise) {
    var ensureAtomState = getInternalBuildingBlocks(store)[11];
    var atomState = ensureAtomState(store, atom);
    var hasPrevValue = 'v' in atomState;
    var prevValue = atomState.v;
    if (isPromiseLike(valueOrPromise)) {
      for (var _iterator10 = _createForOfIteratorHelperLoose(atomState.d.keys()), _step10; !(_step10 = _iterator10()).done;) {
        var a = _step10.value;
        addPendingPromiseToDependency(atom, valueOrPromise, ensureAtomState(store, a));
      }
    }
    atomState.v = valueOrPromise;
    delete atomState.e;
    if (!hasPrevValue || !Object.is(prevValue, atomState.v)) {
      ++atomState.n;
      if (isPromiseLike(prevValue)) {
        abortPromise(prevValue);
      }
    }
  };
  var BUILDING_BLOCK_storeGet = function BUILDING_BLOCK_storeGet(store, atom) {
    var readAtomState = getInternalBuildingBlocks(store)[14];
    return returnAtomValue(readAtomState(store, atom));
  };
  var BUILDING_BLOCK_storeSet = function BUILDING_BLOCK_storeSet(store, atom) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var flushCallbacks = buildingBlocks[12];
    var recomputeInvalidatedAtoms = buildingBlocks[13];
    var writeAtomState = buildingBlocks[16];
    try {
      for (var _len7 = arguments.length, args = new Array(_len7 > 2 ? _len7 - 2 : 0), _key7 = 2; _key7 < _len7; _key7++) {
        args[_key7 - 2] = arguments[_key7];
      }
      return writeAtomState.apply(void 0, [store, atom].concat(args));
    } finally {
      recomputeInvalidatedAtoms(store);
      flushCallbacks(store);
    }
  };
  var BUILDING_BLOCK_storeSub = function BUILDING_BLOCK_storeSub(store, atom, listener) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var flushCallbacks = buildingBlocks[12];
    var mountAtom = buildingBlocks[18];
    var unmountAtom = buildingBlocks[19];
    var mounted = mountAtom(store, atom);
    var listeners = mounted.l;
    listeners.add(listener);
    flushCallbacks(store);
    return function () {
      listeners.delete(listener);
      unmountAtom(store, atom);
      flushCallbacks(store);
    };
  };
  var buildingBlockMap = new WeakMap();
  var getInternalBuildingBlocks = function getInternalBuildingBlocks(store) {
    var buildingBlocks = buildingBlockMap.get(store);
    if (!buildingBlocks) {
      throw new Error('Store must be created by buildStore to read its building blocks');
    }
    return buildingBlocks;
  };
  function getBuildingBlocks(store) {
    var buildingBlocks = getInternalBuildingBlocks(store);
    var enhanceBuildingBlocks = buildingBlocks[24];
    if (enhanceBuildingBlocks) {
      return enhanceBuildingBlocks(buildingBlocks);
    }
    return buildingBlocks;
  }
  function buildStore() {
    for (var _len8 = arguments.length, buildArgs = new Array(_len8), _key8 = 0; _key8 < _len8; _key8++) {
      buildArgs[_key8] = arguments[_key8];
    }
    var store = {
      get: function get(atom) {
        var storeGet = getInternalBuildingBlocks(store)[21];
        return storeGet(store, atom);
      },
      set: function set(atom) {
        var storeSet = getInternalBuildingBlocks(store)[22];
        for (var _len9 = arguments.length, args = new Array(_len9 > 1 ? _len9 - 1 : 0), _key9 = 1; _key9 < _len9; _key9++) {
          args[_key9 - 1] = arguments[_key9];
        }
        return storeSet.apply(void 0, [store, atom].concat(args));
      },
      sub: function sub(atom, listener) {
        var storeSub = getInternalBuildingBlocks(store)[23];
        return storeSub(store, atom, listener);
      }
    };
    var buildingBlocks = [new WeakMap(), new WeakMap(), new WeakMap(), new Set(), new Set(), new Set(), {}, BUILDING_BLOCK_atomRead, BUILDING_BLOCK_atomWrite, BUILDING_BLOCK_atomOnInit, BUILDING_BLOCK_atomOnMount, BUILDING_BLOCK_ensureAtomState, BUILDING_BLOCK_flushCallbacks, BUILDING_BLOCK_recomputeInvalidatedAtoms, BUILDING_BLOCK_readAtomState, BUILDING_BLOCK_invalidateDependents, BUILDING_BLOCK_writeAtomState, BUILDING_BLOCK_mountDependencies, BUILDING_BLOCK_mountAtom, BUILDING_BLOCK_unmountAtom, BUILDING_BLOCK_setAtomStateValueOrPromise, BUILDING_BLOCK_storeGet, BUILDING_BLOCK_storeSet, BUILDING_BLOCK_storeSub, undefined].map(function (fn, i) {
      return buildArgs[i] || fn;
    });
    buildingBlockMap.set(store, Object.freeze(buildingBlocks));
    return store;
  }

  exports.INTERNAL_abortPromise = abortPromise;
  exports.INTERNAL_addPendingPromiseToDependency = addPendingPromiseToDependency;
  exports.INTERNAL_buildStoreRev2 = buildStore;
  exports.INTERNAL_getBuildingBlocksRev2 = getBuildingBlocks;
  exports.INTERNAL_getMountedOrPendingDependents = getMountedOrPendingDependents;
  exports.INTERNAL_hasInitialValue = hasInitialValue;
  exports.INTERNAL_initializeStoreHooksRev2 = initializeStoreHooks;
  exports.INTERNAL_isActuallyWritableAtom = isActuallyWritableAtom;
  exports.INTERNAL_isAtomStateInitialized = isAtomStateInitialized;
  exports.INTERNAL_isPendingPromise = isPendingPromise;
  exports.INTERNAL_isPromiseLike = isPromiseLike;
  exports.INTERNAL_promiseStateMap = promiseStateMap;
  exports.INTERNAL_registerAbortHandler = registerAbortHandler;
  exports.INTERNAL_returnAtomValue = returnAtomValue;

}));
