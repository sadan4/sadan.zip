/**                                                                                                                                                                           
 * @param text the module text                                                                                                                                                
 * @returns if the module text is a webpack module or an extracted find                                                                                                       
 */
export function isWebpackModule(text: string): boolean {
    return text.startsWith("// Webpack Module ")
      || text.substring(0, 100)
          .includes("//OPEN FULL MODULE:");
}
