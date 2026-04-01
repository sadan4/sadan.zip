import definePlugin, { OptionType } from "@utils/types";
// import { ELEMENT_ID } from "./constants";
// The actual file uses the import from "./constants"; however,
// if this ever gets merged, the definition should be moved to the main plugin file
const ELEMENT_ID = "vc-imgzoom-magnify-modal";

export default definePlugin({
    name: "Plugin3",
    authors: [],

    patches: [
        {
            find: "disableArrowKeySeek:!0",
            replacement: [
                {
                    match: /useFullWidth:!0,shouldLink:/,
                    replace: `id:"${ELEMENT_ID}",$&`
                },
            ]
        },
    ],
});
