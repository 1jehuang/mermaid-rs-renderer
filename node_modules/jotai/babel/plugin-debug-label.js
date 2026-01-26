'use strict';

var _templateBuilder = require('@babel/template');

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

var templateBuilder = _templateBuilder.default || _templateBuilder;
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
          var buildExport = templateBuilder("\n          const %%atomIdentifier%% = %%atom%%;\n          export default %%atomIdentifier%%\n          ");
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

module.exports = debugLabelPlugin;
