// Migrated from ESLint (eslint.config.mts) to Oxlint.
// Structure preserved: grouped rule consts, the padding-line-between-statements
// IIFE + helpers, computed tailwind callees, and the shared `extensions` glob.
//
// Key differences vs the old ESLint flat config:
// - `@typescript-eslint/*` rules -> native `typescript/*` (or core where oxlint
//   folds them into the base rule, e.g. `no-use-before-define`, `default-param-last`,
//   `prefer-destructuring`).
// - `@eslint-react/*` -> react-x / react-dom / react-rsc / react-web-api /
//   react-naming-convention JS plugins.
// - `react-refresh/only-export-components` -> native `react/only-export-components`.
// - `react-hooks/*` (React Compiler family) -> native experimental
//   `react/react-compiler` rule.
// - `@stylistic/*` kept verbatim, loaded via the `@stylistic/eslint-plugin` JS plugin.
// - Local `require-css-as-namespace` rule loaded via its `.ts` source as a JS plugin.
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type DummyRuleMap } from "oxlint";

const __dirname = dirname(fileURLToPath(import.meta.url));

type RuleEntry = "off" | "warn" | "error" | [unknown, ...unknown[]];

type RuleMap = Record<string, RuleEntry>;

type _statementType = string;

type PaddingSchema = {
    blankLine: "any" | "always" | "never";
    prev: _statementType | _statementType[] | readonly _statementType[];
    next: _statementType | _statementType[] | readonly _statementType[];
}[];

