/** Quote one OS argument the way Windows shows it in a command line. */
export function quoteArgument(value: string): string {
  return /[\s"]/.test(value) ? `"${value.replaceAll('"', '\\"')}"` : value
}

/** Format the recorded argv as a readable, copyable command line. */
export function formatArgv(argv: readonly string[]): string {
  return argv.map(quoteArgument).join(' ')
}
