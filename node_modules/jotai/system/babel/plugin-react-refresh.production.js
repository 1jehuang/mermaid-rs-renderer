System.register(["@babel/core","@babel/template"],(function(c){"use strict";var m;return{setters:[null,function(n){m=n.default}],execute:(function(){c("default",u);function n(e,a,t=[]){const o=[...h,...t];if(e.isIdentifier(a)&&o.includes(a.name))return!0;if(e.isMemberExpression(a)){const{property:i}=a;if(e.isIdentifier(i)&&o.includes(i.name))return!0}return!1}const h=["atom","atomFamily","atomWithDefault","atomWithObservable","atomWithReducer","atomWithReset","atomWithStorage","freezeAtom","loadable","selectAtom","splitAtom","unwrap","atomWithMachine","atomWithImmer","atomWithProxy","atomWithQuery","atomWithMutation","atomWithSubscription","atomWithStore","atomWithHash","atomWithLocation","focusAtom","atomWithValidate","validateAtoms","atomWithCache","atomWithRecoilValue"],l=m.default||m;function u({types:e},a){return{pre({opts:t}){if(!t.filename)throw new Error("Filename must be available")},visitor:{Program:{exit(t){const o=l(`
          globalThis.jotaiAtomCache = globalThis.jotaiAtomCache || {
            cache: new Map(),
            get(name, inst) { 
              if (this.cache.has(name)) {
                return this.cache.get(name)
              }
              this.cache.set(name, inst)
              return inst
            },
          }`)();t.unshiftContainer("body",o)}},ExportDefaultDeclaration(t,o){const{node:i}=t;if(e.isCallExpression(i.declaration)&&n(e,i.declaration.callee,a==null?void 0:a.customAtomNames)){const r=`${o.filename||"unknown"}/defaultExport`,s=l("export default globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)")({atomKey:e.stringLiteral(r),atom:i.declaration});t.replaceWith(s)}},VariableDeclarator(t,o){var i,r;if(e.isIdentifier(t.node.id)&&e.isCallExpression(t.node.init)&&n(e,t.node.init.callee,a==null?void 0:a.customAtomNames)&&((i=t.parentPath.parentPath)!=null&&i.isProgram()||(r=t.parentPath.parentPath)!=null&&r.isExportNamedDeclaration())){const s=`${o.filename||"unknown"}/${t.node.id.name}`,d=l("const %%atomIdentifier%% = globalThis.jotaiAtomCache.get(%%atomKey%%, %%atom%%)")({atomIdentifier:e.identifier(t.node.id.name),atomKey:e.stringLiteral(s),atom:t.node.init});t.parentPath.replaceWith(d)}}}}}})}}));
