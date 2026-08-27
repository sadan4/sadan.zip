import type { SchemaBase } from "./schema";
import { createVirtualAnalyzer } from ".";

import { InternalSymbolName } from "typescript";

/**
 * @internal
 */
export function handleDefaultExport(tsCode: string): SchemaBase {
    const analyzer = createVirtualAnalyzer(tsCode);
    const defaultExportSym = analyzer.getSymbolForExportName(InternalSymbolName.Default);

    if (!defaultExportSym) {
        throw new Error("No default export found");
    }

    return analyzer.getSchemaForSymbol(defaultExportSym);
}
