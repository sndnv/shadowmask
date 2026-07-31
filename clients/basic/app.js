const sm = (() => {
  const API = "";
  const KEY = "shadowmask.tokens";

  const load = () => {
    try {
      return JSON.parse(localStorage.getItem(KEY)) || null;
    } catch {
      return null;
    }
  };
  const save = (t) => localStorage.setItem(KEY, JSON.stringify(t));
  const clear = () => localStorage.removeItem(KEY);

  const base = (() => {
    const current = document.currentScript;
    const src = current && current.src ? current.src : location.href;
    return new URL(".", src).href;
  })();
  const url = (path) => base + path;

  async function raw(path, opts = {}, token) {
    const headers = Object.assign({ Accept: "application/json" }, opts.headers || {});
    if (opts.body !== undefined && !headers["Content-Type"]) {
      headers["Content-Type"] = "application/json";
    }
    if (token) {
      headers["Authorization"] = "Bearer " + token;
    }
    return fetch(API + path, Object.assign({}, opts, { headers }));
  }

  async function refresh() {
    const t = load();
    if (!t || !t.refresh_token) {
      throw new Error("no refresh token");
    }
    const res = await raw("/api/v1/auth/refresh", {
      method: "POST",
      body: JSON.stringify({ refresh_token: t.refresh_token }),
    });
    if (!res.ok) {
      clear();
      throw new Error("refresh failed");
    }
    const next = Object.assign({}, t, await res.json());
    save(next);
    return next.access_token;
  }

  async function api(path, opts = {}) {
    const t = load();
    let res = await raw(path, opts, t && t.access_token);
    if (res.status === 401 && t && t.refresh_token) {
      const at = await refresh();
      res = await raw(path, opts, at);
    }
    if (res.status === 401) {
      clear();
      location.href = url("index.html");
      throw new Error("unauthorized");
    }
    return res;
  }

  async function json(path, opts) {
    const res = await api(path, opts);
    if (!res.ok) {
      throw new Error("HTTP " + res.status);
    }
    if (res.status === 204) return null;
    const text = await res.text();
    return text ? JSON.parse(text) : null;
  }

  async function login(username, password) {
    const res = await raw("/api/v1/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
    if (!res.ok) {
      throw new Error("HTTP " + res.status);
    }
    save(await res.json());
  }

  async function logout() {
    const t = load();
    if (t && t.refresh_token) {
      try {
        await raw("/api/v1/auth/logout", {
          method: "POST",
          body: JSON.stringify({ refresh_token: t.refresh_token }),
        });
      } catch {}
    }
    clear();
    location.href = url("index.html");
  }

  function requireAuth() {
    if (!load()) {
      location.href = url("index.html");
      return false;
    }
    return true;
  }

  let selfCache = null;
  function self() {
    if (!selfCache) {
      selfCache = json("/api/v1/users/self").catch((e) => {
        selfCache = null;
        throw e;
      });
    }
    return selfCache;
  }

  function qs(name) {
    return new URLSearchParams(location.search).get(name);
  }

  function lang(code) {
    return !code || code === "und" ? "unknown" : code;
  }

  function time(iso) {
    if (!iso) return "";
    const parsed = new Date(iso);
    return isNaN(parsed.getTime()) ? String(iso) : parsed.toLocaleString();
  }

  async function onDeckEpisode(uid, seasonIds) {
    if (!uid) return null;
    const cont = await json(
      "/api/v1/users/" + encodeURIComponent(uid) + "/continue",
    ).catch(() => null);
    const set = new Set(seasonIds);
    return ((cont && cont.next_episodes) || []).find((e) => set.has(e.season_id)) || null;
  }

  async function versionPicker(uid, versions) {
    const list = (versions || []).slice();
    const resume = {};
    if (uid) {
      const cont = await json(
        "/api/v1/users/" + encodeURIComponent(uid) + "/continue",
      ).catch(() => null);
      const collect = (arr) =>
        (arr || []).forEach((item) => {
          if (item && item.progress && item.progress.version_id) {
            resume[item.progress.version_id] = item.card ? item.card.progress_percent : 0;
          }
        });
      if (cont) {
        collect(cont.now_playing);
        collect(cont.in_progress);
      }
    }
    const available = list.filter((v) => v.available);
    const inProgress = available.filter((v) =>
      Object.prototype.hasOwnProperty.call(resume, v.id),
    );
    let single = null;
    if (available.length === 1) single = available[0].id;
    else if (inProgress.length === 1) single = inProgress[0].id;

    const details = await Promise.all(
      list.map((v) =>
        json("/api/v1/versions/" + encodeURIComponent(v.id)).catch(() => null),
      ),
    );

    const node = el("div");
    node.appendChild(el("h2", { text: "Versions (" + list.length + ")" }));
    if (!list.length) {
      node.appendChild(el("p", { text: "No versions available." }));
      return { single: null, node };
    }
    if (!single && available.length > 1) {
      node.appendChild(
        el("p", { text: "Multiple versions are available. Choose one to play." }),
      );
    }
    const rows = list.map((v, i) => {
      const detail = details[i];
      const bits = [v.quality.toUpperCase(), v.container];
      const vid = detail && detail.video && detail.video[0];
      if (vid) bits.push(vid.width + "x" + vid.height, vid.codec);
      const aud = detail && detail.audio && detail.audio[0];
      if (aud) bits.push(aud.codec + " " + aud.channels + "ch");
      if (v.edition) bits.push(v.edition);
      bits.push(Math.round(v.size_bytes / 1048576) + " MB");
      if (!v.available) bits.push("unavailable");
      const row = el("li", null, [el("span", { text: bits.join(" · ") + " " })]);
      if (v.available) {
        const isResume = Object.prototype.hasOwnProperty.call(resume, v.id);
        const play = el(
          "a",
          { href: href.watch(v.id) },
          isResume ? "Resume " + (resume[v.id] || 0) + "%" : "Play",
        );
        row.appendChild(play);
        if (isResume && uid) {
          const dismiss = el("button", { type: "button" }, "Dismiss");
          dismiss.addEventListener("click", async () => {
            dismiss.disabled = true;
            try {
              await json(
                "/api/v1/users/" +
                  encodeURIComponent(uid) +
                  "/progress/" +
                  encodeURIComponent(v.id),
                { method: "DELETE" },
              );
              play.textContent = "Play";
              dismiss.remove();
            } catch (e) {
              dismiss.disabled = false;
            }
          });
          row.appendChild(el("span", { text: " " }));
          row.appendChild(dismiss);
        }
      }
      return row;
    });
    node.appendChild(el("ul", null, rows));
    return { single, node };
  }

  function el(tag, props, children) {
    const node = document.createElement(tag);
    if (props) {
      for (const key of Object.keys(props)) {
        const value = props[key];
        if (value === null || value === undefined) continue;
        if (key === "text") node.textContent = value;
        else node.setAttribute(key, value);
      }
    }
    const kids = (Array.isArray(children) ? children : [children]).flat(Infinity);
    for (const child of kids) {
      if (child === null || child === undefined || child === false) continue;
      node.appendChild(
        typeof child === "object" ? child : document.createTextNode(String(child)),
      );
    }
    return node;
  }

  function img(artwork, kind, width) {
    const set = artwork && artwork[kind];
    if (!set || !Array.isArray(set.widths) || set.widths.length === 0) {
      return null;
    }
    const wider = set.widths.filter((w) => w >= width).sort((a, b) => a - b)[0];
    const chosen = wider !== undefined ? wider : set.widths[set.widths.length - 1];
    return el("img", {
      src: set.base + "/" + chosen,
      alt: "",
      width: String(width),
      loading: "lazy",
    });
  }

  function poster(artwork, width) {
    return (
      img(artwork, "poster", width) ||
      el("img", {
        src: url("placeholder.svg"),
        alt: "",
        width: String(width),
        loading: "lazy",
      })
    );
  }

  function authImg(path, attrs = {}) {
    const image = el("img", Object.assign({ alt: "", loading: "lazy" }, attrs));
    api(path)
      .then((res) => (res.ok ? res.blob() : Promise.reject(new Error("HTTP " + res.status))))
      .then((blob) => {
        image.src = URL.createObjectURL(blob);
        image.addEventListener("load", () => URL.revokeObjectURL(image.src), { once: true });
      })
      .catch(() => {
        image.alt = "unavailable";
      });
    return image;
  }

  function card(opts) {
    const image = poster(opts.artwork, 180);
    const caption = el("figcaption", null, [
      el("strong", { text: opts.title }),
      opts.subtitle ? el("div", { text: opts.subtitle }) : null,
    ]);
    return el("li", null, el("a", { href: opts.href }, el("figure", null, [image, caption])));
  }

  function grid(items) {
    const list = el("ul", { class: "sm-grid" }, items.map(card));
    const refs = items.map((i) => i.ref).filter(Boolean);
    if (refs.length) {
      markWatched(list, items, refs);
    }
    return list;
  }

  async function markWatched(list, items, refs) {
    try {
      const uid = (await self()).id;
      const states = await json("/api/v1/users/" + enc(uid) + "/state/batch", {
        method: "POST",
        body: JSON.stringify({ titles: refs }),
      });
      const watched = new Set();
      states.forEach((s) => {
        if (s && s.watched && s.title) watched.add(s.title.type + ":" + s.title.id);
      });
      const cards = list.children;
      items.forEach((item, index) => {
        if (!item.ref) return;
        if (watched.has(item.ref.type + ":" + item.ref.id) && cards[index]) {
          const caption = cards[index].querySelector("figcaption");
          if (caption) caption.appendChild(el("span", { class: "sm-badge" }, "Watched"));
        }
      });
    } catch (e) {}
  }

  function table(headers, rows) {
    return el("table", null, [
      el("thead", null, el("tr", null, headers.map((h) => el("th", { scope: "col" }, h)))),
      el(
        "tbody",
        null,
        rows.map((cells) => el("tr", null, cells.map((cell) => el("td", null, cell)))),
      ),
    ]);
  }

  function pager(total, offset, limit) {
    const params = new URLSearchParams(location.search);
    const link = (label, next) => {
      const p = new URLSearchParams(params);
      p.set("offset", String(next));
      return el("a", { href: location.pathname + "?" + p.toString() }, label);
    };
    const parts = [];
    if (offset > 0) {
      parts.push(link("Prev", Math.max(0, offset - limit)));
    }
    const from = total === 0 ? 0 : offset + 1;
    parts.push(
      el("span", { text: " " + from + " to " + Math.min(offset + limit, total) + " of " + total + " " }),
    );
    if (offset + limit < total) {
      parts.push(link("Next", offset + limit));
    }
    return el("nav", null, parts);
  }

  function breadcrumbs(items) {
    const nodes = [];
    items.forEach((item, index) => {
      if (index) nodes.push(document.createTextNode(" / "));
      if (item.href) nodes.push(el("a", { href: item.href }, item.label));
      else nodes.push(el("span", { text: item.label }));
    });
    return el("nav", { class: "sm-crumbs" }, nodes);
  }

  function dialog(title, body) {
    const box = el("dialog", { class: "sm-dialog" });
    const close = el("button", { type: "button" }, "Close");
    close.addEventListener("click", () => box.close());
    box.addEventListener("close", () => box.remove());
    box.appendChild(el("div", { class: "sm-dialog-head" }, [el("strong", { text: title }), close]));
    box.appendChild(body);
    document.body.appendChild(box);
    if (box.showModal) box.showModal();
    else box.setAttribute("open", "open");
    return box;
  }

  function tabs(items) {
    const bar = el("div", { class: "sm-tabs", role: "tablist" });
    const panels = [];
    const buttons = [];
    const select = (index) => {
      buttons.forEach((b, i) => b.setAttribute("aria-selected", i === index ? "true" : "false"));
      panels.forEach((p, i) => {
        p.hidden = i !== index;
      });
    };
    items.forEach((item, index) => {
      const button = el("button", { type: "button", class: "sm-tab", role: "tab" }, item.label);
      button.addEventListener("click", () => select(index));
      buttons.push(button);
      bar.appendChild(button);
      const panel = el("section", { class: "sm-tabpanel", role: "tabpanel" }, item.panel);
      panels.push(panel);
    });
    const wrap = el("div", null, [bar, ...panels]);
    select(0);
    return wrap;
  }

  const enc = encodeURIComponent;
  const href = {
    movie: (id) => url("catalog/title.html?type=movie&id=" + enc(id)),
    series: (id) => url("catalog/title.html?type=series&id=" + enc(id)),
    collection: (id) => url("catalog/collections.html?id=" + enc(id)),
    person: (id) => url("catalog/person.html?id=" + enc(id)),
    version: (id) => url("catalog/version.html?id=" + enc(id)),
    watch: (id) => url("player/watch.html?version=" + enc(id)),
    season: (id, series) =>
      url("catalog/season.html?id=" + enc(id) + (series ? "&series=" + enc(series) : "")),
    episode: (id, season, series) =>
      url(
        "catalog/episode.html?id=" +
          enc(id) +
          (season ? "&season=" + enc(season) : "") +
          (series ? "&series=" + enc(series) : ""),
      ),
    ref: (r) => (r.type === "movie" ? href.movie(r.id) : href.episode(r.id)),
  };

  function wireLogout() {
    const link = document.getElementById("logout");
    if (link) {
      link.addEventListener("click", (e) => {
        e.preventDefault();
        logout();
      });
    }
  }

  function navLinks() {
    const items = [
      ["home.html", "Home"],
      ["catalog/movies.html", "Movies"],
      ["catalog/series.html", "Series"],
      ["catalog/collections.html", "Collections"],
      ["catalog/genres.html", "Genres"],
      ["catalog/libraries.html", "Libraries"],
      ["catalog/search.html", "Search"],
      ["account/account.html", "Account"],
    ];
    const nodes = items.map((item) => el("a", { href: url(item[0]) }, item[1]));
    nodes.push(el("a", { href: "#", id: "logout" }, "Sign out"));
    return nodes;
  }

  function chrome() {
    const holder = document.getElementById("nav");
    if (holder) {
      holder.textContent = "";
      navLinks().forEach((node) => holder.appendChild(node));
      self()
        .then((me) => {
          if (!me) return;
          const logoutEl = holder.querySelector("#logout");
          if (!logoutEl) return;
          if (me.role === "admin") {
            holder.insertBefore(el("a", { href: url("admin/index.html") }, "Admin"), logoutEl);
          }
          holder.insertBefore(el("span", { class: "sm-who" }, me.username), logoutEl);
          holder.insertBefore(document.createTextNode(" / "), logoutEl);
        })
        .catch(() => {});
    }
    wireLogout();
  }

  async function libraryControls(leafType, leafId) {
    const box = el("div", null);
    const uid = (await self()).id;
    const path = (kind) => "/api/v1/users/" + enc(uid) + "/" + kind + "/" + enc(leafId);
    const toggle = (on, onLabel, offLabel, action) => {
      const button = el("button", { type: "button" }, on ? onLabel : offLabel);
      button.addEventListener("click", async () => {
        button.setAttribute("disabled", "disabled");
        try {
          await action(on);
          await refresh();
        } catch (e) {
          button.removeAttribute("disabled");
        }
      });
      return button;
    };
    const render = (state) => {
      box.textContent = "";
      box.appendChild(
        el("p", { class: "sm-status" }, [
          el("strong", { text: state.watched ? "Watched" : "Not watched" }),
        ]),
      );
      box.appendChild(
        toggle(state.watchlisted, "Remove from watchlist", "Add to watchlist", (on) =>
          on
            ? json(path("watchlist"), { method: "DELETE" })
            : json(path("watchlist"), { method: "PUT", body: JSON.stringify({ type: leafType }) }),
        ),
      );
      box.appendChild(document.createTextNode(" "));
      box.appendChild(
        toggle(state.favorite, "Remove favorite", "Add favorite", (on) =>
          on
            ? json(path("favorites"), { method: "DELETE" })
            : json(path("favorites"), { method: "PUT", body: JSON.stringify({ type: leafType }) }),
        ),
      );
      box.appendChild(document.createTextNode(" "));
      box.appendChild(
        toggle(state.watched, "Mark unwatched", "Mark watched", (on) =>
          json("/api/v1/users/" + enc(uid) + "/watched/" + enc(leafId), {
            method: "PUT",
            body: JSON.stringify({ type: leafType, watched: !on }),
          }),
        ),
      );
    };
    async function refresh() {
      const states = await json("/api/v1/users/" + enc(uid) + "/state/batch", {
        method: "POST",
        body: JSON.stringify({ titles: [{ type: leafType, id: leafId }] }),
      });
      render(states[0] || { favorite: false, watchlisted: false, watched: false });
    }
    await refresh();
    return box;
  }

  async function episodeIdsFor(targetType, targetId) {
    if (targetType === "season") {
      const eps = await json("/api/v1/series/-/seasons/" + enc(targetId) + "/episodes").catch(
        () => [],
      );
      return eps.map((e) => e.id);
    }
    if (targetType === "series") {
      const seasons = await json("/api/v1/series/" + enc(targetId) + "/seasons").catch(() => []);
      const ids = [];
      for (const season of seasons) {
        const eps = await json("/api/v1/series/-/seasons/" + enc(season.id) + "/episodes").catch(
          () => [],
        );
        eps.forEach((e) => ids.push(e.id));
      }
      return ids;
    }
    return [];
  }

  async function watchedControls(targetType, targetId) {
    const uid = (await self()).id;
    const box = el("div", null);
    const render = async () => {
      box.textContent = "Loading...";
      const ids = await episodeIdsFor(targetType, targetId);
      let watched = 0;
      if (ids.length) {
        const states = await json("/api/v1/users/" + enc(uid) + "/state/batch", {
          method: "POST",
          body: JSON.stringify({ titles: ids.map((id) => ({ type: "episode", id })) }),
        }).catch(() => []);
        watched = states.filter((s) => s && s.watched).length;
      }
      const all = ids.length > 0 && watched === ids.length;
      const statusText =
        ids.length === 0
          ? "No episodes"
          : all
            ? "Watched"
            : watched > 0
              ? "Partially watched (" + watched + " of " + ids.length + ")"
              : "Not watched";
      box.textContent = "";
      box.appendChild(el("p", { class: "sm-status" }, [el("strong", { text: statusText })]));
      if (!ids.length) return;
      const button = el("button", { type: "button" }, all ? "Mark unwatched" : "Mark all watched");
      button.addEventListener("click", async () => {
        button.setAttribute("disabled", "disabled");
        try {
          await json("/api/v1/users/" + enc(uid) + "/watched/" + enc(targetId), {
            method: "PUT",
            body: JSON.stringify({ type: targetType, watched: !all }),
          });
          await render();
        } catch (e) {
          button.removeAttribute("disabled");
        }
      });
      box.appendChild(button);
    };
    await render();
    return box;
  }

  function relinkPanel(versionId, label, onDone) {
    const panel = el("div", { class: "sm-relink" });
    const results = el("div", null);
    const move = async (target, button) => {
      button.setAttribute("disabled", "disabled");
      try {
        await json("/api/v1/versions/" + enc(versionId) + "/relink", {
          method: "POST",
          body: JSON.stringify({ target }),
        });
        panel.textContent = "";
        panel.appendChild(
          el("p", { class: "sm-status" }, [el("strong", { text: "Relink queued." })]),
        );
        if (onDone) onDone();
      } catch (err) {
        button.removeAttribute("disabled");
        button.textContent = "Failed";
      }
    };

    panel.appendChild(el("h4", { text: "Relink " + label }));
    panel.appendChild(el("p", { text: "Move this file to a different title." }));

    const form = el("form", null, [
      el("input", { name: "q", placeholder: "Search titles", autocomplete: "off" }),
      el("select", { name: "type" }, [
        el("option", { value: "movie" }, "Movie"),
        el("option", { value: "episode" }, "Episode"),
      ]),
      el("button", { type: "submit" }, "Search"),
    ]);
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const q = form.elements["q"].value;
      const type = form.elements["type"].value;
      if (!q) return;
      results.textContent = "Loading...";
      try {
        const page = await json("/api/v1/search?" + new URLSearchParams({ q, type }).toString());
        results.textContent = "";
        const items = (page.items || []).filter((r) => r.type === "movie" || r.type === "episode");
        if (!items.length) {
          results.appendChild(el("p", { text: "No matches." }));
          return;
        }
        const rows = items.map((r) => {
          const pick = el("button", { type: "button" }, "Move here");
          pick.addEventListener("click", () =>
            move({ kind: "existing", title: { type: r.type, id: r.id } }, pick),
          );
          return [r.title + (r.year ? " (" + r.year + ")" : ""), r.type, pick];
        });
        results.appendChild(table(["Match", "Type", ""], rows));
      } catch (err) {
        results.textContent = "Search failed.";
      }
    });
    panel.appendChild(form);
    panel.appendChild(results);

    const provValue = el("input", { name: "value", placeholder: "movie/603", autocomplete: "off" });
    const provSource = el("input", { name: "source", value: "tmdb", size: "6" });
    const provBtn = el("button", { type: "submit" }, "Move to provider id");
    const prov = el("form", { class: "sm-relink-provider" }, [
      el("label", { text: "Or a provider id: " }),
      provSource,
      provValue,
      provBtn,
    ]);
    prov.addEventListener("submit", (e) => {
      e.preventDefault();
      if (!provValue.value) return;
      move({ kind: "provider", source: provSource.value || "tmdb", value: provValue.value }, provBtn);
    });
    panel.appendChild(prov);

    const cancel = el("button", { type: "button" }, "Cancel");
    cancel.addEventListener("click", () => panel.remove());
    panel.appendChild(cancel);

    return panel;
  }

  async function accessAwareEmpty(noun) {
    const params = new URLSearchParams(location.search);
    if (!params.get("genre") && !params.get("library")) {
      try {
        const me = await self();
        if (me.role !== "admin") {
          const libs = await json("/api/v1/libraries").catch(() => []);
          if (!libs.length) {
            return el("p", {
              text: "No libraries have been shared with your account yet. Ask an administrator to grant you access.",
            });
          }
        }
      } catch (e) {}
    }
    return el("p", { text: "No " + noun + " found." });
  }

  if (typeof document !== "undefined" && document.head) {
    document.head.appendChild(el("link", { rel: "stylesheet", href: url("app.css") }));
  }

  return {
    login,
    logout,
    api,
    json,
    self,
    refresh,
    requireAuth,
    tokens: load,
    qs,
    lang,
    time,
    versionPicker,
    onDeckEpisode,
    el,
    img,
    poster,
    authImg,
    card,
    grid,
    table,
    pager,
    breadcrumbs,
    dialog,
    tabs,
    href,
    url,
    wireLogout,
    chrome,
    libraryControls,
    watchedControls,
    relinkPanel,
    accessAwareEmpty,
  };
})();
