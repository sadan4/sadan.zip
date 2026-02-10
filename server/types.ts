import { z } from "zod";

export const messageBaseSchema = z.object({
    type: z.string(),
});

const withMessageIdSchema = z.object({
    messageId: z.number(),
});

export type MessageBase = z.infer<typeof messageBaseSchema>;

export const queryBundlesMessageSchema = messageBaseSchema.safeExtend({
    type: z.literal("queryBundles"),
});

export type QueryBundlesMessage = z.infer<typeof queryBundlesMessageSchema>;

export const getBundleMetadataMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleMetadata"),
    bundleHash: z.string(),
});

export type GetBundleMetadataMessage = z.infer<typeof getBundleMetadataMessageSchema>;

export const getBundleDepGraphMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleDepGraph"),
    bundleHash: z.string(),
});

export type GetBundleDepGraphMessage = z.infer<typeof getBundleDepGraphMessageSchema>;

export const getAllBundleFilesMessageSchema = messageBaseSchema.extend({
    type: z.literal("getAllBundleFiles"),
    bundleHash: z.string(),
});

export type GetAllBundleFilesMessage = z.infer<typeof getAllBundleFilesMessageSchema>;

export const getBundleFileMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleFile"),
    bundleHash: z.string(),
    moduleNumber: z.string(),
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

export const moduleInfoSchema = z.record(z.string(), z.array(z.string()));

export type ModuleInfo = z.infer<typeof moduleInfoSchema>;

export const bundleInfoSchema = z.object({
    buildHash: z.string(),
    buildNumber: z.string(),
    firstSeen: z.number(),
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
    fluxDispatcherClass: z.array(z.tuple([z.string(), z.union([z.string(), z.symbol()])])),
});

export type KeyModules = z.infer<typeof keyModulesSchema>;

export const mainDepsSchema = z.record(z.string(), z.object({
    syncUses: z.array(z.string()),
    lazyUses: z.array(z.string()),
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
    bundleHash: z.string(),
    files: z.record(z.string(), z.string()),
});

export type AllBundleFilesResponseMessage = z.infer<typeof allBundleFilesResponseMessageSchema>;

export const bundleMetadataResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleMetadataResponse"),
    bundleHash: z.string(),
    metadata: bundleInfoSchema,
});

export type BundleMetadataResponseMessage = z.infer<typeof bundleMetadataResponseMessageSchema>;

export const bundleDepGraphResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleDepGraphResponse"),
    bundleHash: z.string(),
    depGraph: depsJsonSchema,
});

export type BundleDepGraphResponseMessage = z.infer<typeof bundleDepGraphResponseMessageSchema>;

export const bundleFileResponseMessageSchema = messageBaseSchema.extend({
    type: z.literal("getBundleFileResponse"),
    bundleHash: z.string(),
    moduleNumber: z.string(),
    fileText: z.string(),
});

export type BundleFileResponseMessage = z.infer<typeof bundleFileResponseMessageSchema>;


export const errorMessageSchema = messageBaseSchema.extend({
    type: z.literal("error"),
    message: z.string(),
});

export type ErrorMessage = z.infer<typeof errorMessageSchema>;

const baseMessageToCLientSchema = z.discriminatedUnion("type", [
    bundlesResponseMessageSchema,
    allBundleFilesResponseMessageSchema,
    bundleMetadataResponseMessageSchema,
    bundleDepGraphResponseMessageSchema,
    bundleFileResponseMessageSchema,
    errorMessageSchema,
]);

export type BaseMessageToClient = z.infer<typeof baseMessageToCLientSchema>;

export const messageToClientSchema = z.intersection(withMessageIdSchema, baseMessageToCLientSchema);

export type MessageToClient = z.infer<typeof messageToClientSchema>;

