import { namedContext } from "@/utils/devtools";
import { isMobileDevice } from "@/utils/dom";

export const CustomCursorContext = namedContext<true | undefined>(isMobileDevice() || undefined, "CustomCursorContext");
