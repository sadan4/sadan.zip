module.exports = function makeCssModuleDts(dts, { classes: _classes, filename: _filename, logger: _logger }) {
    return [
        "/* eslint-disable */",
        dts,
        "export const code: string;",
    ].join("\n");
};
