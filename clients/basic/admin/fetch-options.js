export function downloadOptions(values) {
  const options = {};
  const format = values.format.trim();
  const skipNote = values.skipNote.trim();
  if (format) options.format = format;
  if (skipNote) options.skip_note = skipNote;
  if (values.preferH264) options.prefer_h264 = true;
  if (values.siteSubtitles) options.site_subtitles = true;
  if (values.chapters) options.chapters = true;
  return options;
}

export function optionsProblem(options) {
  return options.format && options.skip_note
    ? "A format replaces the default choice, so it cannot also skip noted streams."
    : null;
}
