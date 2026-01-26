(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports, require('jotai/vanilla/internals')) :
  typeof define === 'function' && define.amd ? define(['exports', 'jotai/vanilla/internals'], factory) :
  (global = typeof globalThis !== 'undefined' ? globalThis : global || self, factory(global.jotaiVanilla = {}, global.jotaiVanillaInternals));
})(this, (function (exports, internals) { 'use strict';

  var keyCount = 0;
  function atom(read, write) {
    var key = "atom" + ++keyCount;
    var config = {
      toString: function toString() {
        return this.debugLabel ? key + ':' + this.debugLabel : key;
      }
    };
    if (typeof read === 'function') {
      config.read = read;
    } else {
      config.init = read;
      config.read = defaultRead;
      config.write = defaultWrite;
    }
    if (write) {
      config.write = write;
    }
    return config;
  }
  function defaultRead(get) {
    return get(this);
  }
  function defaultWrite(get, set, arg) {
    return set(this, typeof arg === 'function' ? arg(get(this)) : arg);
  }

  var overriddenCreateStore;
  function INTERNAL_overrideCreateStore(fn) {
    overriddenCreateStore = fn(overriddenCreateStore);
  }
  function createStore() {
    if (overriddenCreateStore) {
      return overriddenCreateStore();
    }
    return internals.INTERNAL_buildStoreRev2();
  }
  var defaultStore;
  function getDefaultStore() {
    if (!defaultStore) {
      defaultStore = createStore();
      {
        var _ref;
        (_ref = globalThis).__JOTAI_DEFAULT_STORE__ || (_ref.__JOTAI_DEFAULT_STORE__ = defaultStore);
        if (globalThis.__JOTAI_DEFAULT_STORE__ !== defaultStore) {
          console.warn('Detected multiple Jotai instances. It may cause unexpected behavior with the default store. https://github.com/pmndrs/jotai/discussions/2044');
        }
      }
    }
    return defaultStore;
  }

  exports.INTERNAL_overrideCreateStore = INTERNAL_overrideCreateStore;
  exports.atom = atom;
  exports.createStore = createStore;
  exports.getDefaultStore = getDefaultStore;

}));
