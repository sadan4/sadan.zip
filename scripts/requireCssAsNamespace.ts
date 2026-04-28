import { ESLintUtils, TSESLint, TSESTree as N } from "@typescript-eslint/utils";

const createRule = ESLintUtils.RuleCreator((_name: string) => import.meta.filename);

export default createRule({
    create(ctx) {
        return {
            "ImportDeclaration:has(ImportDefaultSpecifier.specifiers)"(node: N.ImportDeclaration) {
                const defaultImport = node.specifiers.find((s: N.ImportClause) => s.type === "ImportDefaultSpecifier")!;

                if (node.source.value.match(/\.s?css$/)) {
                    ctx.report({
                        node: defaultImport,
                        messageId: "namespace",
                        suggest: [
                            {
                                messageId: "convert",
                                fix(fixer: TSESLint.RuleFixer) {
                                    return fixer.insertTextBefore(defaultImport, "* as ");
                                },
                            },
                        ],
                    });
                }
            },
        };
    },
    name: "require-css-as-namespace",
    meta: {
        docs: {
            description: "Require CSS modules to be imported as namespaces for better tree shaking",
        },
        messages: {
            namespace: "CSS modules should be imported as namespaces to ensure proper tree shaking.",
            convert: "Convert this to a namespace import",
        },
        type: "problem",
        schema: [],
        hasSuggestions: true,
    },
    defaultOptions: [],
});

