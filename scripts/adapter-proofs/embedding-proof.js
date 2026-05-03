const path = require("path");
const { createRequire } = require("module");

const repoRoot = path.resolve(__dirname, "..", "..");
const nodeRoot =
  process.env.QORX_ADAPTER_NODE_ROOT ||
  path.join(repoRoot, "target", "adapter-proof-node");
const requireFromAdapter = createRequire(path.join(nodeRoot, "package.json"));
const { pipeline, env } = requireFromAdapter("@xenova/transformers");

function dot(a, b) {
  let value = 0;
  for (let i = 0; i < a.length; i += 1) {
    value += a[i] * b[i];
  }
  return value;
}

async function main() {
  env.cacheDir = path.join(nodeRoot, "model-cache");
  const extractor = await pipeline("feature-extraction", "Xenova/all-MiniLM-L6-v2");
  const output = await extractor(
    [
      "qorx saves prompt tokens with local context handles",
      "banana bread recipe with ripe fruit",
    ],
    { pooling: "mean", normalize: true }
  );
  const data = output.tolist();
  const row0 = data[0];
  const row1 = data[1];
  const selfSimilarity = dot(row0, row0);
  const crossSimilarity = dot(row0, row1);
  const status =
    data.length === 2 &&
    row0.length === 384 &&
    selfSimilarity > 0.99 &&
    crossSimilarity < 0.95
      ? "pass"
      : "fail";

  console.log(
    JSON.stringify(
      {
        adapter: "embedding-backend",
        backend: "@xenova/transformers",
        model: "Xenova/all-MiniLM-L6-v2",
        status,
        dims: [data.length, row0.length],
        self_similarity: Number(selfSimilarity.toFixed(6)),
        cross_similarity: Number(crossSimilarity.toFixed(6)),
        cache_dir: env.cacheDir,
        boundary:
          "This proves a local dense embedding backend can run. Qorx core still defaults to deterministic sparse vectors unless an embedding adapter is explicitly configured.",
      },
      null,
      2
    )
  );
}

main().catch((error) => {
  console.error(error && error.stack ? error.stack : String(error));
  process.exit(1);
});