const ESLintRules: DummyRuleMap = {
    "array-callback-return": [
        "error",
        {
            allowImplicit: true,
        },
    ],
    // done by tsserver
    "constructor-super": "off",
    "for-direction": "error",
    // done by tsserver
    "getter-return": "off",
    "no-async-promise-executor": "error",
    // done by tsserver
    "no-class-assign": "off",
    "no-compare-neg-zero": "error",
    "no-cond-assign": ["error", "except-parens"],
    // done by tsserver
    "no-const-assign": "off",
    "no-constant-binary-expression": "error",
    "no-constant-condition": [
        "error",
        {
            checkLoops: "allExceptWhileTrue",
        },
    ],
    "no-constructor-return": "error",
    "no-control-regex": "error",
    "no-debugger": "warn",
    // done by tsserver
    "no-dupe-class-members": "off",
    "no-dupe-else-if": "error",
    // done by tsserver
    "no-dupe-keys": "off",
    "no-duplicate-case": "error",
    "no-duplicate-imports": "error",
    "no-empty-character-class": "error",
    "no-empty-pattern": "error",
    "no-ex-assign": "error",
    "no-fallthrough": [
        "error",
        {
            allowEmptyCase: true,
            reportUnusedFallthroughComment: true,
        },
    ],
    // done by tsserver
    "no-func-assign": "off",
    // done by tsserver
    "no-import-assign": "off",
    // only for pre-es6
    "no-inner-declarations": "off",
    // FIXME: allow for \i in patches
    "no-invalid-regexp": "error",
    "no-irregular-whitespace": "error",
    "no-loss-of-precision": "error",
    "no-misleading-character-class": "error",
    // done by tsserver
    "no-new-native-nonconstructor": "off",
    // done by tsserver
    "no-obj-calls": "off",
    "no-prototype-builtins": "error",
    "no-self-assign": "error",
    "no-self-compare": "error",
    // done by tsserver
    "no-setter-return": "off",
    "no-sparse-arrays": "error",
    "no-template-curly-in-string": "error",
    // done by tsserver
    "no-this-before-super": "off",
    // done by tsserver
    "no-undef": "off",
    "no-unexpected-multiline": "error",
    "no-unmodified-loop-condition": "error",
    // done by tsserver
    "no-unreachable": "off",
    "no-unreachable-loop": "error",
    "no-unsafe-finally": "error",
    // done by tsserver
    "no-unsafe-negation": "off",
    "no-unsafe-optional-chaining": "error",
    "no-unused-private-class-members": "error",
    // done by no-unused-imports
    "no-unused-vars": "off",
    "no-useless-assignment": "error",
    "no-useless-backreference": "error",
    "use-isnan": [
        "error",
        {
            enforceForIndexOf: true,
            enforceForSwitchCase: true,
        },
    ],
    "valid-typeof": "error",
    // suggestions
    "accessor-pairs": [
        "error",
        {
            enforceForClassMembers: true,
        },
    ],
    "block-scoped-var": "error",
    "default-case-last": "error",
    "dot-notation": "error",
    eqeqeq: ["error", "always", { null: "ignore" }],
    "grouped-accessor-pairs": ["error", "getBeforeSet"],
    "logical-assignment-operators": [
        "error",
        "always",
        {
            enforceForIfStatements: true,
        },
    ],
    "no-caller": "error",
    "no-case-declarations": "error",
    "no-delete-var": "error",
    "no-else-return": "error",
    "no-empty": "error",
    "no-empty-static-block": "error",
    "no-extend-native": "error",
    "no-extra-bind": "error",
    "no-extra-boolean-cast": "error",
    "no-extra-label": "error",
    "no-global-assign": "error",
    "no-implied-eval": "error",
    "no-label-var": "error",
    "no-lonely-if": "error",
    "no-multi-str": "error",
    "no-nonoctal-decimal-escape": "error",
    "no-redeclare": "off",
    "no-regex-spaces": "error",
    "no-return-assign": ["error", "except-parens"],
    "no-sequences": "error",
    "no-shadow-restricted-names": "error",
    "no-throw-literal": "error",
    "no-unneeded-ternary": "error",
    "no-unused-labels": "error",
    "no-useless-call": "error",
    "no-useless-catch": "error",
    "no-useless-computed-key": "error",
    "no-useless-concat": "error",
    "no-useless-escape": "error",
    "no-useless-rename": "error",
    "no-with": "error",
    "object-shorthand": "error",
    "operator-assignment": ["error", "always"],
    "prefer-const": [
        "error",
        {
            destructuring: "all",
        },
    ],
    "prefer-exponentiation-operator": "error",
    "prefer-numeric-literals": "error",
    "prefer-object-has-own": "error",
    "prefer-object-spread": "error",
    "prefer-promise-reject-errors": "error",
    "prefer-regex-literals": [
        "error",
        {
            disallowRedundantWrapping: true,
        },
    ],
    "prefer-rest-params": "error",
    "prefer-spread": "error",
    "prefer-template": "error",
    "require-yield": "error",
    yoda: ["error", "never"],
    "func-style": ["error", "declaration"],
    // was @typescript-eslint/prefer-destructuring; oxlint folds into the core rule
    "prefer-destructuring": [
        "error",
        {
            VariableDeclarator: {
                object: true,
                array: true,
            },
            AssignmentExpression: {
                object: false,
                array: false,
            },
        },
        {
            enforceForRenamedProperties: false,
        },
    ],
};

