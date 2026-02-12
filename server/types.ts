import { z } from "zod";

export const messageBaseSchema = z.object({
    type: z.string(),
});

const withMessageIdSchema = z.object({
    messageId: z.number(),
});

export type MessageBase = z.infer<typeof messageBaseSchema>;

export const TBundleHash = z.string().brand<"bundleHash", "inout">();

export type TBundleHash = z.infer<typeof TBundleHash>;

export const TModuleId = z.string().brand<"moduleId", "inout">();

export type TModuleId = z.infer<typeof TModuleId>;


export const queryBundlesMessageSchema = messageBaseSchema.safeExtend({
    type: z.literal("queryBundles"),
});

export type QueryBundlesMessage = z.infer<typeof queryBundlesMessageSchema>;

export const getBundleMetadataMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleMetadata"),
    bundleHash: TBundleHash,
});

export type GetBundleMetadataMessage = z.infer<typeof getBundleMetadataMessageSchema>;

export const getBundleDepGraphMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleDepGraph"),
    bundleHash: TBundleHash,
});

export type GetBundleDepGraphMessage = z.infer<typeof getBundleDepGraphMessageSchema>;

export const getAllBundleFilesMessageSchema = messageBaseSchema.extend({
    type: z.literal("getAllBundleFiles"),
    bundleHash: TBundleHash,
});

export type GetAllBundleFilesMessage = z.infer<typeof getAllBundleFilesMessageSchema>;

export const getBundleFileMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleFile"),
    bundleHash: TBundleHash,
    moduleNumber: TModuleId,
});

export type GetBundleFileMessage = z.infer<typeof getBundleFileMessageSchema>;

const baseMessageToServerSchema = z.discriminatedUnion("type", [
    queryBundlesMessageSchema,
    getBundleMetadataMessageSchema,
    getBundleDepGraphMessageSchema,
    getAllBundleFilesMessageSchema,
    getBundleFileMessageSchema,
]);

export const messageToServerSchema = z.intersection(withMessageIdSchema, baseMessageToServerSchema);

export type BaseMessageToServer = z.infer<typeof baseMessageToServerSchema>;

export type MessageToServer = z.infer<typeof messageToServerSchema>;

export const moduleInfoSchema = z.record(z.string(), z.array(TModuleId));

export type ModuleInfo = z.infer<typeof moduleInfoSchema>;

/**
 * schema for info.json
 */
export const bundleInfoSchema = z.object({
    buildHash: TBundleHash,
    buildNumber: z.string(),
    firstSeen: z.number(),
    /**
     * The entry point of the module, May be undefined on bundles parsed before this field was added
     * or if the entry point could not be found
     */
    entryPoint: TModuleId.optional(),
    modules: moduleInfoSchema,
    /**
     * can't be serialized as it contains symbols, but is cheap to parse, and guaranteed to be valid
     */
    envVarText: z.string(),
});

export const keyModulesSchema = z.object({
    /**
     * [moduleId, exportName][]
     */
    fluxDispatcherClass: z.array(z.tuple([TModuleId, /* exportName */ z.union([z.string(), z.symbol()])])),
});

export type KeyModules = z.infer<typeof keyModulesSchema>;

export const mainDepsSchema = z.record(TModuleId, z.object({
    syncUses: z.array(TModuleId),
    lazyUses: z.array(TModuleId),
}));

export type MainDeps = z.infer<typeof mainDepsSchema>;

export const depsJsonSchema = z.object({
    deps: mainDepsSchema,
    keyModules: keyModulesSchema,
});

export type DepsJson = z.infer<typeof depsJsonSchema>;

export type BundleInfo = z.infer<typeof bundleInfoSchema>;

export const bundlesResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("queryBundlesResponse"),
    bundles: z.array(bundleInfoSchema),
});

export type BundlesResponseMessage = z.infer<typeof bundlesResponseMessageSchema>;

export const allBundleFilesResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getAllBundleFilesResponse"),
    bundleHash: TBundleHash,
    files: z.record(TModuleId, z.string()),
});

export type AllBundleFilesResponseMessage = z.infer<typeof allBundleFilesResponseMessageSchema>;

export const bundleMetadataResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleMetadataResponse"),
    bundleHash: TBundleHash,
    metadata: bundleInfoSchema,
});

export type BundleMetadataResponseMessage = z.infer<typeof bundleMetadataResponseMessageSchema>;

export const bundleDepGraphResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleDepGraphResponse"),
    bundleHash: TBundleHash,
    depGraph: depsJsonSchema,
});

export type BundleDepGraphResponseMessage = z.infer<typeof bundleDepGraphResponseMessageSchema>;

export const bundleFileResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleFileResponse"),
    bundleHash: TBundleHash,
    moduleNumber: TModuleId,
    fileText: z.string(),
});

export type BundleFileResponseMessage = z.infer<typeof bundleFileResponseMessageSchema>;


export const errorMessageSchema = messageBaseSchema.extend({
    type: z.literal("error"),
    message: z.string(),
});

export type ErrorMessage = z.infer<typeof errorMessageSchema>;

const baseMessageToClientSchema = z.discriminatedUnion("type", [
    bundlesResponseMessageSchema,
    allBundleFilesResponseMessageSchema,
    bundleMetadataResponseMessageSchema,
    bundleDepGraphResponseMessageSchema,
    bundleFileResponseMessageSchema,
    errorMessageSchema,
]);

export type BaseMessageToClient = z.infer<typeof baseMessageToClientSchema>;

export const messageToClientSchema = z.intersection(withMessageIdSchema, baseMessageToClientSchema);

export type MessageToClient = z.infer<typeof messageToClientSchema>;

