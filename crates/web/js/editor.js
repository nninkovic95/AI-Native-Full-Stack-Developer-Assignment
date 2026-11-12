// Thin glue between Yew and Quill 2 (loaded globally from the CDN in index.html).

export function createEditor(container, onChange) {
  const quill = new Quill(container, {
    theme: "snow",
    // Without an allowlist Quill embeds pasted or dropped images as base64, which
    // outgrows the server's request size limit and makes every later save fail.
    formats: ["header", "bold", "italic", "underline", "list", "indent"],
    modules: {
      toolbar: [
        [{ header: [1, 2, false] }],
        ["bold", "italic", "underline"],
        [{ list: "ordered" }, { list: "bullet" }],
      ],
    },
  });
  // Only user edits count; programmatic setContents uses the 'silent' source.
  quill.on("text-change", (_delta, _old, source) => {
    if (source === "user") onChange();
  });
  return quill;
}

export function getContents(quill) {
  return JSON.stringify(quill.getContents());
}

export function setContents(quill, json) {
  quill.setContents(JSON.parse(json), "silent");
  // Quill's history records silent changes too; without this, undo removes the
  // loaded document and the empty result is autosaved.
  quill.history.clear();
}
