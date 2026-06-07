import { writeFile } from "node:fs/promises";

import esbuild from "esbuild";

import { commonOpts } from "./common.mts";

const IS_DEV = process.argv.includes("--dev");

const res = await esbuild.build({
    ...commonOpts,
    sourcemap: "linked",
    metafile: true,
    minify: !IS_DEV,
});

await writeFile("dist/meta.json", JSON.stringify(res.metafile));
