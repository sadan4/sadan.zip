import type { Config } from "stylelint";

export default {
    extends: ["stylelint-config-standard"],
    ignoreFiles: ["./dist/**/*", "./target/**/*"],
    overrides: [
        {
            files: "*.scss",
            extends: ["stylelint-config-standard-scss"],
        },
        {
            files: "*.css",
            rules: {
                "at-rule-no-unknown": [
                    true,
                    {
                        // tailwind at rules
                        ignoreAtRules: ["reference", "apply", "utility", "theme", "source"],
                    },
                ],
            },
        },
        {
            files: ["*.module.css", "*.module.scss"],
            rules: {
                "selector-pseudo-class-no-unknown": [
                    true,
                    {
                        ignorePseudoClasses: ["global", "local"],
                    },
                ],
            },
        },
    ],
    rules: {
        "lightness-notation": "number",
        "hue-degree-notation": "number",
        "alpha-value-notation": "number",
        "value-keyword-case": [
            "lower",
            {
                ignoreFunctions: ["local"],
                camelCaseSvgKeywords: true,
            },
        ],
        "custom-property-pattern": null,
        // tailwind only inlines string imports; `url()` imports skip its @theme processing
        "import-notation": "string",
    },
} satisfies Config;
