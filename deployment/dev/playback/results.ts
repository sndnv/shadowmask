import type { Version } from "./api.ts";
import type { Result } from "./runner.ts";

export const OUT = Deno.env.get("PLAYBACK_OUT") ?? "/out";

export type Prepared = { library: boolean; versions: Version[]; labels: Record<string, string> };

export async function loadSubjects(): Promise<Prepared> {
  return JSON.parse(await Deno.readTextFile(`${OUT}/versions.json`));
}

export async function writePrepared(prepared: Prepared): Promise<void> {
  await Deno.writeTextFile(`${OUT}/versions.json`, JSON.stringify(prepared, null, 2));
}

export async function writeResults(engine: string, results: Result[]): Promise<void> {
  await Deno.writeTextFile(`${OUT}/results-${engine}.json`, JSON.stringify(results, null, 2));
}

export async function readResults(): Promise<Result[]> {
  const results: Result[] = [];
  for await (const entry of Deno.readDir(OUT)) {
    if (entry.isFile && /^results-.*\.json$/.test(entry.name)) {
      results.push(...JSON.parse(await Deno.readTextFile(`${OUT}/${entry.name}`)));
    }
  }
  return results;
}
