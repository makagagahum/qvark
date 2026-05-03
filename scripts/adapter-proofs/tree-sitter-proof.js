const path = require("path");
const { createRequire } = require("module");

function adapterRequire(name) {
  const repoRoot = path.resolve(__dirname, "..", "..");
  const nodeRoot =
    process.env.QORX_ADAPTER_NODE_ROOT ||
    path.join(repoRoot, "target", "adapter-proof-node");
  return createRequire(path.join(nodeRoot, "package.json"))(name);
}

const Parser = adapterRequire("tree-sitter");
const Rust = adapterRequire("tree-sitter-rust");

const parser = new Parser();
parser.setLanguage(Rust);

const source = [
  "pub fn qorx_adapter_probe(input: &str) -> usize {",
  "    let token_budget = input.len() / 4;",
  "    token_budget.saturating_sub(1)",
  "}",
].join("\n");

const tree = parser.parse(source);
const treeText = tree.rootNode.toString();
const status =
  tree.rootNode.type === "source_file" &&
  treeText.includes("function_item") &&
  treeText.includes("identifier")
    ? "pass"
    : "fail";

console.log(
  JSON.stringify(
    {
      adapter: "tree-sitter",
      status,
      grammar: "tree-sitter-rust",
      root: tree.rootNode.type,
      named_children: tree.rootNode.namedChildCount,
      saw_function_item: treeText.includes("function_item"),
      saw_identifier: treeText.includes("identifier"),
      boundary:
        "This proves a real Tree-sitter Rust grammar parse. It does not replace Qorx's built-in deterministic index unless wired as an optional adapter.",
    },
    null,
    2
  )
);
