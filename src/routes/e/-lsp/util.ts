import { type Monaco, monaco } from "@/utils/monaco";
import * as SharedPosition from "@vencord-companion/shared/Position";
import * as SharedRange from "@vencord-companion/shared/Range";

export function toMonacoRange(r: SharedRange.IRange) {
    return new monaco.Range(r.start.line + 1, r.start.character + 1, r.end.line + 1, r.end.character + 1);
}

export function toParserPosition(pos: Monaco.IPosition) {
    return new SharedPosition.Position(pos.lineNumber - 1, pos.column - 1);
}

export {
    isWebpackModule,
} from "@vencord-companion/webpack-ast-parser";