// Rules ported from @typescript-eslint/* -> oxlint `typescript/*` (or core where
// oxlint exposes them without the plugin prefix).
const TSLintRules: DummyRuleMap = {
    "typescript/adjacent-overload-signatures": "error",
    "typescript/array-type": "error",
    "typescript/class-literal-property-style": ["error", "fields"],
    "typescript/consistent-generic-constructors": ["error", "constructor"],
    "typescript/consistent-type-definitions": ["error", "interface"],
    "typescript/consistent-type-exports": "error",
    "typescript/consistent-type-imports": [
        "deny",
        {
            fixStyle: "inline-type-imports",
            // needed for `const foo: typeof import("foo") = someRuntimeExpr();`
            disallowTypeAnnotations: false,
        },
    ],
    // consider typescript/explicit-member-accessibility
    "typescript/method-signature-style": ["warn", "method"],
    "typescript/no-confusing-non-null-assertion": "error",
    "typescript/no-confusing-void-expression": "warn",
    "typescript/no-deprecated": "warn",
    "typescript/no-extraneous-class": "error",
    "typescript/no-import-type-side-effects": "error",
    "typescript/no-inferrable-types": "error",
    "typescript/no-invalid-void-type": "warn",
    "typescript/no-misused-promises": [
        "warn",
        {
            checksVoidReturn: {
                arguments: false,
                attributes: false,
            },
        },
    ],
    "typescript/no-mixed-enums": "error",
    "typescript/no-non-null-asserted-nullish-coalescing": "warn",
    "typescript/no-unnecessary-boolean-literal-compare": "warn",
    "typescript/no-unnecessary-condition": "warn",
    "typescript/no-unnecessary-qualifier": "warn",
    "typescript/no-unnecessary-template-expression": "warn",
    "typescript/no-unnecessary-type-arguments": "warn",
    "typescript/no-unnecessary-type-assertion": "warn",
    "typescript/no-unnecessary-type-constraint": "warn",
    "typescript/no-unnecessary-type-conversion": "warn",
    "typescript/non-nullable-type-assertion-style": "warn",
    "typescript/only-throw-error": [
        "warn",
        {
            allow: [
                {
                    from: "package",
                    package: "@tanstack/react-router",
                    name: ["NotFoundError", "Redirect"],
                },
            ],
        },
    ],
    "typescript/prefer-find": "warn",
    "typescript/prefer-for-of": "warn",
    "typescript/prefer-function-type": "error",
    "typescript/prefer-includes": "warn",
    "typescript/prefer-optional-chain": "warn",
    "typescript/prefer-promise-reject-errors": "warn",
    "typescript/prefer-readonly": "warn",
    "typescript/prefer-reduce-type-parameter": "warn",
    "typescript/prefer-return-this-type": "warn",
    "typescript/prefer-ts-expect-error": "warn",
    "typescript/related-getter-setter-pairs": "warn",
    "typescript/restrict-plus-operands": "warn",
    "typescript/strict-boolean-expressions": "off",
    // core in oxlint
    "no-use-before-define": [
        "error",
        {
            ignoreTypeReferences: true,
            functions: false,
        },
    ],
    "typescript/require-await": "error",
    // core in oxlint
    "default-param-last": "error",
    "typescript/switch-exhaustiveness-check": [
        "error",
        {
            allowDefaultCaseForExhaustiveSwitch: false,
            requireDefaultForNonUnion: true,
            considerDefaultExhaustiveForUnions: false,
        },
    ],
    "typescript/consistent-type-assertions": [
        "error",
        {
            assertionStyle: "as",
        },
    ],
    "typescript/triple-slash-reference": "off",
    // triggers on react prop destructuring methods
    "typescript/unbound-method": "off",
    // react-sprint makes everything return a promise, gets annoying when launching animations
    "typescript/no-floating-promises": "off",
};

const unicornRules: DummyRuleMap = {
    "unicorn/consistent-existence-index-check": "error",
    "unicorn/consistent-function-scoping": "error",
    "unicorn/consistent-template-literal-escape": "error",
    "unicorn/custom-error-definition": "error",
    "unicorn/no-anonymous-default-export": "error",
    "unicorn/no-array-reverse": "warn",
    "unicorn/no-array-sort": "warn",
    "unicorn/prefer-node-protocol": "error",
};

