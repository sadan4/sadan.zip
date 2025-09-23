module.exports = (dts, { classes, filename, logger }) => {
    return [
      '/* eslint-disable */',
      dts,
      'export const code: string;',
    ].join('\n');
}
