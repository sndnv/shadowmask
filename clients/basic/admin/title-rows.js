const KINDS = {
  movie: { label: "Movie", path: "movies" },
  series: { label: "Series", path: "series" },
  season: { label: "Season", path: "seasons" },
  episode: { label: "Episode", path: "episodes" },
};

const CHOICES = {
  kind: Object.keys(KINDS),
  availability: ["available", "unavailable", "no_version"],
  sort: ["title", "kind", "year", "versions", "added"],
  order: ["asc", "desc"],
};

const eu = encodeURIComponent;

function count(n, one, many) {
  return n === 1 ? "1 " + one : n + " " + many;
}

function seasonLabel(r) {
  return "Season " + (r.season_number ?? 0);
}

export function rowName(r, episodeLine) {
  if (r.kind === "season") return r.title || seasonLabel(r);
  if (r.kind === "episode") return episodeLine(r.season_number, r.episode_number ?? 0, r.title);
  return r.title || r.id;
}

export function rowContext(r) {
  if (!r.series_title) return null;
  if (r.kind === "season") return r.series_title;
  if (r.kind === "episode") return r.series_title + " · " + seasonLabel(r);
  return null;
}

export function fullName(r, episodeLine) {
  const name = rowName(r, episodeLine);
  const child = r.kind === "season" || r.kind === "episode";
  return child && r.series_title ? r.series_title + " · " + name : name;
}

export function kindLabel(kind) {
  return KINDS[kind].label;
}

export function versionsText(r) {
  if (r.episodes == null) return count(r.files, "version", "versions");
  if (r.episodes === 0) return "No episodes";
  return r.files + " of " + count(r.episodes, "episode", "episodes");
}

function remaining(verb, n, one, many) {
  return n ? verb + " its " + count(n, "remaining " + one, "remaining " + many) + " first." : null;
}

export function deleteBlock(r) {
  if (r.kind === "series") return remaining("Delete", r.seasons, "season", "seasons");
  if (r.kind === "season") return remaining("Delete", r.episodes, "episode", "episodes");
  return remaining("Remove", r.files, "version", "versions");
}

export function deletePath(r) {
  return KINDS[r.kind].path;
}

export function rowTone(r) {
  if (r.available) return null;
  return r.files ? "sm-row-unavailable" : "sm-row-no-version";
}

export function rowHref(r, href) {
  if (!r.readable) return null;
  if (r.kind === "movie") return href.movie(r.id);
  if (r.kind === "series") return href.series(r.id);
  if (r.kind === "season") return href.season(r.id, r.series_id);
  return href.episode(r.id, r.season_id, r.series_id);
}

export function listsVersions(r) {
  return r.readable && (r.kind === "movie" || r.kind === "episode");
}

export function versionsPath(r) {
  if (r.kind === "movie") return "/api/v1/movies/" + eu(r.id) + "/versions";
  const season = "/api/v1/series/" + eu(r.series_id) + "/seasons/" + eu(r.season_id);
  return season + "/episodes/" + eu(r.id) + "/versions";
}

export function titlesQuery(search) {
  const from = new URLSearchParams(search);
  const params = new URLSearchParams();
  for (const name of ["offset", "limit", "filter"]) {
    const value = from.get(name);
    if (value) params.set(name, value);
  }
  for (const [name, values] of Object.entries(CHOICES)) {
    const value = from.get(name);
    if (values.includes(value)) params.set(name, value);
  }
  const qs = params.toString();
  return qs ? "?" + qs : "";
}

export function stepBack(page) {
  if (page.items.length || page.offset === 0) return null;
  if (page.limit <= 0) return 0;
  return Math.max(0, Math.floor((page.total - 1) / page.limit) * page.limit);
}