const styleRules: RuleMap = {
    "@stylistic/array-bracket-newline": [
        "error",
        {
            multiline: true,
        },
    ],
    "@stylistic/array-bracket-spacing": ["error", "never"],
    "@stylistic/array-element-newline": [
        "error",
        {
            consistent: true,
            multiline: true,
        },
    ],
    "@stylistic/arrow-parens": ["error", "always"],
    "@stylistic/arrow-spacing": [
        "error",
        {
            before: true,
            after: true,
        },
    ],
    "@stylistic/block-spacing": ["error", "always"],
    "@stylistic/brace-style": ["error", "1tbs"],
    "@stylistic/comma-dangle": ["error", "always-multiline"],
    "@stylistic/comma-spacing": ["error"],
    "@stylistic/comma-style": ["error", "last"],
    "@stylistic/computed-property-spacing": ["error", "never"],
    "@stylistic/curly-newline": [
        "error",
        {
            consistent: true,
        },
    ],
    "@stylistic/dot-location": ["error", "property"],
    "@stylistic/eol-last": ["error", "always"],
    "@stylistic/function-call-spacing": ["error", "never"],
    "@stylistic/function-call-argument-newline": ["error", "consistent"],
    "@stylistic/function-paren-newline": ["error", "multiline"],
    "@stylistic/generator-star-spacing": [
        "error",
        {
            before: true,
            after: false,
        },
    ],
    "@stylistic/implicit-arrow-linebreak": ["error", "beside"],
    "@stylistic/indent": [
        "error",
        4,
        {
            // ton of overrides available
            SwitchCase: 1,
        },
    ],
    "@stylistic/indent-binary-ops": ["error", 2],
    "@stylistic/key-spacing": [
        "error",
        {
            beforeColon: false,
            afterColon: true,
            mode: "strict",
        },
    ],
    "@stylistic/keyword-spacing": [
        "error",
        {
            before: true,
            after: true,
        },
    ],
    "@stylistic/line-comment-position": ["off"],
    // done by git autocrlf
    "@stylistic/linebreak-style": ["off"],
    "@stylistic/lines-around-comment": ["off"],
    "@stylistic/lines-between-class-members": [
        "error",
        {
            enforce: [
                {
                    blankLine: "always",
                    prev: "*",
                    next: "*",
                },
                {
                    blankLine: "always",
                    prev: "*",
                    next: "method",
                },
                {
                    blankLine: "always",
                    prev: "method",
                    next: "*",
                },
                {
                    blankLine: "always",
                    prev: "field",
                    next: "*",
                },
                {
                    blankLine: "always",
                    prev: "*",
                    next: "field",
                },
                {
                    blankLine: "never",
                    prev: "field",
                    next: "field",
                },
            ],
        },
        {
            exceptAfterOverload: true,
        },
    ],
    "@stylistic/max-len": [
        "error",
        {
            code: 120,
            ignoreComments: true,
            ignoreUrls: true,
            ignoreStrings: true,
            ignoreTemplateLiterals: true,
            ignoreRegExpLiterals: true,
        },
    ],
    "@stylistic/max-statements-per-line": ["off"],
    "@stylistic/member-delimiter-style": [
        "error",
        {
            multiline: {
                delimiter: "semi",
                requireLast: true,
            },
            singleline: {
                delimiter: "semi",
                requireLast: true,
            },
            multilineDetection: "brackets",
        },
    ],
    // messes with editor comment hotkeys
    "@stylistic/multiline-comment-style": ["off"],
    "@stylistic/multiline-ternary": ["error", "always-multiline"],
    "@stylistic/new-parens": ["error", "always"],
    "@stylistic/newline-per-chained-call": [
        "error",
        {
            ignoreChainWithDepth: 2,
        },
    ],
    "@stylistic/no-confusing-arrow": [
        "error",
        {
            allowParens: true,
        },
    ],
    "@stylistic/no-extra-parens": [
        "error",
        "all",
        {
            // things like (foo && bar) || baz || qux
            nestedBinaryExpressions: false,
            enforceForArrowConditionals: false,
            returnAssign: false,
            conditionalAssign: false,
            ignoreJSX: "multi-line",
        },
    ],
    "@stylistic/no-extra-semi": ["error"],
    "@stylistic/no-floating-decimal": ["error"],
    "@stylistic/no-mixed-operators": ["error"],
    "@stylistic/no-mixed-spaces-and-tabs": ["error"],
    "@stylistic/no-multi-spaces": ["error"],
    "@stylistic/no-multiple-empty-lines": [
        "error",
        {
            max: 2,
            maxBOF: 0,
            maxEOF: 1,
        },
    ],
    "@stylistic/no-tabs": ["error"],
    "@stylistic/no-trailing-spaces": [
        "error",
        {
            // jsdoc 2 spaces for a linebreak
            ignoreComments: true,
        },
    ],
    "@stylistic/no-whitespace-before-property": ["error"],
    "@stylistic/nonblock-statement-body-position": ["error", "below"],
    "@stylistic/object-curly-newline": [
        "error",
        {
            // destructring assingment
            ObjectPattern: {
                multiline: true,
                consistent: true,
            },
            ObjectExpression: {
                consistent: true,
                multiline: true,
                minProperties: 4,
            },
            ImportDeclaration: {
                multiline: true,
            },
            ExportDeclaration: "always",
        },
    ],
    "@stylistic/object-curly-spacing": ["error", "always"],
    "@stylistic/object-property-newline": [
        "error",
        {
            // FIXME: no way to limit total number of props before a linebreak is needed other than one
            allowAllPropertiesOnSameLine: false,
        },
    ],
    "@stylistic/one-var-declaration-per-line": ["error", "initializations"],
    "@stylistic/operator-linebreak": ["error", "before"],
    "@stylistic/padded-blocks": ["error", "never"],
    "@stylistic/padding-line-between-statements": [
        "error",
        ...function (): PaddingSchema {
            const tsTypes = ["enum", "interface", "type"] as const;
            const varTypes = ["var", "let", "const"] as const;
            const singlelineVar = varTypes.map((x) => `singleline-${x}` as const);
            const multilineVar = varTypes.map((x) => `multiline-${x}` as const);

            function makeVariableDecPadding(): PaddingSchema {
                return [
                    // add a line before and after variable blocks of variables
                    withInverse({
                        blankLine: "always",
                        prev: "*",
                        next: varTypes,
                    }),
                    // no lines within groups of single line variable declarations
                    {
                        blankLine: "never",
                        prev: singlelineVar,
                        next: singlelineVar,
                    } as const,
                    // multiline variable declarations will always be padded with newlines
                    withInverse({
                        blankLine: "always",
                        prev: multilineVar,
                        next: "*",
                    }),
                ].flat(99);
            }

            function makeFunctionPadding(): PaddingSchema {
                return [
                    // overloads
                    {
                        blankLine: "always",
                        prev: "*",
                        next: "function-overload",
                    },
                    {
                        blankLine: "never",
                        prev: "function-overload",
                        next: ["function-overload", "function"],
                    },
                ];
            }

            function makeTypescriptPadding(): PaddingSchema {
                return withInverse({
                    blankLine: "always",
                    prev: "*",
                    next: tsTypes,
                });
            }

            type ElementType<T> = T extends (infer U)[] ? U : never;

            type PaddingElement = ElementType<PaddingSchema>;

            // util
            function withInverse(rule: PaddingElement): [PaddingElement, PaddingElement] {
                const { prev, next } = rule;

                return [
                    rule,
                    {
                        blankLine: rule.blankLine,
                        prev: next,
                        next: prev,
                    },
                ];
            }

            return [
                makeVariableDecPadding(),
                makeFunctionPadding(),
                makeTypescriptPadding(),
            ].flat(99);
        }().flat() /* doesn't include readonly arrays for some reason */,
    ],
    "@stylistic/quote-props": ["error", "as-needed"],
    "@stylistic/quotes": [
        "error",
        "double",
        {
            avoidEscape: true,
            allowTemplateLiterals: "avoidEscape",
        },
    ],
    "@stylistic/rest-spread-spacing": ["error", "never"],
    "@stylistic/semi": ["error", "always"],
    "@stylistic/semi-spacing": ["error"],
    "@stylistic/semi-style": ["error", "last"],
    "@stylistic/space-before-blocks": ["error", "always"],
    "@stylistic/space-before-function-paren": [
        "error",
        {
            anonymous: "always",
            named: "never",
            asyncArrow: "always",
        },
    ],
    "@stylistic/space-in-parens": ["error", "never"],
    "@stylistic/space-infix-ops": ["error"],
    "@stylistic/space-unary-ops": ["error"],
    "@stylistic/spaced-comment": [
        "error",
        "always",
        {
            exceptions: [
                "/",
                "#",
                "!",
                "@",
                "*",
                // template literal tags
                "js",
                "css",
            ],
        },
    ],
    "@stylistic/switch-colon-spacing": ["error"],
    "@stylistic/template-curly-spacing": ["error", "never"],
    "@stylistic/template-tag-spacing": ["error"],
    "@stylistic/type-annotation-spacing": [
        "error",
        {
            after: true,
            before: false,
            overrides: {
                arrow: {
                    after: true,
                    before: true,
                },
            },
        },
    ],
    "@stylistic/type-generic-spacing": ["error"],
    "@stylistic/type-named-tuple-spacing": ["error"],
    "@stylistic/wrap-regex": ["off"],
    "@stylistic/yield-star-spacing": ["error"],
    "@stylistic/jsx-closing-bracket-location": ["warn", "tag-aligned"],
    "@stylistic/jsx-closing-tag-location": ["warn", "tag-aligned"],
    "@stylistic/jsx-curly-brace-presence": [
        "error",
        {
            children: "never",
            props: "never",
            propElementValues: "always",
        },
    ],
    "@stylistic/jsx-curly-newline": ["error", "consistent"],
    "@stylistic/jsx-curly-spacing": [
        "error",
        {
            when: "never",
        },
    ],
    "@stylistic/jsx-equals-spacing": ["error", "never"],
    "@stylistic/jsx-first-prop-new-line": ["error", "multiline-multiprop"],
    "@stylistic/jsx-function-call-newline": ["error", "multiline"],
    "@stylistic/jsx-indent-props": ["error", 4],
    "@stylistic/jsx-max-props-per-line": [
        "error",
        {
            maximum: 1,
            when: "always",
        },
    ],
    "@stylistic/jsx-pascal-case": ["error"],
    "@stylistic/jsx-wrap-multilines": [
        "error",
        {
            arrow: "parens-new-line",
            assignment: "parens-new-line",
            declaration: "parens-new-line",
            condition: "parens-new-line",
            logical: "parens-new-line",
            prop: "parens-new-line",
            return: "parens-new-line",
            propertyValue: "parens-new-line",
        },
    ],
    "@stylistic/jsx-self-closing-comp": [
        "error",
        {
            html: true,
            component: true,
        },
    ],
};

