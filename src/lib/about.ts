/** The licence the repository declares, shown as plain text on the about page. */
export const LICENSE = 'GPL-3.0-or-later'

/** One pasteable line: an issue needs the build, not a screenshot of it. */
export function copyVersion(version: string, commit: string) {
  return `SAS Launcher ${version} (${commit})`
}
