System.register(['@babel/core', '@babel/template'], (function (exports) {
  'use strict';
  var _templateBuilder;
  return {
    setters: [null, function (module) {
      _templateBuilder = module.default;
    }],
    execute: (function () {

      exports("default", debugLabelPlugin);

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
      function debugLabelPlugin({ types: t }, options) {
        return {
          visitor: {
            ExportDefaultDeclaration(nodePath, state) {
              const { node } = nodePath;
              if (t.isCallExpression(node.declaration) && isAtom(t, node.declaration.callee, options == null ? void 0 : options.customAtomNames)) {
                const filename = (state.filename || "unknown").replace(/\.\w+$/, "");
                let displayName = filename.split("/").pop();
                if (displayName === "index") {
                  displayName = filename.slice(0, -"/index".length).split("/").pop() || "unknown";
                }
                const buildExport = templateBuilder(`
          const %%atomIdentifier%% = %%atom%%;
          export default %%atomIdentifier%%
          `);
                const ast = buildExport({
                  atomIdentifier: t.identifier(displayName),
                  atom: node.declaration
                });
                nodePath.replaceWithMultiple(ast);
              }
            },
            VariableDeclarator(path) {
              if (t.isIdentifier(path.node.id) && t.isCallExpression(path.node.init) && isAtom(t, path.node.init.callee, options == null ? void 0 : options.customAtomNames)) {
                path.parentPath.insertAfter(
                  t.expressionStatement(
                    t.assignmentExpression(
                      "=",
                      t.memberExpression(
                        t.identifier(path.node.id.name),
                        t.identifier("debugLabel")
                      ),
                      t.stringLiteral(path.node.id.name)
                    )
                  )
                );
              }
            }
          }
        };
      }

    })
  };
}));
