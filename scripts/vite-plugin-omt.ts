import omtPlugin from "@surma/rollup-plugin-off-main-thread";

import type { Rolldown } from "vite";

/**
 * fix issue in omt where it doesn't resolve file urls correctly
 * 
 * this broke after updating rolldown to 1.2.4 where it implemented resolveFileUrl
 * omt then started using that, causing workers to 404 because the path was resolved relative to the root instead of the importing file
 * 
 * @preview ![Workflow](https://imgs.xkcd.com/comics/workflow.png)
 */
export function omt(): Rolldown.Plugin {
    return {
        ...omtPlugin() as unknown as Rolldown.Plugin,
        resolveFileUrl({ moduleId, relativePath }) {
            // only trigger on our own workers
            if (!moduleId.startsWith("omt:")) {
                return null;
            }

            return `new URL(${JSON.stringify(relativePath)}, import.meta.url).href`;
        },
    };
}

export default omt;
