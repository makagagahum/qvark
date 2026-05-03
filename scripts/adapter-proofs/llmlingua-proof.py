import json
import os
import warnings

warnings.filterwarnings("ignore", category=UserWarning)

from llmlingua import PromptCompressor


def main() -> None:
    model_name = os.environ.get("QORX_LLMLINGUA_MODEL", "sshleifer/tiny-gpt2")
    compressor = PromptCompressor(model_name=model_name, device_map="cpu")
    context = [
        "Qorx keeps repository context local and avoids repeatedly sending unchanged code. "
        * 12,
        "The CE planner preserves stable prefixes, exact cache keys, and readable context accounting. "
        * 12,
    ]
    result = compressor.compress_prompt(
        context,
        instruction="Answer from Qorx context.",
        question="How does Qorx reduce token burn?",
        rate=0.5,
    )
    origin_tokens = int(result.get("origin_tokens") or 0)
    compressed_tokens = int(result.get("compressed_tokens") or 0)
    status = "pass" if origin_tokens > 0 and compressed_tokens < origin_tokens else "fail"
    print(
        json.dumps(
            {
                "adapter": "llmlingua",
                "status": status,
                "model": model_name,
                "device": "cpu",
                "origin_tokens": origin_tokens,
                "compressed_tokens": compressed_tokens,
                "ratio": result.get("ratio"),
                "compressed_prompt_len": len(result.get("compressed_prompt", "")),
                "boundary": (
                    "This proves LLMLingua can run and compress a local prompt. "
                    "Quality and task-success effects still need task benchmarks."
                ),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
