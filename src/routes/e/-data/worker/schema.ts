import { TModuleId } from "@/utils/types";

import { z } from "zod";

export const Span = z.object({
    start: z.int32(),
    end: z.int32(),
}).strict();
export type Span = z.infer<typeof Span>;

export const ClassName = z.object({
    module: TModuleId,
    exported_as: z.string(),
    span: Span,
    value: z.string(),
}).strict();
export type ClassName = z.infer<typeof ClassName>;

export const CssModule = z.object({
    class_names: ClassName.array(),
}).strict();
export type CssModule = z.infer<typeof CssModule>;
