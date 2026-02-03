#!/usr/bin/env zsh

set -x
set -eo pipefail

# https://unix.stackexchange.com/questions/76505/unix-portable-way-to-get-scripts-absolute-path-in-zsh
pushd ${0:a:h}

commonEsbuildArgs=(
    --sourcemap=linked
    --platform=node
    --format=cjs
    --tsconfig=../tsconfig.json
    # doesn't play nice with bundling
    --external:jsdom
    --banner:js='#!/usr/bin/env node'
    --bundle
)
baseDistDir=../dist.server

rm -r ${baseDistDir}/* || :

mainOutPath=${baseDistDir}/index.cjs
watcherOutPath=${baseDistDir}/watcher.cjs
parserWorkerOutPath=${baseDistDir}/parserWorker.cjs

esbuild ${commonEsbuildArgs} --outfile=${mainOutPath} --metafile=${baseDistDir}/index.meta.json ./index.ts

esbuild ${commonEsbuildArgs} --outfile=${watcherOutPath} --metafile=${baseDistDir}/watcher.meta.json ./watcher.ts

esbuild ${commonEsbuildArgs} --outfile=${parserWorkerOutPath} --metafile=${baseDistDir}/parserWorker.meta.json ./parserWorker.ts

chmod +x ${mainOutPath}
chmod +x ${watcherOutPath}

popd
