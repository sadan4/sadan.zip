import { type Config } from "stylelint";

export default {
    extends: ["stylelint-config-standard"],
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
                        ignoreAtRules: ["reference", "apply", "utility"],
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
    },
} satisfies Config;
