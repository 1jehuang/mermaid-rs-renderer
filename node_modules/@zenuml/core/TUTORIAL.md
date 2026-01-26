
# ZenUML Integration Tutorial

This tutorial provides a comprehensive guide on how to integrate ZenUML into your applications. There are two primary methods for integration: as a library or as an embedded iframe.

## 1. As a Library

This is the most flexible method, allowing for deep integration with your application.

### Installation

First, add the `@zenuml/core` package to your project:

```bash
npm install @zenuml/core
```

or

```bash
bun add @zenuml/core
```

### Usage

The main entry point of the library is the `ZenUml` class. Here's a basic example of how to use it:

```javascript
import ZenUml from '@zenuml/core';

// 1. Get the container element
const el = document.getElementById('zenuml-container');

// 2. Create a new instance of ZenUml
const zenUml = new ZenUml(el);

// 3. Render a diagram
const code = 'A->B: message';
const config = {
  theme: 'default',
};
zenUml.render(code, config);
```

### Configuration

The `render` method accepts a configuration object with the following properties:

- `theme`: The name of the theme to use. A list of available themes can be found in the documentation.
- `enableScopedTheming`: A boolean that indicates whether to scope the theme to the container element. This is useful when you have multiple diagrams on the same page with different themes.
- `onThemeChange`: A callback function that is called when the theme is changed.
- `enableMultiTheme`: A boolean that indicates whether to enable multi-theme support.
- `stickyOffset`: A number that indicates the offset for the sticky header.
- `onContentChange`: A callback function that is called when the content of the diagram is changed.
- `onEventEmit`: A callback function that is called when an event is emitted from the diagram.
- `mode`: The rendering mode. Can be `RenderMode.Dynamic` or `RenderMode.Static`.

### Example

Here's a more advanced example that uses some of the configuration options:

```javascript
import ZenUml from '@zenuml/core';

const el = document.getElementById('zenuml-container');
const zenUml = new ZenUml(el);

const code = `
  // This is a comment
  A->B: synchronous message
  B-->A: asynchronous message
`;

const config = {
  theme: 'blue',
  enableScopedTheming: true,
  onContentChange: (newCode) => {
    console.log('Diagram code changed:', newCode);
  },
};

zenUml.render(code, config);
```

### Exporting Diagrams

You can export diagrams to PNG and SVG formats. The `ZenUml` class provides the following methods for exporting:

- `getPng()`: Returns a promise that resolves to a PNG data URL.
- `getSvg()`: Returns a promise that resolves to an SVG data URL.

Here's an example of how to use these methods:

```javascript
import ZenUml from '@zenuml/core';

const el = document.getElementById('zenuml-container');
const zenUml = new ZenUml(el);

const code = 'A->B: message';

async function exportDiagram() {
  await zenUml.render(code, { theme: 'default' });
  const png = await zenUml.getPng();
  // Do something with the PNG data URL
  console.log(png);

  const svg = await zenUml.getSvg();
  // Do something with the SVG data URL
  console.log(svg);
}

exportDiagram();
```

This tutorial should provide you with a solid foundation for integrating ZenUML into your applications. For more detailed information, please refer to the official documentation.
