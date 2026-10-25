export function logLevel(entry) {
  const words = String(entry).split("\n")[0].trim().split(/\s+/);
  const level = (words[1] || "").toLowerCase();
  return ["debug", "warn", "error"].includes(level) ? level : "info";
}

export function logLine(entry) {
  const line = document.createElement("div");
  line.className = "sm-log-line sm-log-" + logLevel(entry);
  line.textContent = entry;
  return line;
}

export function logBox(entries) {
  const box = document.createElement("div");
  box.className = "sm-log";
  box.append(...entries.map(logLine));
  return box;
}
