'use client';
import { useCallback, useMemo } from 'react';
import { useSetAtom, useAtom, useStore } from 'jotai/react';
import { RESET } from 'jotai/vanilla/utils';
import { atom } from 'jotai/vanilla';

function useResetAtom(anAtom, options) {
  const setAtom = useSetAtom(anAtom, options);
  const resetAtom = useCallback(() => setAtom(RESET), [setAtom]);
  return resetAtom;
}

function useReducerAtom(anAtom, reducer, options) {
  if ((import.meta.env ? import.meta.env.MODE : void 0) !== "production") {
    console.warn(
      "[DEPRECATED] useReducerAtom is deprecated and will be removed in the future. Please create your own version using the recipe. https://github.com/pmndrs/jotai/pull/2467"
    );
  }
  const [state, setState] = useAtom(anAtom, options);
  const dispatch = useCallback(
    (action) => {
      setState((prev) => reducer(prev, action));
    },
    [setState, reducer]
  );
  return [state, dispatch];
}

function useAtomCallback(callback, options) {
  const anAtom = useMemo(
    () => atom(null, (get, set, ...args) => callback(get, set, ...args)),
    [callback]
  );
  return useSetAtom(anAtom, options);
}

const hydratedMap = /* @__PURE__ */ new WeakMap();
function useHydrateAtoms(values, options) {
  const store = useStore(options);
  const hydratedSet = getHydratedSet(store);
  for (const [atom, ...args] of values) {
    if (!hydratedSet.has(atom) || (options == null ? void 0 : options.dangerouslyForceHydrate)) {
      hydratedSet.add(atom);
      store.set(atom, ...args);
    }
  }
}
const getHydratedSet = (store) => {
  let hydratedSet = hydratedMap.get(store);
  if (!hydratedSet) {
    hydratedSet = /* @__PURE__ */ new WeakSet();
    hydratedMap.set(store, hydratedSet);
  }
  return hydratedSet;
};

export { useAtomCallback, useHydrateAtoms, useReducerAtom, useResetAtom };
