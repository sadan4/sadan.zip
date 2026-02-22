import { monaco } from "@/utils/monaco";
import * as SharedRange from "@vencord-companion/shared/Range";

export function toMonacoRange(r: SharedRange.IRange) {
    return new monaco.Range(r.start.line + 1, r.start.character + 1, r.end.line + 1, r.end.character + 1);
}
