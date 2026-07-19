// Oxlint JS-plugin wrapper for the repo's local ESLint rules.
// Exposes `require-css-as-namespace` under the `local` plugin namespace,
// matching the old ESLint flat-config `local/require-css-as-namespace` key.
import requireCssAsNamespace from "./requireCssAsNamespace.ts";

export default {
    meta: {
        name: "local",
    },
    rules: {
        "require-css-as-namespace": requireCssAsNamespace,
    },
};
