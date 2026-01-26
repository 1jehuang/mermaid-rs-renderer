import {
  __name
} from "./chunk-CJCKSLXZ.mjs";

// src/mermaidUtils.ts
var warning = /* @__PURE__ */ __name((s) => {
  console.error("Log function was called before initialization", s);
}, "warning");
var log = {
  trace: warning,
  debug: warning,
  info: warning,
  warn: warning,
  error: warning,
  fatal: warning
};
var setLogLevel;
var getConfig;
var sanitizeText;
var setupGraphViewbox;
var injectUtils = /* @__PURE__ */ __name((_log, _setLogLevel, _getConfig, _sanitizeText, _setupGraphViewbox) => {
  _log.info("Mermaid utils injected");
  log.trace = _log.trace;
  log.debug = _log.debug;
  log.info = _log.info;
  log.warn = _log.warn;
  log.error = _log.error;
  log.fatal = _log.fatal;
  setLogLevel = _setLogLevel;
  getConfig = _getConfig;
  sanitizeText = _sanitizeText;
  setupGraphViewbox = _setupGraphViewbox;
}, "injectUtils");

// src/parser.ts
var parser_default = {
  parse: /* @__PURE__ */ __name(() => {
  }, "parse")
};

// src/zenumlRenderer.ts
import ZenUml from "@zenuml/core";
var regexp = /^\s*zenuml/;
function createTemporaryZenumlContainer(id) {
  const container = document.createElement("div");
  container.id = `container-${id}`;
  container.style.display = "flex";
  container.innerHTML = `<div id="zenUMLApp-${id}"></div>`;
  const app = container.querySelector(`#zenUMLApp-${id}`);
  return { container, app };
}
__name(createTemporaryZenumlContainer, "createTemporaryZenumlContainer");
function createForeignObject(id) {
  const foreignObject = document.createElementNS("http://www.w3.org/2000/svg", "foreignObject");
  foreignObject.setAttribute("x", "0");
  foreignObject.setAttribute("y", "0");
  foreignObject.setAttribute("width", "100%");
  foreignObject.setAttribute("height", "100%");
  const { container, app } = createTemporaryZenumlContainer(id);
  foreignObject.appendChild(container);
  return { foreignObject, container, app };
}
__name(createForeignObject, "createForeignObject");
var draw = /* @__PURE__ */ __name(async function(text, id) {
  log.info("draw with Zen UML renderer", ZenUml);
  text = text.replace(regexp, "");
  const { securityLevel } = getConfig();
  let sandboxElement = null;
  if (securityLevel === "sandbox") {
    sandboxElement = document.getElementById("i" + id);
  }
  const root = securityLevel === "sandbox" ? sandboxElement?.contentWindow?.document : document;
  const svgContainer = root?.querySelector(`svg#${id}`);
  if (!root || !svgContainer) {
    log.error("Cannot find root or svgContainer");
    return;
  }
  const { foreignObject, container, app } = createForeignObject(id);
  svgContainer.appendChild(foreignObject);
  const zenuml = new ZenUml(app);
  await zenuml.render(text, { theme: "default", mode: "static" });
  const { width, height } = window.getComputedStyle(container);
  log.debug("zenuml diagram size", width, height);
  svgContainer.setAttribute("style", `width: ${width}; height: ${height};`);
}, "draw");
var zenumlRenderer_default = {
  draw
};

// src/zenuml-definition.ts
var diagram = {
  db: {
    clear: /* @__PURE__ */ __name(() => {
    }, "clear")
  },
  renderer: zenumlRenderer_default,
  parser: parser_default,
  styles: /* @__PURE__ */ __name(() => {
  }, "styles"),
  injectUtils
};
export {
  diagram
};
