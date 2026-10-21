"""Download pinned, public evaluation artifacts only; never modify installer resources.
Records upstream LFS SHA256 and verifies the downloaded ONNX weights. No remote code.
"""
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
CANDIDATES = {
    "multilingual-e5-small": ("Xenova/multilingual-e5-small", "761b726dd34fb83930e26aab4e9ac3899aa1fa78", "query: ", "passage: "),
    "multilingual-minilm": ("Xenova/paraphrase-multilingual-MiniLM-L12-v2", "2c4055b12046f11709e9df2c122e59ffbdc2f900", "", ""),
}

def fetch(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "vibeKidbright-local-evaluation"}), timeout=120)

def main():
    for name, (repo, revision, query, passage) in CANDIDATES.items():
        directory = ROOT / "src-tauri/target/retrieval-candidates" / name
        directory.mkdir(parents=True, exist_ok=True)
        with fetch(f"https://huggingface.co/api/models/{repo}/revision/{revision}?blobs=true") as response:
            metadata = json.load(response)
        card = metadata.get("cardData", {})
        base = "intfloat/multilingual-e5-small" if name == "multilingual-e5-small" else "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2"
        with fetch(f"https://huggingface.co/api/models/{base}") as response:
            upstream = json.load(response)
        license_name = card.get("license") or upstream.get("cardData", {}).get("license")
        if license_name not in {"mit", "apache-2.0"}:
            raise RuntimeError(f"Review candidate license before download: {repo}: {card.get('license')}")
        artifacts = []
        for remote, local in [("onnx/model_quantized.onnx", "model.onnx"), ("config.json", "model-config.json"),
                              ("tokenizer.json", "tokenizer.json"), ("tokenizer_config.json", "tokenizer_config.json"),
                              ("special_tokens_map.json", "special_tokens_map.json")]:
            source = next(item for item in metadata["siblings"] if item["rfilename"] == remote)
            expected = source.get("lfs", {}).get("sha256")
            target = directory / local
            if not target.exists() or (expected and hashlib.sha256(target.read_bytes()).hexdigest() != expected):
                print(f"Downloading pinned {name}/{remote}", flush=True)
                temporary = target.with_suffix(target.suffix + ".partial")
                with fetch(f"https://huggingface.co/{repo}/resolve/{revision}/{remote}") as response, temporary.open("wb") as output:
                    while data := response.read(1024 * 1024):
                        output.write(data)
                if expected and hashlib.sha256(temporary.read_bytes()).hexdigest() != expected:
                    raise RuntimeError(f"Artifact digest mismatch: {remote}")
                temporary.replace(target)
            artifacts.append({"file": local, "source": remote, "bytes": target.stat().st_size,
                              "sha256": hashlib.sha256(target.read_bytes()).hexdigest(), "upstream_lfs_sha256": expected})
        record = {"repository": repo, "revision": revision, "license": license_name,
                  "license_source": base, "base_revision": upstream["sha"], "conversion_license_declared": card.get("license"),
                  "model_identity": f"{repo}@{revision}:384:mean:quantized", "query_prefix": query,
                  "passage_prefix": passage, "artifacts": artifacts, "installer_model_replaced": False}
        (directory / "manifest.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        print(f"Verified {name}: {sum(a['bytes'] for a in artifacts)} bytes; base license {license_name}", flush=True)

if __name__ == "__main__":
    main()
