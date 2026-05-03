const fs = require("fs");
const path = require("path");
const { createRequire } = require("module");

const repoRoot = path.resolve(__dirname, "..", "..");
const nodeRoot =
  process.env.QORX_ADAPTER_NODE_ROOT ||
  path.join(repoRoot, "target", "adapter-proof-node");
const requireFromAdapter = createRequire(path.join(nodeRoot, "package.json"));
const ort = requireFromAdapter("onnxruntime-node");
const { onnx } = requireFromAdapter("onnx-proto");

function tensorValueInfo(name) {
  return onnx.ValueInfoProto.create({
    name,
    type: onnx.TypeProto.create({
      tensorType: {
        elemType: onnx.TensorProto.DataType.FLOAT,
        shape: { dim: [{ dimValue: 2 }] },
      },
    }),
  });
}

function writeAddModel(modelPath) {
  const model = onnx.ModelProto.create({
    irVersion: 7,
    producerName: "qorx-adapter-proof",
    opsetImport: [{ domain: "", version: 13 }],
    graph: {
      name: "qorx_add_graph",
      input: [tensorValueInfo("x")],
      output: [tensorValueInfo("y")],
      initializer: [
        {
          name: "bias",
          dataType: onnx.TensorProto.DataType.FLOAT,
          dims: [2],
          rawData: Buffer.from(new Float32Array([1, 2]).buffer),
        },
      ],
      node: [
        {
          opType: "Add",
          input: ["x", "bias"],
          output: ["y"],
          name: "add_bias",
        },
      ],
    },
  });
  const bytes = onnx.ModelProto.encode(model).finish();
  fs.mkdirSync(path.dirname(modelPath), { recursive: true });
  fs.writeFileSync(modelPath, bytes);
  return bytes.length;
}

async function main() {
  const modelPath = path.join(nodeRoot, "qorx-add.onnx");
  const bytes = writeAddModel(modelPath);
  const session = await ort.InferenceSession.create(modelPath);
  const feeds = {
    x: new ort.Tensor("float32", Float32Array.from([3, 4]), [2]),
  };
  const outputs = await session.run(feeds);
  const y = Array.from(outputs.y.data);
  const status = y.length === 2 && y[0] === 4 && y[1] === 6 ? "pass" : "fail";

  console.log(
    JSON.stringify(
      {
        adapter: "onnxruntime-node",
        status,
        model_path: modelPath,
        model_bytes: bytes,
        inputs: Object.keys(feeds),
        outputs: Object.keys(outputs),
        y,
        boundary:
          "This proves ONNX Runtime can execute a local model. It is a compressor-runtime proof point, not proof of a trained Qorx compressor.",
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
