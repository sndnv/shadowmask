const sm = (() => {
  const API = "";
  const KEY = "shadowmask.tokens";
  let dialogSeq = 0;
  let tabsSeq = 0;

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
      let message = "HTTP " + res.status;
      let code = null;
      try {
        const body = await res.json();
        if (body && body.error) {
          if (body.error.message) message = body.error.message;
          code = body.error.code || null;
        }
      } catch (e) {
        void e;
      }
      const err = new Error(message);
      err.status = res.status;
      err.code = code;
      throw err;
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

  function date(iso) {
    if (!iso) return "";
    const parsed = new Date(iso);
    return isNaN(parsed.getTime())
      ? String(iso)
      : parsed.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
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
      bits.push(Math.round(v.size_bytes / 1048576) + " MB");
      if (!v.available) bits.push("unavailable");
      const row = el("li", null, [el("span", { text: bits.join(" · ") })]);
      if (v.available) {
        const isResume = Object.prototype.hasOwnProperty.call(resume, v.id);
        const play = el(
          "a",
          { href: href.watch(v.id), class: "sm-play" },
          isResume ? "Resume " + (resume[v.id] || 0) + "%" : "Play",
        );
        row.appendChild(play);
        if (isResume && uid) {
          const dismiss = el("button", { type: "button", class: "sm-secondary" }, "Dismiss");
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
          row.appendChild(dismiss);
        }
      }
      return row;
    });
    node.appendChild(el("ul", { class: "sm-versions" }, rows));
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

  function imgFromSet(set, width, alt) {
    if (!set || !Array.isArray(set.widths) || set.widths.length === 0) {
      return null;
    }
    const wider = set.widths.filter((w) => w >= width).sort((a, b) => a - b)[0];
    const chosen = wider !== undefined ? wider : set.widths[set.widths.length - 1];
    return el("img", {
      src: set.base + "/" + chosen,
      alt: alt || "",
      width: String(width),
      loading: "lazy",
    });
  }

  function img(artwork, kind, width, alt) {
    const list = artwork && artwork[kind + "s"];
    return imgFromSet(Array.isArray(list) ? list[0] : null, width, alt);
  }

  function poster(artwork, width, alt, placeholder) {
    return (
      img(artwork, "poster", width, alt) ||
      img(artwork, "backdrop", width, alt) ||
      el("img", {
        src: url(placeholder || "placeholder.svg"),
        alt: alt || "",
        width: String(width),
        loading: "lazy",
      })
    );
  }

  function mosaicPoster(artwork, width, alt, placeholder) {
    const posters = (artwork && artwork.posters) || [];
    if (posters.length <= 1) {
      return poster(artwork, width, alt, placeholder);
    }
    const tiles = posters
      .slice(0, 4)
      .map((set) => imgFromSet(set, Math.round(width / 2), ""))
      .filter(Boolean);
    return el("div", { class: "sm-mosaic" }, tiles);
  }

  function landscapeImage(artwork, width, alt, placeholder) {
    return (
      img(artwork, "backdrop", width, alt) ||
      img(artwork, "poster", width, alt) ||
      el("img", {
        src: url(placeholder || "placeholder-landscape.svg"),
        alt: alt || "",
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
        image.classList.add("sm-img-broken");
      });
    return image;
  }

  function card(opts, landscape) {
    let image;
    if (opts.mosaic) {
      image = mosaicPoster(opts.artwork, 180, null, opts.placeholder);
    } else if (landscape) {
      image = landscapeImage(opts.artwork, 320, null, opts.placeholder);
    } else {
      image = poster(opts.artwork, 180, null, opts.placeholder);
    }
    const caption = el("figcaption", null, [
      el("strong", { text: opts.title }),
      opts.subtitle ? el("div", { text: opts.subtitle }) : null,
      opts.caption ? el("div", { text: opts.caption }) : null,
    ]);
    return el("li", null, el("a", { href: opts.href }, el("figure", null, [image, caption])));
  }

  function grid(items, opts) {
    const augment = opts && opts.augment;
    const landscape = !!(opts && opts.landscape);
    const nodes = items.map((item) => {
      const li = card(item, landscape);
      if (augment) augment(li, item);
      return li;
    });
    const list = el("ul", { class: landscape ? "sm-grid sm-grid-landscape" : "sm-grid" }, nodes);
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

  function table(headers, rows, opts) {
    const options = opts || {};
    const body = [];
    rows.forEach((cells, index) => {
      const row = el("tr", null, cells.map((cell) => el("td", null, cell)));
      const rowClass = options.rowClass && options.rowClass(index);
      if (rowClass) row.setAttribute("class", rowClass);
      body.push(row);
      const detail = options.detailRow && options.detailRow(index);
      if (detail) {
        body.push(
          el(
            "tr",
            { class: "sm-detail-row" },
            el("td", { colspan: String(headers.length) }, detail),
          ),
        );
      }
    });
    const node = el("table", null, [
      el("thead", null, el("tr", null, headers.map((h) => el("th", { scope: "col" }, h)))),
      el("tbody", null, body),
    ]);
    return el("div", { class: "sm-table-wrap" }, node);
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
    return el("nav", { "aria-label": "Pagination" }, parts);
  }

  function breadcrumbs(items) {
    const nodes = [];
    items.forEach((item, index) => {
      if (index) nodes.push(document.createTextNode(" / "));
      if (item.href) nodes.push(el("a", { href: item.href }, item.label));
      else nodes.push(el("span", { text: item.label }));
    });
    return el("nav", { class: "sm-crumbs", "aria-label": "Breadcrumb" }, nodes);
  }

  function dialog(title, body) {
    const box = el("dialog", { class: "sm-dialog" });
    const titleId = "sm-dialog-title-" + dialogSeq++;
    box.setAttribute("aria-labelledby", titleId);
    const close = el("button", { type: "button", class: "sm-secondary" }, "Close");
    close.addEventListener("click", () => box.close());
    box.addEventListener("close", () => box.remove());
    box.appendChild(
      el("div", { class: "sm-dialog-head" }, [el("strong", { id: titleId, text: title }), close]),
    );
    box.appendChild(body);
    document.body.appendChild(box);
    if (box.showModal) box.showModal();
    else box.setAttribute("open", "open");
    return box;
  }

  function openDialog(title, buildBody) {
    const host = el("div", { class: "sm-dialog-body" });
    const box = dialog(title, host);
    host.appendChild(buildBody(box));
    return box;
  }

  function tabs(items, initial) {
    const group = tabsSeq++;
    const bar = el("div", { class: "sm-tabs", role: "tablist" });
    const panels = [];
    const buttons = [];
    const select = (index, focus) => {
      buttons.forEach((b, i) => {
        const on = i === index;
        b.setAttribute("aria-selected", on ? "true" : "false");
        b.setAttribute("tabindex", on ? "0" : "-1");
        if (on && focus) b.focus();
      });
      panels.forEach((p, i) => {
        p.hidden = i !== index;
      });
    };
    items.forEach((item, index) => {
      const tabId = "sm-tab-" + group + "-" + index;
      const panelId = "sm-tabpanel-" + group + "-" + index;
      const button = el(
        "button",
        { type: "button", class: "sm-tab", role: "tab", id: tabId, "aria-controls": panelId },
        item.label,
      );
      button.addEventListener("click", () => select(index, false));
      button.addEventListener("keydown", (event) => {
        const last = items.length - 1;
        let next = null;
        if (event.key === "ArrowRight" || event.key === "ArrowDown") {
          next = index === last ? 0 : index + 1;
        } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
          next = index === 0 ? last : index - 1;
        } else if (event.key === "Home") {
          next = 0;
        } else if (event.key === "End") {
          next = last;
        }
        if (next !== null) {
          event.preventDefault();
          select(next, true);
        }
      });
      buttons.push(button);
      bar.appendChild(button);
      const panel = el(
        "section",
        { class: "sm-tabpanel", role: "tabpanel", id: panelId, "aria-labelledby": tabId, tabindex: "0" },
        item.panel,
      );
      panels.push(panel);
    });
    const wrap = el("div", null, [bar, ...panels]);
    const start = Number.isInteger(initial) && initial >= 0 && initial < items.length ? initial : 0;
    select(start, false);
    return wrap;
  }

  function toast(message, kind) {
    let host = document.querySelector(".sm-toast-host");
    if (!host) {
      host = el("div", { class: "sm-toast-host", role: "status", "aria-live": "polite" });
      document.body.appendChild(host);
    }
    const node = el("div", { class: kind ? "sm-toast sm-toast-" + kind : "sm-toast" }, message);
    host.appendChild(node);
    setTimeout(() => node.remove(), 3200);
    return node;
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

  function activeSection() {
    const p = location.pathname;
    if (p.includes("/admin/")) return "admin";
    if (p.endsWith("/catalog/title.html")) {
      return qs("type") === "series" ? "catalog/series.html" : "catalog/movies.html";
    }
    if (p.endsWith("/catalog/season.html") || p.endsWith("/catalog/episode.html")) {
      return "catalog/series.html";
    }
    const sections = [
      "home.html",
      "catalog/movies.html",
      "catalog/series.html",
      "catalog/collections.html",
      "catalog/search.html",
      "account/account.html",
    ];
    return sections.find((key) => p.endsWith("/" + key)) || null;
  }

  function navLinks() {
    const items = [
      ["home.html", "Home"],
      ["catalog/movies.html", "Movies"],
      ["catalog/series.html", "Series"],
      ["catalog/collections.html", "Collections"],
      ["catalog/search.html", "Search"],
      ["account/account.html", "Account"],
    ];
    const active = activeSection();
    const logo = el(
      "a",
      { href: url("home.html"), class: "sm-logo", "aria-label": "Shadowmask home" },
      el("img", { src: url("logo.svg"), alt: "", width: "22", height: "22" }),
    );
    const nodes = [logo].concat(
      items.map((item) => {
        const attrs = { href: url(item[0]) };
        if (item[0] === active) attrs["aria-current"] = "page";
        return el("a", attrs, item[1]);
      }),
    );
    nodes.push(el("button", { type: "button", id: "logout", class: "sm-secondary" }, "Sign out"));
    return nodes;
  }

  function footer() {
    if (document.querySelector(".sm-footer")) return;
    if (!load()) return;
    const node = el("footer", { class: "sm-footer" }, [
      el("a", { href: url("about.html") }, "About"),
    ]);
    document.body.appendChild(node);
  }

  function chrome() {
    if (location.pathname.includes("/admin/")) {
      document.body.classList.add("sm-wide");
    }
    const holder = document.getElementById("nav");
    if (holder) {
      holder.setAttribute("aria-label", "Primary");
      if (!document.querySelector(".sm-skip")) {
        const skip = el("a", { href: "#main", class: "sm-skip" }, "Skip to content");
        document.body.insertBefore(skip, document.body.firstChild);
      }
      holder.textContent = "";
      navLinks().forEach((node) => holder.appendChild(node));
      self()
        .then((me) => {
          if (!me) return;
          const logoutEl = holder.querySelector("#logout");
          if (!logoutEl) return;
          if (me.role === "admin") {
            const adminAttrs = { href: url("admin/index.html") };
            if (activeSection() === "admin") adminAttrs["aria-current"] = "page";
            holder.insertBefore(el("a", adminAttrs, "Admin"), logoutEl);
          }
          holder.insertBefore(el("span", { class: "sm-who" }, me.username), logoutEl);
          holder.insertBefore(document.createTextNode(" / "), logoutEl);
        })
        .catch(() => {});
    }
    wireLogout();
    footer();
  }

  async function libraryControls(leafType, leafId) {
    const box = el("div", null);
    let me;
    try {
      me = await self();
    } catch (e) {
      box.textContent = "Sign-in required.";
      return box;
    }
    const uid = me.id;
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
        el("p", { class: "sm-status", "aria-live": "polite" }, [
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

  async function watchedControls(targetType, targetId) {
    const box = el("div", null);
    let me;
    try {
      me = await self();
    } catch (e) {
      box.textContent = "Sign-in required.";
      return box;
    }
    const uid = me.id;
    const render = async () => {
      box.textContent = "Loading…";
      const rollups = await json("/api/v1/users/" + enc(uid) + "/state/rollup", {
        method: "POST",
        body: JSON.stringify({ targets: [{ type: targetType, id: targetId }] }),
      }).catch(() => []);
      const roll = rollups[0] || { watched: false, watched_episodes: 0, total_episodes: 0 };
      const total = roll.total_episodes;
      const watched = roll.watched_episodes;
      const all = roll.watched;
      const statusText =
        total === 0
          ? "No episodes"
          : all
            ? "Watched"
            : watched > 0
              ? "Partially watched (" + watched + " of " + total + ")"
              : "Not watched";
      box.textContent = "";
      box.appendChild(
        el("p", { class: "sm-status", "aria-live": "polite" }, [el("strong", { text: statusText })]),
      );
      if (total === 0) return;
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
    const results = el("div", { "aria-live": "polite" });
    const move = async (target, button) => {
      button.setAttribute("disabled", "disabled");
      try {
        await json("/api/v1/versions/" + enc(versionId) + "/relink", {
          method: "POST",
          body: JSON.stringify({ target }),
        });
        panel.textContent = "";
        panel.appendChild(
          el("p", { class: "sm-status", "aria-live": "polite" }, [
            el("strong", { text: "Relink queued." }),
          ]),
        );
        toast("Relink queued.", "ok");
        if (onDone) onDone();
      } catch (err) {
        button.removeAttribute("disabled");
        button.textContent = "Failed";
        toast("Relink failed.", "danger");
      }
    };

    panel.appendChild(el("p", { text: "Move this file to a different title." }));

    const form = el("form", null, [
      el("label", null, ["Search titles ", el("input", { name: "q", autocomplete: "off" })]),
      " ",
      el("label", null, [
        "Type ",
        el("select", { name: "type" }, [
          el("option", { value: "movie" }, "Movie"),
          el("option", { value: "episode" }, "Episode"),
        ]),
      ]),
      " ",
      el("button", { type: "submit" }, "Search"),
    ]);
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const q = form.elements["q"].value;
      const type = form.elements["type"].value;
      if (!q) return;
      results.textContent = "Loading…";
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
      "Or a provider id: ",
      el("label", null, ["Source ", provSource]),
      " ",
      el("label", null, ["Value ", provValue]),
      " ",
      provBtn,
    ]);
    prov.addEventListener("submit", (e) => {
      e.preventDefault();
      if (!provValue.value) return;
      move({ kind: "provider", source: provSource.value || "tmdb", value: provValue.value }, provBtn);
    });
    panel.appendChild(prov);

    return panel;
  }

  function relinkButton(versionId, label, onDone, available) {
    const btn = el("button", { type: "button" }, "Relink");
    if (available === false) {
      btn.disabled = true;
      btn.title =
        "The file for this version is missing, so there is nothing to read. " +
        "Remove the version instead.";
      return btn;
    }
    btn.addEventListener("click", () => {
      openDialog("Relink " + label, (box) =>
        relinkPanel(versionId, label, () => {
          if (onDone) onDone();
          box.close();
        }),
      );
    });
    return btn;
  }

  function deleteTitleButton(kind, id, label, blockedReason, onDone) {
    const btn = el(
      "button",
      { type: "button", class: blockedReason ? "sm-secondary" : "sm-danger" },
      "Delete",
    );
    if (blockedReason) {
      btn.disabled = true;
      btn.title = blockedReason;
      return btn;
    }
    btn.addEventListener("click", async () => {
      if (
        !window.confirm(
          "Delete " +
            label +
            " from the catalog, along with its artwork, cast and ratings. " +
            "Nothing is removed from disk.",
        )
      ) {
        return;
      }
      btn.disabled = true;
      try {
        await json("/api/v1/admin/" + kind + "/" + enc(id), { method: "DELETE" });
        if (onDone) onDone();
      } catch (e) {
        btn.disabled = false;
        window.alert("Delete failed.");
      }
    });
    return btn;
  }

  function refreshMetadataButton(titleType, id) {
    const toggle = el("button", { type: "button", class: "sm-secondary" }, "Metadata");
    toggle.addEventListener("click", () => {
      openDialog("Refresh metadata", (box) => refreshMetadataBody(titleType, id, box));
    });
    return toggle;
  }

  function refreshMetadataBody(titleType, id, box) {
    const panel = el("div", { class: "sm-refresh" });
    const endpoint = titleType === "series" ? "tv" : "movie";
    const idInput = el("input", {
      name: "id",
      placeholder: "603 or tt0133093",
      autocomplete: "off",
    });
    const button = el("button", { type: "submit" }, "Refresh metadata");
    const form = el("form", { class: "sm-refresh-form" }, [
      el("label", null, ["TMDB or IMDb id (optional) ", idInput]),
      " ",
      button,
    ]);
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      button.setAttribute("disabled", "disabled");
      const raw = idInput.value.trim();
      let body;
      if (raw) {
        const external = /^tt\d+$/i.test(raw)
          ? { source: "imdb", value: raw }
          : { source: "tmdb", value: raw.includes("/") ? raw : endpoint + "/" + raw };
        body = JSON.stringify({ external_id: external });
      }
      try {
        await json("/api/v1/" + titleType + "/" + enc(id) + "/refresh", { method: "POST", body });
        toast("Metadata refresh queued.", "ok");
        box.close();
      } catch (err) {
        toast("Could not queue refresh.", "danger");
        button.removeAttribute("disabled");
      }
    });
    panel.appendChild(
      el("p", {
        text: "Leave blank to re-match by title, or pin a TMDB id (e.g. 603) or IMDb id (e.g. tt0133093).",
      }),
    );
    panel.appendChild(form);
    return panel;
  }

  async function accessAwareEmpty(noun) {
    const params = new URLSearchParams(location.search);
    if (!params.get("genre") && !params.get("library")) {
      try {
        const me = await self();
        const granted = await json(
          "/api/v1/users/" + encodeURIComponent(me.id) + "/libraries",
        ).catch(() => []);
        if (!granted.length) {
          return el("p", {
            text: "No libraries have been shared with your account yet. Ask an administrator to grant you access.",
          });
        }
      } catch (e) {}
    }
    return el("p", { text: "No " + noun + " found." });
  }

  function pad2(n) {
    return String(n).padStart(2, "0");
  }

  function episodeCode(season, number) {
    return "S" + pad2(season) + "E" + pad2(number);
  }

  function episodeCount(n) {
    return n === 1 ? "1 episode" : n + " episodes";
  }

  function groupCode(code) {
    return code.length === 8 ? code.slice(0, 4) + "-" + code.slice(4) : code;
  }

  function sortControls(params, genres) {
    const option = (value, current) =>
      el(
        "option",
        Object.assign({ value }, value === current ? { selected: "selected" } : {}),
        value,
      );
    const currentSort = params.get("sort") || "added_at";
    const currentOrder = params.get("order") || "asc";
    const sort = el("select", { name: "sort" }, [
      option("added_at", currentSort),
      option("title", currentSort),
      option("year", currentSort),
    ]);
    const order = el("select", { name: "order" }, [
      option("asc", currentOrder),
      option("desc", currentOrder),
    ]);
    const selectedGenres = new Set(
      (params.get("genres") || "")
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
    );
    const genreBoxes = (genres || []).map((g) => {
      const box = el(
        "input",
        Object.assign(
          { type: "checkbox", value: g.name },
          selectedGenres.has(g.name) ? { checked: "checked" } : {},
        ),
      );
      return { box, node: el("label", { class: "sm-genre-chip" }, [box, " " + g.name]) };
    });
    const apply = () => {
      const next = new URLSearchParams(params);
      next.set("sort", sort.value);
      next.set("order", order.value);
      const chosen = genreBoxes.filter((entry) => entry.box.checked).map((entry) => entry.box.value);
      if (chosen.length) next.set("genres", chosen.join(","));
      else next.delete("genres");
      next.delete("offset");
      location.search = next.toString();
    };
    sort.addEventListener("change", apply);
    order.addEventListener("change", apply);
    genreBoxes.forEach((entry) => entry.box.addEventListener("change", apply));
    const children = [
      el("label", null, ["Sort ", sort]),
      el("label", null, ["Order ", order]),
    ];
    if (genreBoxes.length) {
      children.push(
        el("span", { class: "sm-genre-chips" }, [
          el("span", { class: "sm-controls-label" }, "Genres"),
          ...genreBoxes.map((entry) => entry.node),
        ]),
      );
    }
    return el("div", { class: "sm-controls" }, children);
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
    date,
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
    openDialog,
    tabs,
    toast,
    href,
    url,
    wireLogout,
    chrome,
    libraryControls,
    watchedControls,
    relinkPanel,
    relinkButton,
    deleteTitleButton,
    refreshMetadataButton,
    accessAwareEmpty,
    pad2,
    episodeCode,
    episodeCount,
    groupCode,
    sortControls,
    mosaicPoster,
  };
})();
