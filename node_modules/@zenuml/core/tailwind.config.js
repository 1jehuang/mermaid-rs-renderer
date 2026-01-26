module.exports = {
  important: ".zenuml",
  content: ["./src/**/*.{js,jsx,ts,tsx,vue}", "./index.html"],
  safelist: [
    // add classes from tailwind.css
    "theme-default",
    "theme-mermaid",
    "theme-darcula",
    "theme-sky",
    "theme-idle-afternoon",
    "theme-coles",
    "theme-woolworths",
    "theme-anz",
    "theme-nab",
    "theme-google",
    "theme-diagramly",
    "theme-creately",
    "theme-purple",
    {
      pattern: /(bg|text|outline)-.*/,
    },
  ],
  theme: {
    extend: {
      colors: {
        skin: {
          frame: "var(--color-bg-frame)", // it will generate from/via/to-skin-frame. see lifeline layer
        },
      },
      textColor: {
        skin: {
          title:
            "var(--color-text-title, var(--color-text-message, var(--color-text-base, #000)))",
          participant:
            "var(--color-text-participant, var(--color-text-message, var(--color-text-base, #000)))",
          "lifeline-group-name":
            "var(--color-text-message, var(--color-text-base, #000))",
          message: "var(--color-text-message, var(--color-text-base, #000))",
          "message-arrow":
            "var(--color-message-arrow, var(--color-border-frame, var(--color-border-base, #000)))", // message arrow head
          comment:
            "var(--color-text-comment, var(--color-text-secondary, var(--color-text-base, #000)))",
          "fragment-header":
            "var(--color-text-fragment-header, var(--color-text-message, #000))",
          fragment:
            "var(--color-text-fragment, var(--color-text-message, #000))",
          base: "var(--color-text-base)",
          header: "var(--color-text-header)",
          secondary: "var(--color-text-secondary)",
          control:
            "var(--color-text-control, var(--color-text-secondary, var(--color-text-base, #000)))",
          muted: "var(--color-text-muted)",
          hover: "var(--color-text-hover)",
          link: "var(--color-text-link, var(--color-text-secondary, var(--color-text-base, #000)))",
          fill: "var(--color-text-fill)",
        },
      },
      backgroundColor: {
        skin: {
          canvas:
            "var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff)))",
          frame:
            "var(--color-bg-frame, var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff))))",
          title:
            "var(--color-bg-title, var(--color-bg-frame, var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff)))))",
          participant:
            "var(--color-bg-participant, var(--color-bg-frame, var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff)))))",
          lifeline:
            "var(--color-border-participant, var(--color-border-participant, var(--color-border-frame, var(--color-border-base, #000))))",
          divider:
            "var(--color-border-participant, var(--color-border-frame, var(--color-border-base, #000)))",
          "fragment-header":
            "var(--color-bg-fragment-header, var(--color-bg-frame, var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff)))))",
          occurrence:
            "var(--color-bg-occurrence, var(--color-bg-participant, var(--color-bg-frame, var(--color-bg-canvas, var(--color-bg-base, var(--color-backup-white, #ffffff))))))",
          base: "var(--color-bg-base)",
          secondary: "var(--color-bg-secondary)",
          hover: "var(--color-bg-hover)",
          fill: "var(--color-bg-fill)",
        },
      },
      borderColor: {
        primary: "var(--color-border-primary)",
        skin: {
          frame: "var(--color-border-frame, var(--color-border-base, #000))",
          participant:
            "var(--color-border-participant, var(--color-border-frame, var(--color-border-base, #000)))",
          "message-arrow":
            "var(--color-message-arrow, var(--color-border-frame,  var(--color-border-base, #000)))", // message arrow line
          fragment:
            "var(--color-border-fragment, var(--color-border-frame,  var(--color-border-base, #000)))",
          occurrence:
            "var(--color-border-occurrence, var(--color-border-frame,  var(--color-border-base, #000)))",
          base: "var(--color-border-base)",
          secondary: "var(--color-border-secondary)",
        },
      },
      outlineColor: {
        skin: {
          primary: "var(--color-outline-primary)",
        },
      },
      gradientColorStops: {
        skin: {
          base: "var(--color-bg-base)",
          secondary: "var(--color-bg-secondary)",
        },
      },
      boxShadow: {
        participant: "var(--color-shadow-participant, transparent)",
        occurrence: "var(--color-shadow-occurrence, transparent)",
      },
    },
  },
  variants: {
    extend: {
      overflow: ["hover", "focus"],
      whitespace: ["hover", "focus"],
      display: ["group-hover"],
    },
  },
  corePlugins: {
    preflight: false,
  },
  plugins: [require("@headlessui/tailwindcss")],
};
