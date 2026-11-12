// Thin glue between Yew and Quill 2 (loaded globally from the CDN in index.html).

export function createEditor(container, onChange) {
  const quill = new Quill(container, {
    theme: "snow",
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
}
