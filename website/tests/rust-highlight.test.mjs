import assert from "node:assert/strict";
import test from "node:test";

import { escapeHtml, highlightRust, tokenizeRust } from "../rust-highlight.mjs";

function typed(source) {
  return tokenizeRust(source)
    .filter((token) => token.type !== null)
    .map((token) => [token.type, token.text]);
}

function unescapeHtml(text) {
  return text
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", "\"")
    .replaceAll("&#39;", "'")
    .replaceAll("&amp;", "&");
}

test("tokens always concatenate back to the source", () => {
  const source = "fn main() { let x: Vec<&'static str> = vec![\"a//b\", r#\"q\"uote\"#]; /* c */ }\n";
  assert.equal(tokenizeRust(source).map((token) => token.text).join(""), source);
});

test("escapes HTML in every token and leaves no raw markup", () => {
  const source = "let s = \"<script>alert(1)</script>\"; // <b>&</b>\nlet t = a < b && c > d;";
  const output = highlightRust(source);
  assert.doesNotMatch(output, /<script|<b>/u);
  assert.equal(unescapeHtml(output.replace(/<\/?span[^>]*>/gu, "")), source);
  assert.equal(escapeHtml(`<a href="x">'&'</a>`), "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
});

test("keeps // inside strings and handles escaped quotes", () => {
  assert.deepEqual(typed("let url = \"https://example.com/\\\"q\\\"\"; // done"), [
    ["keyword", "let"],
    ["string", "\"https://example.com/\\\"q\\\"\""],
    ["comment", "// done"],
  ]);
});

test("recognizes raw strings with hashes and embedded quotes", () => {
  assert.deepEqual(typed("let a = r#\"say \"hi\" // not a comment\"#; let b = r\"x\";"), [
    ["keyword", "let"],
    ["string", "r#\"say \"hi\" // not a comment\"#"],
    ["keyword", "let"],
    ["string", "r\"x\""],
  ]);
  assert.deepEqual(typed("r##\"a \"# b\"##"), [["string", "r##\"a \"# b\"##"]]);
});

test("distinguishes lifetimes from char literals", () => {
  assert.deepEqual(typed("fn f<'a>(x: &'a str) -> char { 'a' }"), [
    ["keyword", "fn"],
    ["function", "f"],
    ["lifetime", "'a"],
    ["lifetime", "'a"],
    ["type", "str"],
    ["type", "char"],
    ["char", "'a'"],
  ]);
  assert.deepEqual(typed("let q = '\\''; let n = '\\n'; let s: &'static str;"), [
    ["keyword", "let"],
    ["char", "'\\''"],
    ["keyword", "let"],
    ["char", "'\\n'"],
    ["keyword", "let"],
    ["lifetime", "'static"],
    ["type", "str"],
  ]);
});

test("separates doc comments, line comments, and nested block comments", () => {
  assert.deepEqual(typed("//! Crate docs\n/// Item docs\n// plain\n//// plain too\n/* a /* b */ c */ x"), [
    ["doc-comment", "//! Crate docs"],
    ["doc-comment", "/// Item docs"],
    ["comment", "// plain"],
    ["comment", "//// plain too"],
    ["comment", "/* a /* b */ c */"],
  ]);
});

test("highlights attributes, macros, types, constants, and numbers", () => {
  assert.deepEqual(
    typed("#[derive(Component, Default)]\nconst MAX_RAYS: usize = 1_024;\nprintln!(\"{}\", 0.5f32 + 1e-3 + 0xFF);"),
    [
      ["attribute", "#[derive(Component, Default)]"],
      ["keyword", "const"],
      ["constant", "MAX_RAYS"],
      ["type", "usize"],
      ["number", "1_024"],
      ["macro", "println!"],
      ["string", "\"{}\""],
      ["number", "0.5f32"],
      ["number", "1e-3"],
      ["number", "0xFF"],
    ],
  );
  assert.deepEqual(typed("#![doc = \"a ] b\"] x"), [["attribute", "#![doc = \"a ] b\"]"]]);
});

test("does not swallow ranges, method calls, or != into numbers and macros", () => {
  assert.deepEqual(typed("for i in 0..10 { a.0.max(1) }"), [
    ["keyword", "for"],
    ["keyword", "in"],
    ["number", "0"],
    ["number", "10"],
    ["number", "0"],
    ["function", "max"],
    ["number", "1"],
  ]);
  assert.deepEqual(typed("if a != b {}"), [["keyword", "if"]]);
});

test("identifiers containing r or b are not string prefixes", () => {
  assert.deepEqual(typed("bar\"x\""), [["string", "\"x\""]]);
  assert.deepEqual(typed("let r#type = br\"\\d\";"), [
    ["keyword", "let"],
    ["string", "br\"\\d\""],
  ]);
});