// @eslint-react/* rules split across oxlint's react-x / react-dom / react-rsc /
// react-web-api / react-naming-convention JS plugins (rule prefixes below).
// react-hooks/* (React Compiler family) are covered by native `react/react-compiler`.
// TODO: move to oxcs react rules where possible
const eslintReactRules: DummyRuleMap = {
    // react-x rules
    "react-x/exhaustive-deps": "allow", // done with react/exhaustive-effect-dependencies
    "react-x/rules-of-hooks": "error",
    "react-x/set-state-in-effect": "off", // too noisy
    "react-x/unsupported-syntax": "warn",
    "react-x/no-nested-component-definitions": "error",
    "react-x/use-memo": "error",
    "react-x/component-hook-factories": "error",
    "react-x/error-boundaries": "error",
    "react-x/jsx-dollar": "warn",
    "react-x/jsx-key-before-spread": "warn",
    "react-x/jsx-no-comment-textnodes": "warn",
    "react-x/jsx-no-duplicate-props": "error",
    "react-x/jsx-shorthand-boolean": "warn",
    "react-x/jsx-shorthand-fragment": "error",
    "react-x/no-array-index-key": "error",
    "react-x/no-children-prop": "warn",
    "react-x/no-context-provider": "warn",
    "react-x/no-forward-ref": "warn",
    // type-aware (💭) react-x rules cannot run through oxlint's JS-plugin layer
    // yet (no parser type services in JS plugins) -> disabled to avoid crashes.
    "react-x/no-implicit-key": "off",
    "react-x/no-leaked-conditional-rendering": "off",
    "react-x/no-missing-component-display-name": "error",
    "react-x/no-missing-context-display-name": "error",
    "react-x/no-missing-key": "error",
    "react-x/no-misused-capture-owner-stack": "error",
    "react-x/no-unnecessary-use-callback": "error",
    "react-x/no-unnecessary-use-memo": "error",
    "react-x/no-unnecessary-use-prefix": "error",
    "react-x/no-unstable-context-value": "error",
    "react-x/no-unstable-default-props": "error",
    // TODO: add
    // "react-x/no-unused-props": "warn",
    "react-x/no-use-context": "error",
    "react-x/no-useless-fragment": "error",
    "react-x/prefer-destructuring-assignment": "error",
    "react-x/prefer-namespace-import": "error",
    "react-x/set-state-in-render": "error",
    "react-x/use-state": "error",
    // react-rsc
    "react-rsc/function-definition": "error",
    // react-dom
    "react-dom/no-dangerously-set-innerhtml": "error",
    "react-dom/no-dangerously-set-innerhtml-with-children": "error",
    "react-dom/no-missing-iframe-sandbox": "error",
    "react-dom/no-namespace": "error",
    "react-dom/no-string-style-prop": "error",
    "react-dom/no-unknown-property": "warn",
    "react-dom/no-unsafe-iframe-sandbox": "error",
    "react-dom/no-unsafe-target-blank": "error",
    "react-dom/no-void-elements-with-children": "error",
    // react-web-api
    "react-web-api/no-leaked-event-listener": "error",
    "react-web-api/no-leaked-interval": "error",
    "react-web-api/no-leaked-resize-observer": "error",
    "react-web-api/no-leaked-timeout": "error",
    // react-naming-contention
    "react-naming-convention/context-name": "error",
    "react-naming-convention/id-name": "warn",
    "react-naming-convention/ref-name": "error",
    // compiler rules
    "react-hooks/exhaustive-deps": "allow", // done with react/exhaustive-effect-dependencies
    "react/error-boundaries": "error",
    "react/exhaustive-effect-dependencies": "error",
    "react/globals": "error",
    "react/preserve-manual-memoization": "error",
    "react/incompatible-library": "warn",
    "react/todo": "warn",
    "react/syntax": "error",
    "react/immutability": "error",
    "react/refs": "error",
    "react/purity": "error",
    "react/set-state-in-render": "error",
    // way too noisy and it's often fine
    "react/set-state-in-effect": "allow",
    "react/only-export-components": [
        "warn",
        {
            allowConstantExport: true,
            customHOCs: ["createFileRoute", "createRootRoute", "animated", "createLink"],
        },
    ],
};

