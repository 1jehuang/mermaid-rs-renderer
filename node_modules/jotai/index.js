'use strict';

var vanilla = require('jotai/vanilla');
var react = require('jotai/react');



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
