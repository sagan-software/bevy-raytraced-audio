/** Adds a copy button to the generated source listing when the Clipboard API is available. */
export function installSourceCopy(document, clipboard) {
  const button = document.querySelector(".copy-source");
  const code = document.querySelector(".source-code code");
  if (!button || !code || typeof clipboard?.writeText !== "function") {
    return false;
  }

  button.hidden = false;
  button.addEventListener("click", async () => {
    try {
      await clipboard.writeText(code.textContent);
      button.textContent = "Copied";
    } catch {
      button.textContent = "Copy failed";
    }
  });
  return true;
}

if (typeof document !== "undefined" && typeof navigator !== "undefined") {
  installSourceCopy(document, navigator.clipboard);
}
