(function (global, factory) {
	typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports, require('jotai/vanilla'), require('jotai/react')) :
	typeof define === 'function' && define.amd ? define(['exports', 'jotai/vanilla', 'jotai/react'], factory) :
	(global = typeof globalThis !== 'undefined' ? globalThis : global || self, factory(global.jotai = {}, global.jotaiVanilla, global.jotaiReact));
})(this, (function (exports, vanilla, react) { 'use strict';

	Object.keys(vanilla).forEach(function (k) {
		if (k !== 'default' && !Object.prototype.hasOwnProperty.call(exports, k)) Object.defineProperty(exports, k, {
			enumerable: true,
			get: function () { return vanilla[k]; }
		});
	});
	Object.keys(react).forEach(function (k) {
		if (k !== 'default' && !Object.prototype.hasOwnProperty.call(exports, k)) Object.defineProperty(exports, k, {
			enumerable: true,
			get: function () { return react[k]; }
		});
	});

}));
