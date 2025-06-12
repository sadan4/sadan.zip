module.exports = (dts, { classes, filename, logger }) => {
    logger.log('Example log');
    return [
      '/* eslint-disable */',
      dts,
      'export const code: string;',
    ].join('\n');
}
