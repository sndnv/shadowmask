import { Api, sleep, type Version } from "./api.ts";
import { OUT, writePrepared } from "./results.ts";

const WAIT_KINDS = new Set(["library_scan", "scheduled_scan", "ingest", "relink", "playback_prep"]);
const library = Deno.env.get("PLAYBACK_LIBRARY") === "1";
const timeoutMs = Number(Deno.env.get("PLAYBACK_PREPARE_TIMEOUT_S") ?? "1800") * 1000;

async function settle(api: Api): Promise<void> {
  const until = Date.now() + timeoutMs;
  let quiet = 0;
  let reported = -1;
  while (quiet < 3) {
    const jobs = await api.activeJobs();
    for (const job of jobs.filter((j) => j.kind === "trickplay" && j.cancellable)) {
      await api.cancel(job.id).catch(() => undefined);
    }
    const busy = jobs.filter((j) => WAIT_KINDS.has(j.kind));
    if (busy.length !== reported) {
      console.log(`waiting for [${busy.length}] scan and playback prep job(s)`);
      reported = busy.length;
    }
    quiet = busy.length === 0 ? quiet + 1 : 0;
    if (Date.now() > until) {
      throw new Error(`scan and playback prep did not finish within ${timeoutMs / 1000} s`);
    }
    await sleep(2000);
  }
}

const api = await Api.login();
for (const lib of await api.libraries()) {
  await api.scan(lib.id).catch((err) => {
    if (!String(err).includes("[409]")) {
      throw err;
    }
  });
}
await settle(api);

const versions: Version[] = [];
for (const lib of await api.libraries()) {
  for (const listed of await api.libraryVersions(lib.id)) {
    versions.push(await api.version(listed.id));
  }
}
const labels: Record<string, string> = library ? JSON.parse(await Deno.readTextFile(`${OUT}/samples.json`)) : {};
await writePrepared({ library, versions, labels });
console.log(`prepared [${versions.length}] version(s)`);