const oxcRules: DummyRuleMap = {
    "oxc/approx-constant": "warn",
    // false positives when creating bitmasks from enums
    "oxc/bad-bitwise-operator": "off",
    "oxc/branches-sharing-code": "warn",
    "oxc/misrefactored-assign-op": "warn",
    "oxc/no-accumulating-spread": "warn",
    "oxc/no-barrel-file": "warn",
    // oftentimes the tersest option
    "oxc/no-map-spread": "off",
    "oxc/no-this-in-exported-function": "error",
};

const extensions = "{js,mjs,cjs,jsx,mjsx,cjsx,ts,mts,cts,tsx,mtsx,ctsx}";
const tailwindConfig = join(__dirname, "src", "index.css");

const tailwindCallees = Object.freeze({
    callees: ["classnames", "clsx", "ctl", "cva", "tv", "cn"],
    config: tailwindConfig,
});

// TODO: re-add stories when storybook is re-added
export default defineConfig({
    plugins: ["react", "typescript", "import", "unicorn", "oxc"],
    options: {
        typeAware: true,
    },
    ignorePatterns: [
        "dist",
        "crates",
        "src/**/*.stories.tsx",
        "dist.server",
        "builds",
        "node_modules",
        ".vite-inspect",
        ".wrangler",
        // subpackage has its own separate tooling; out of scope for root lint
        "packages",
        // vendored upstream vscode-textmate source (MIT); not ours to lint
        "src/components/CodeEditor/Monaco/vscode-textmate",
    ],
    settings: {
        // ported from the ESLint `react-x` settings block
        "react-x": {
            additionalEffectHooks: "(useIsomorphicLayoutEffect)",
            polymorphicPropName: "tag",
            compilationMode: "infer",
        },
    },
    extends: [
        {
            plugins: ["react"],
            categories: {
                correctness: "error",
            },
        },
    ],
    overrides: [
        {
            files: [
                `src/**/*.${extensions}`,
                `server/**/*.${extensions}`,
                `eslint.config.${extensions}`,
                `oxlint.config.${extensions}`,
                `vite.config.${extensions}`,
                `stylelint.config.${extensions}`,
                `scripts/**/*.${extensions}`,
                `vitest.config.${extensions}`,
                `.storybook/*.${extensions}`,
            ],
            jsPlugins: [
                "@stylistic/eslint-plugin",
                "eslint-plugin-react-x",
                "eslint-plugin-react-rsc",
                "eslint-plugin-react-dom",
                "eslint-plugin-react-web-api",
                "eslint-plugin-react-naming-convention",
                "eslint-plugin-unused-imports",
                "eslint-plugin-simple-import-sort",
                "eslint-plugin-tailwindcss",
                // local custom rule (require-css-as-namespace)
                "./scripts/oxlintLocalPlugin.ts",
            ],
            rules: {
                ...ESLintRules,
                ...TSLintRules,
                ...unicornRules,
                // Style Rules
                ...styleRules,
                ...eslintReactRules,
                ...oxcRules,
                "unused-imports/no-unused-imports": "error",
                "unused-imports/no-unused-vars": [
                    "warn",
                    {
                        vars: "all",
                        varsIgnorePattern: "^_",
                        args: "after-used",
                        argsIgnorePattern: "^_",
                    },
                ],
                // Plugin Rules
                "simple-import-sort/imports": [
                    "error",
                    {
                        groups: [
                            ["^@.+$"],
                            ["^\\./(?=.*/)(?!/?$)", "^\\.(?!/?$)", "^\\./?$", "^\\.\\.(?!/?$)", "^\\.\\./?$"],
                            ["^(assert|buffer|child_process|cluster|console|constants|crypto|dgram|dns|domain|events|fs|http|https|module|net|os|path|punycode|querystring|readline|repl|stream|string_decoder|sys|timers|tls|tty|url|util|vm|zlib|freelist|v8|process|async_hooks|http2|perf_hooks)(/.*|$)"],
                        ],
                    },
                ],
                "simple-import-sort/exports": "error",
                "tailwindcss/classnames-order": [
                    "error",
                    tailwindCallees,
                ],
                "tailwindcss/enforces-negative-arbitrary-values": ["error", tailwindCallees],
                "tailwindcss/enforces-shorthand": ["error", tailwindCallees],
                // maybe add no-arbitrary-value
                "tailwindcss/no-contradicting-classname": ["error", tailwindCallees],
                // not yet working in the beta
                // "tailwindcss/no-custom-classname": ["error", tailwindCallees],
                "tailwindcss/no-unnecessary-arbitrary-value": ["error", tailwindCallees],
                "local/require-css-as-namespace": "error",
            },
        },
        {
            files: [
                "**/*.stories.@(ts|tsx|js|jsx|mjs|cjs)",
                "**/*.story.@(ts|tsx|js|jsx|mjs|cjs)",
            ],
            jsPlugins: ["eslint-plugin-storybook"],
            rules: {
                "storybook/await-interactions": "error",
                "storybook/context-in-play-function": "error",
                "storybook/default-exports": "error",
                "storybook/hierarchy-separator": "warn",
                "storybook/no-redundant-story-name": "warn",
                "storybook/no-renderer-packages": "error",
                "storybook/prefer-pascal-case": "warn",
                "storybook/story-exports": "error",
                "storybook/use-storybook-expect": "error",
                "storybook/use-storybook-testing-library": "error",
                "import/no-anonymous-default-export": "off",
                "react/rules-of-hooks": "off",
            },
        },
        {
            files: [".storybook/main.@(js|cjs|mjs|ts)"],
            jsPlugins: ["eslint-plugin-storybook"],
            rules: {
                "storybook/no-uninstalled-addons": "error",
            },
        },
    ],
});
