import '@babel/core';
import _templateBuilder from '@babel/template';

function isAtom(t, callee, customAtomNames = []) {
  const atomNames = [...atomFunctionNames, ...customAtomNames];
  if (t.isIdentifier(callee) && atomNames.includes(callee.name)) {
    return true;
  }
  if (t.isMemberExpression(callee)) {
    const { property } = callee;
    if (t.isIdentifier(property) && atomNames.includes(property.name)) {
      return true;
    }
  }
  return false;
}
const atomFunctionNames = [
  // Core
  "atom",
  "atomFamily",
  "atomWithDefault",
  "atomWithObservable",
  "atomWithReducer",
  "atomWithReset",
  "atomWithStorage",
  "freezeAtom",
  "loadable",
  "selectAtom",
  "splitAtom",
  "unwrap",
  // jotai-xstate
  "atomWithMachine",
  // jotai-immer
  "atomWithImmer",
  // jotai-valtio
  "atomWithProxy",
  // jotai-trpc + jotai-relay
  "atomWithQuery",
  "atomWithMutation",
  "atomWithSubscription",
  // jotai-redux + jotai-zustand
  "atomWithStore",
  // jotai-location
  "atomWithHash",
  "atomWithLocation",
  // jotai-optics
  "focusAtom",
  // jotai-form
  "atomWithValidate",
  "validateAtoms",
  // jotai-cache
  "atomWithCache",
  // jotai-recoil
  "atomWithRecoilValue"
];

const templateBuilder = _templateBuilder.default || _templateBuilder;
function reactRefreshPlugin({ types: t }, options) {
  return {
    pre({ opts }) {
      if (!opts.filename) {
        throw new Error("Filename must be available");
      }
    },
    visitor: {
      Program: {
        exit(path) {
          const jotaiAtomCache = templateBuilder(`
          globalThis.jotaiAtomCache = globalThis.jotaiAtomCache || {
            cache: new Map(),
            get(name, inst) { 
              if (this.cache.has(name)) {
                return this.cache.get(name)
              }
              this.cache.set(name, inst)
              return inst
            },
          }`)();
          path.unshiftContainer("body", jotaiAtomCache);
        }
      },
      ExportDefaultDeclaration(nodePath, state) {
        const { node } = nodePath;
        if (t.isCallExpression(node.declaration) && isAtom(t, node.declaration.callee, options == null ? void 0 : options.customAtomNames)) {
          const filename = state.filename || "unknown";
          const atomKey = `${filename}/defaultExport`;
          const buildExport = templateBuilder(
            `export default globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)`
          );
          const ast = buildExport({
            atomKey: t.stringLiteral(atomKey),
            atom: node.declaration
          });
          nodePath.replaceWith(ast);
        }
      },
      VariableDeclarator(nodePath, state) {
        var _a, _b;
        if (t.isIdentifier(nodePath.node.id) && t.isCallExpression(nodePath.node.init) && isAtom(t, nodePath.node.init.callee, options == null ? void 0 : options.customAtomNames) && // Make sure atom declaration is in module scope
        (((_a = nodePath.parentPath.parentPath) == null ? void 0 : _a.isProgram()) || ((_b = nodePath.parentPath.parentPath) == null ? void 0 : _b.isExportNamedDeclaration()))) {
          const filename = state.filename || "unknown";
          const atomKey = `${filename}/${nodePath.node.id.name}`;
          const buildAtomDeclaration = templateBuilder(
            `const %%atomIdentifier%% = globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)`
          );
          const ast = buildAtomDeclaration({
            atomIdentifier: t.identifier(nodePath.node.id.name),
            atomKey: t.stringLiteral(atomKey),
            atom: nodePath.node.init
          });
          nodePath.parentPath.replaceWith(ast);
        }
      }
    }
  };
}

export { reactRefreshPlugin as default };
