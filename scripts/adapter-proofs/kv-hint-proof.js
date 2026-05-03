const fs = require("fs");
const path = require("path");

function readSafetensors(filePath) {
  const bytes = fs.readFileSync(filePath);
  if (bytes.length < 8) {
    throw new Error("safetensors file is too short");
  }
  const headerLength = Number(bytes.readBigUInt64LE(0));
  const headerStart = 8;
  const headerEnd = headerStart + headerLength;
  if (headerEnd > bytes.length) {
    throw new Error("safetensors header exceeds file length");
  }
  const header = JSON.parse(bytes.subarray(headerStart, headerEnd).toString("utf8"));
  const tensors = Object.entries(header).filter(([key]) => key !== "__metadata__");
  for (const [name, spec] of tensors) {
    if (spec.dtype !== "U8") {
      throw new Error(`${name} is ${spec.dtype}, expected U8`);
    }
    if (!Array.isArray(spec.data_offsets) || spec.data_offsets.length !== 2) {
      throw new Error(`${name} has invalid data_offsets`);
    }
    const [start, end] = spec.data_offsets;
    if (start < 0 || end < start || headerEnd + end > bytes.length) {
      throw new Error(`${name} offsets are outside the payload`);
    }
    const payload = bytes.subarray(headerEnd + start, headerEnd + end);
    const decoded = JSON.parse(payload.toString("utf8"));
    if (!decoded.id || !decoded.cache_key || !decoded.quantization_hint) {
      throw new Error(`${name} payload is not a Qorx KV hint`);
    }
  }
  return { bytes, header, tensors };
}

const filePath = process.argv[2]
  ? path.resolve(process.argv[2])
  : path.resolve("target", "adapter-proof-node", "qorx-kv-proof.safetensors");
const { bytes, header, tensors } = readSafetensors(filePath);
const metadata = header.__metadata__ || {};
const status =
  metadata.format === "qorx-kv-hints" &&
  metadata.realized_kv_compression === "false" &&
  tensors.length > 0
    ? "pass"
    : "fail";

console.log(
  JSON.stringify(
    {
      adapter: "kv-hint-safetensors",
      status,
      path: filePath,
      bytes: bytes.length,
      metadata,
      tensor_count: tensors.length,
      tensor_names: tensors.map(([name]) => name),
      boundary:
        "This proves Qorx's safetensors-compatible KV hint artifact is readable. It is not a vLLM/TurboQuant runtime memory-saving measurement.",
    },
    null,
    2
  )
);
