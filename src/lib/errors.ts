/** The text to show for anything thrown or rejected, including Rust command errors (strings). */
export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
