(function (global, factory) {
	typeof exports === 'object' && typeof module !== 'undefined' ? factory(exports, require('jotai/vanilla/utils'), require('jotai/react/utils')) :
	typeof define === 'function' && define.amd ? define(['exports', 'jotai/vanilla/utils', 'jotai/react/utils'], factory) :
	(global = typeof globalThis !== 'undefined' ? globalThis : global || self, factory(global.jotaiUtils = {}, global.jotaiVanillaUtils, global.jotaiReactUtils));
})(this, (function (exports, utils, utils$1) { 'use strict';

	Object.keys(utils).forEach(function (k) {
		if (k !== 'default' && !Object.prototype.hasOwnProperty.call(exports, k)) Object.defineProperty(exports, k, {
			enumerable: true,
			get: function () { return utils[k]; }
		});
	});
	Object.keys(utils$1).forEach(function (k) {
		if (k !== 'default' && !Object.prototype.hasOwnProperty.call(exports, k)) Object.defineProperty(exports, k, {
			enumerable: true,
			get: function () { return utils$1[k]; }
		});
	});

}));
