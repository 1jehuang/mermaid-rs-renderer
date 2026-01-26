The `module` field is not officially defined by Node.js and support is not planned.
Instead, the Node.js community settled on package `exports` which they believe is
more versatile.

For practical reasons JavaScript bundlers will continue support the `module` field.
The esbuild docs [explain](https://esbuild.github.io/api/#main-fields)
when to use `module` as well as related fields like `main` and `browser`.


* `main` is the standard field for all packages that are mean to be used with node.
It is reasonable to expect that the file path in this field is a valid CommonJS module.
* `module` is a non-standard field that is used to integrate ECMAScript modules into
node. It is reasonable to expect that the file path in this field is an ECMAScript module (esm).

For @zenuml/core, the `main` field is set to `dist/zenuml.js` and the `module` field
is set to `dist/zenuml.esm.js`.

