// Synthetic component suite only. Native IPC and file writes have separate desktop acceptance.
export function isTauri(){return false}
export async function invoke(){throw Error('Native IPC is deliberately unavailable in this browser component fixture')}
