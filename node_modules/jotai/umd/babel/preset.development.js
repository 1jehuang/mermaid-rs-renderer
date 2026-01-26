(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? module.exports = factory(require('@babel/template')) :
  typeof define === 'function' && define.amd ? define(['@babel/template'], factory) :
  (global = typeof globalThis !== 'undefined' ? globalThis : global || self, global.jotaiBabelPreset = factory(global._templateBuilder));
})(this, (function (_templateBuilder) { 'use strict';

  function isAtom(t, callee, customAtomNames) {
    if (customAtomNames === void 0) {
      customAtomNames = [];
    }
    var atomNames = [].concat(atomFunctionNames, customAtomNames);
    if (t.isIdentifier(callee) && atomNames.includes(callee.name)) {
      return true;
    }
    if (t.isMemberExpression(callee)) {
      var property = callee.property;
      if (t.isIdentifier(property) && atomNames.includes(property.name)) {
        return true;
      }
    }
    return false;
  }
  var atomFunctionNames = ['atom', 'atomFamily', 'atomWithDefault', 'atomWithObservable', 'atomWithReducer', 'atomWithReset', 'atomWithStorage', 'freezeAtom', 'loadable', 'selectAtom', 'splitAtom', 'unwrap', 'atomWithMachine', 'atomWithImmer', 'atomWithProxy', 'atomWithQuery', 'atomWithMutation', 'atomWithSubscription', 'atomWithStore', 'atomWithHash', 'atomWithLocation', 'focusAtom', 'atomWithValidate', 'validateAtoms', 'atomWithCache', 'atomWithRecoilValue'];

  var templateBuilder$1 = _templateBuilder.default || _templateBuilder;
  function debugLabelPlugin(_ref, options) {
    var t = _ref.types;
    return {
      visitor: {
        ExportDefaultDeclaration: function ExportDefaultDeclaration(nodePath, state) {
          var node = nodePath.node;
          if (t.isCallExpression(node.declaration) && isAtom(t, node.declaration.callee, options == null ? void 0 : options.customAtomNames)) {
            var filename = (state.filename || 'unknown').replace(/\.\w+$/, '');
            var displayName = filename.split('/').pop();
            if (displayName === 'index') {
              displayName = filename.slice(0, -'/index'.length).split('/').pop() || 'unknown';
            }
            var buildExport = templateBuilder$1("\n          const %%atomIdentifier%% = %%atom%%;\n          export default %%atomIdentifier%%\n          ");
            var ast = buildExport({
              atomIdentifier: t.identifier(displayName),
              atom: node.declaration
            });
            nodePath.replaceWithMultiple(ast);
          }
        },
        VariableDeclarator: function VariableDeclarator(path) {
          if (t.isIdentifier(path.node.id) && t.isCallExpression(path.node.init) && isAtom(t, path.node.init.callee, options == null ? void 0 : options.customAtomNames)) {
            path.parentPath.insertAfter(t.expressionStatement(t.assignmentExpression('=', t.memberExpression(t.identifier(path.node.id.name), t.identifier('debugLabel')), t.stringLiteral(path.node.id.name))));
          }
        }
      }
    };
  }

  var templateBuilder = _templateBuilder.default || _templateBuilder;
  function reactRefreshPlugin(_ref, options) {
    var t = _ref.types;
    return {
      pre: function pre(_ref2) {
        var opts = _ref2.opts;
        if (!opts.filename) {
          throw new Error('Filename must be available');
        }
      },
      visitor: {
        Program: {
          exit: function exit(path) {
            var jotaiAtomCache = templateBuilder("\n          globalThis.jotaiAtomCache = globalThis.jotaiAtomCache || {\n            cache: new Map(),\n            get(name, inst) { \n              if (this.cache.has(name)) {\n                return this.cache.get(name)\n              }\n              this.cache.set(name, inst)\n              return inst\n            },\n          }")();
            path.unshiftContainer('body', jotaiAtomCache);
          }
        },
        ExportDefaultDeclaration: function ExportDefaultDeclaration(nodePath, state) {
          var node = nodePath.node;
          if (t.isCallExpression(node.declaration) && isAtom(t, node.declaration.callee, options == null ? void 0 : options.customAtomNames)) {
            var filename = state.filename || 'unknown';
            var atomKey = filename + "/defaultExport";
            var buildExport = templateBuilder("export default globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)");
            var ast = buildExport({
              atomKey: t.stringLiteral(atomKey),
              atom: node.declaration
            });
            nodePath.replaceWith(ast);
          }
        },
        VariableDeclarator: function VariableDeclarator(nodePath, state) {
          var _nodePath$parentPath$, _nodePath$parentPath$2;
          if (t.isIdentifier(nodePath.node.id) && t.isCallExpression(nodePath.node.init) && isAtom(t, nodePath.node.init.callee, options == null ? void 0 : options.customAtomNames) && ((_nodePath$parentPath$ = nodePath.parentPath.parentPath) != null && _nodePath$parentPath$.isProgram() || (_nodePath$parentPath$2 = nodePath.parentPath.parentPath) != null && _nodePath$parentPath$2.isExportNamedDeclaration())) {
            var filename = state.filename || 'unknown';
            var atomKey = filename + "/" + nodePath.node.id.name;
            var buildAtomDeclaration = templateBuilder("const %%atomIdentifier%% = globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)");
            var ast = buildAtomDeclaration({
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

  function jotaiPreset(_, options) {
    return {
      plugins: [[debugLabelPlugin, options], [reactRefreshPlugin, options]]
    };
  }

  return jotaiPreset;

}));
