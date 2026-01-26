import {
  __name
} from "./chunks/mermaid-zenuml.esm/chunk-OS44DIFC.mjs";

// src/detector.ts
var id = "zenuml";
var detector = /* @__PURE__ */ __name((txt) => {
  return /^\s*zenuml/.test(txt);
}, "detector");
var loader = /* @__PURE__ */ __name(async () => {
  const { diagram } = await import("./chunks/mermaid-zenuml.esm/zenuml-definition-H5KMDA2O.mjs");
  return { id, diagram };
}, "loader");
var plugin = {
  id,
  detector,
  loader
};
var detector_default = plugin;
export {
  detector_default as default
};
