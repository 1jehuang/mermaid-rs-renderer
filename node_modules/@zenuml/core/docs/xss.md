In this document we are disclosing a cross-site scripting vulnerability in the
[@zenuml/core](https://www.npmjs.com/package/@zenuml/core) package.
XSS is a type of security vulnerability that allows an attacker to inject
malicious code into a web page viewed by other users.

# How to reproduce

ZenUML generates sequence diagrams from text. If the text contains a
malicious script, it will be executed when the diagram is rendered.

The following content is known to pop an alert box:

```
"><img src=x onerror=alert(1)>ent #FFEBE6
```

# Known affected products

| Product           | Severity | Comment                                                             |
|-------------------|----------|---------------------------------------------------------------------|
| Confluence plugin | P3       | Content must be create by registered users; scripts run in sandbox. |
| Web App           | P3       | Only the current user can view content created by themselves.       |
| Chrome Extension  | P3       | Only the current user can view content created by themselves.       |
| Desktop (Win/Mac) | P3       | Only the current user can view content created by themselves.       |
| JetBrains plugin  | P3       | Scripts run inside sandbox.                                         |

# What is the cause of the vulnerability?

The cause of the XSS vulnerability in @zenuml/core is due to accepting an arbitrary text as `innerHTML` in the
WidthProviderFunc.ts file. Specifically, the vulnerability is located in the code that measures the width 
of the message or participant element in the library. The issue is present in line 29 of the file, which 
can be found at [this URL](https://github.com/ZenUml/core/blob/577f2a550a0b82a392215875298bc358a8feff0d/src/positioning/WidthProviderFunc.ts#L29). 
In this line, the code uses unsanitized user input which is passed as an argument in the function, this 
allows an attacker to inject malicious JavaScript code into the web page viewed potentially by other users.

```
  let hiddenDiv = document.querySelector('.textarea-hidden-div') as HTMLDivElement;
  if (!hiddenDiv) {
    const newDiv = document.createElement('div');
    ...
    document.body.appendChild(newDiv);
    hiddenDiv = newDiv;
  }
  hiddenDiv.innerHTML = text;
  const scrollWidth = hiddenDiv.scrollWidth;
  return scrollWidth;
```

# Fix of the vulnerability

The fix is to replace the `innerHTML` with `textContent` in the code. This will prevent the browser from
interpreting the text as HTML and will treat it as plain text.

```
  ...
  hiddenDiv.textContent = text;
  const scrollWidth = hiddenDiv.scrollWidth;
  return scrollWidth;
```
