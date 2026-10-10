/**
 * A small Rust syntax highlighter for the generated example pages.
 *
 * The tokenizer works on the raw source and escapes every token's text when it
 * emits HTML, so no source character reaches the page unescaped.
 */

const KEYWORDS = new Set([
  "as",
  "async",
  "await",
  "break",
  "const",
  "continue",
  "crate",
  "dyn",
  "else",
  "enum",
  "extern",
  "fn",
  "for",
  "if",
  "impl",
  "in",
  "let",
  "loop",
  "match",
  "mod",
  "move",
  "mut",
  "pub",
  "ref",
  "return",
  "self",
  "static",
  "struct",
  "super",
  "trait",
  "type",
  "unsafe",
  "use",
  "where",
  "while",
  "yield",
]);

const LITERALS = new Set(["true", "false"]);

const PRIMITIVE_TYPES = new Set([
  "bool",
  "char",
  "str",
  "u8",
  "u16",
  "u32",
  "u64",
  "u128",
  "usize",
  "i8",
  "i16",
  "i32",
  "i64",
  "i128",
  "isize",
  "f32",
  "f64",
  "Self",
]);

const NUMBER_SUFFIX = "(?:_?(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64))?";
const NUMBER = new RegExp(
  `^(?:0x[0-9a-fA-F_]+|0o[0-7_]+|0b[01_]+|[0-9][0-9_]*(?:\\.(?![.\\p{L}_])(?:[0-9][0-9_]*)?)?(?:[eE][+-]?[0-9_]+)?)${NUMBER_SUFFIX}`,
  "u",
);
const IDENTIFIER = /^(?:r#)?[\p{L}_][\p{L}\p{N}_]*/u;
const CHAR_LITERAL = /^b?'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]{1,8}\}|.)|[^\\'\n\r])'/u;
const LIFETIME = /^'[\p{L}_][\p{L}\p{N}_]*/u;
const RAW_STRING_START = /^(?:br|cr|r)(#*)"/u;
const WHITESPACE = /^\s+/u;

/** Escapes text for use in HTML element content and double-quoted attribute values. */
export function escapeHtml(text) {
  return String(text)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll("\"", "&quot;")
    .replaceAll("'", "&#39;");
}

/** Returns the end index of a quoted string that starts at `start` (pointing at `"`). */
function quotedStringEnd(source, start) {
  let index = start + 1;
  while (index < source.length) {
    const character = source[index];
    if (character === "\\") {
      index += 2;
    } else if (character === "\"") {
      return index + 1;
    } else {
      index += 1;
    }
  }
  return source.length;
}

/** Returns the end index of a (possibly nested) block comment starting at `start`. */
function blockCommentEnd(source, start) {
  let depth = 0;
  let index = start;
  while (index < source.length) {
    if (source.startsWith("/*", index)) {
      depth += 1;
      index += 2;
    } else if (source.startsWith("*/", index)) {
      depth -= 1;
      index += 2;
      if (depth === 0) {
        return index;
      }
    } else {
      index += 1;
    }
  }
  return source.length;
}

/** Returns the end index of a raw string such as `r#"…"#` whose prefix match is given. */
function rawStringEnd(source, start, prefixMatch) {
  const terminator = `"${prefixMatch[1]}`;
  const close = source.indexOf(terminator, start + prefixMatch[0].length);
  return close === -1 ? source.length : close + terminator.length;
}

/** Returns the end index of an attribute such as `#[derive(Component)]`. */
function attributeEnd(source, start) {
  let index = source.indexOf("[", start) + 1;
  let depth = 1;
  while (index < source.length && depth > 0) {
    const character = source[index];
    if (character === "\"") {
      index = quotedStringEnd(source, index);
      continue;
    }
    const rawMatch = RAW_STRING_START.exec(source.slice(index, index + 260));
    if (rawMatch && !/[\p{L}\p{N}_]/u.test(source[index - 1] ?? "")) {
      index = rawStringEnd(source, index, rawMatch);
      continue;
    }
    if (character === "[") {
      depth += 1;
    } else if (character === "]") {
      depth -= 1;
    }
    index += 1;
  }
  return index;
}

function classifyIdentifier(word, source, end) {
  const bare = word.startsWith("r#") ? word.slice(2) : word;
  if (!word.startsWith("r#")) {
    if (KEYWORDS.has(bare)) {
      return "keyword";
    }
    if (LITERALS.has(bare)) {
      return "literal";
    }
  }
  if (PRIMITIVE_TYPES.has(bare)) {
    return "type";
  }
  if (/^[\p{Lu}][\p{Lu}\p{N}_]+$/u.test(bare)) {
    return "constant";
  }
  if (/^[\p{Lu}]/u.test(bare)) {
    return "type";
  }
  const rest = source.slice(end);
  const before = source.slice(Math.max(0, end - word.length - 24), end - word.length);
  if (/\bfn\s+$/u.test(before) || /^\s*\(/u.test(rest) || /^::</u.test(rest)) {
    return "function";
  }
  return null;
}

/**
 * Splits Rust source into tokens.
 * @param {string} source Rust source text.
 * @returns {{ type: string | null, text: string }[]} Tokens whose texts concatenate to `source`.
 */
export function tokenizeRust(source) {
  const tokens = [];
  let plain = "";
  let index = 0;

  const push = (type, end) => {
    if (plain) {
      tokens.push({ type: null, text: plain });
      plain = "";
    }
    tokens.push({ type, text: source.slice(index, end) });
    index = end;
  };

  while (index < source.length) {
    const character = source[index];
    const previous = source[index - 1] ?? "";
    const atWordBoundary = !/[\p{L}\p{N}_]/u.test(previous);
    const rest = source.slice(index, index + 260);

    if (WHITESPACE.test(character)) {
      plain += WHITESPACE.exec(rest)[0];
      index += WHITESPACE.exec(rest)[0].length;
      continue;
    }

    if (source.startsWith("//", index)) {
      const lineEnd = source.indexOf("\n", index);
      const end = lineEnd === -1 ? source.length : lineEnd;
      const isDoc = /^\/\/(?:!|\/(?!\/))/u.test(rest);
      push(isDoc ? "doc-comment" : "comment", end);
      continue;
    }

    if (source.startsWith("/*", index)) {
      const isDoc = /^\/\*(?:!|\*(?![*/]))/u.test(rest);
      push(isDoc ? "doc-comment" : "comment", blockCommentEnd(source, index));
      continue;
    }

    if (atWordBoundary) {
      const rawMatch = RAW_STRING_START.exec(rest);
      if (rawMatch) {
        push("string", rawStringEnd(source, index, rawMatch));
        continue;
      }
      if ((character === "b" || character === "c") && source[index + 1] === "\"") {
        push("string", quotedStringEnd(source, index + 1));
        continue;
      }
    }

    if (character === "\"") {
      push("string", quotedStringEnd(source, index));
      continue;
    }

    if (character === "'" || (character === "b" && source[index + 1] === "'" && atWordBoundary)) {
      const charMatch = CHAR_LITERAL.exec(rest);
      if (charMatch) {
        push("char", index + charMatch[0].length);
        continue;
      }
      const lifetimeMatch = character === "'" ? LIFETIME.exec(rest) : null;
      if (lifetimeMatch) {
        push("lifetime", index + lifetimeMatch[0].length);
        continue;
      }
    }

    if (character === "#" && /^#!?\[/u.test(rest)) {
      push("attribute", attributeEnd(source, index));
      continue;
    }

    if (/[0-9]/u.test(character) && atWordBoundary) {
      push("number", index + NUMBER.exec(rest)[0].length);
      continue;
    }

    const identifierMatch = IDENTIFIER.exec(rest);
    if (identifierMatch && atWordBoundary) {
      const word = identifierMatch[0];
      const end = index + word.length;
      if (source[end] === "!" && source[end + 1] !== "=" && !KEYWORDS.has(word)) {
        push("macro", end + 1);
        continue;
      }
      const type = classifyIdentifier(word, source, end);
      if (type) {
        push(type, end);
      } else {
        plain += word;
        index = end;
      }
      continue;
    }

    plain += character;
    index += 1;
  }

  if (plain) {
    tokens.push({ type: null, text: plain });
  }
  return tokens;
}

/**
 * Highlights Rust source as escaped HTML with `<span class="tok-…">` wrappers.
 * @param {string} source Rust source text.
 * @returns {string} HTML suitable for the content of `<code>`.
 */
export function highlightRust(source) {
  return tokenizeRust(source)
    .map(({ type, text }) => (type ? `<span class="tok-${type}">${escapeHtml(text)}</span>` : escapeHtml(text)))
    .join("");
}
