"""
build_kb_embeddings.py
======================
สร้าง RAG vector index จากไฟล์ใน knowledge_base/
โดยใช้ sentence-transformers (local model, ไม่ต้องการ API key)

Output: resources/knowledge_base/.embeddings.json
        knowledge_base/.embeddings.json  (dev copy)
Format: ตรงกับที่ Rust / ai_chat.rs อ่านได้ (VectorIndex struct)

ติดตั้ง dependencies ก่อน:
    pip install sentence-transformers

รัน (จาก root ของ project):
    python build_kb_embeddings.py              # index ทั้ง 2 ตำแหน่ง
    python build_kb_embeddings.py --force      # force re-index ทุกไฟล์
    python build_kb_embeddings.py --kb-dir ./my_kb  # ระบุ path เอง

จะ index ไฟล์ที่ถูกเปลี่ยนแปลงเท่านั้น (ตรวจจาก mtime)
"""

import json
import os
import time
import re
import sys
import argparse
from pathlib import Path

if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8')

# ── Config ────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent

# ที่เก็บ KB สำหรับ CI/CD (Tauri bundled resources)
RESOURCES_KB_DIR = SCRIPT_DIR / "resources" / "knowledge_base"
# ที่เก็บ KB สำหรับ dev (source of truth)
DEV_KB_DIR = SCRIPT_DIR / "knowledge_base"

MODEL_NAME = "sentence-transformers/all-MiniLM-L6-v2"   # 80 MB, ดีสำหรับ general text
CHUNK_TARGET = 600   # ตัวอักษรต่อ chunk (ใกล้เคียง ~800 ใน Rust)
CHUNK_OVERLAP = 80   # ตัวอักษร overlap ระหว่าง chunk

SUPPORTED_EXTENSIONS = {".md", ".txt", ".c", ".h"}

# ── CLI args ──────────────────────────────────────────────────────────────────

def parse_args():
    parser = argparse.ArgumentParser(description="Build KB embeddings index")
    parser.add_argument(
        "--kb-dir",
        type=Path,
        help="Override knowledge_base directory path (ค่าเริ่มต้น: auto-detect)"
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="Force re-index all files (ignores mtime cache)"
    )
    parser.add_argument(
        "--no-sync",
        action="store_true",
        help="ไม่ sync ไปยัง resources/knowledge_base/ (dev only)"
    )
    return parser.parse_args()

# ── Chunking ──────────────────────────────────────────────────────────────────

def chunk_text(text: str, target_size: int = CHUNK_TARGET, overlap: int = CHUNK_OVERLAP) -> list:
    """
    แบ่ง text ออกเป็น chunks โดยยึดขอบ paragraph/heading
    เพื่อให้ semantic context ของแต่ละ chunk สมบูรณ์
    """
    chunks = []
    current = ""

    # แยกด้วย heading (## xxx) หรือ blank line เป็น natural boundary
    paragraphs = re.split(r'\n{2,}', text.strip())

    for para in paragraphs:
        para = para.strip()
        if not para:
            continue

        if len(current) + len(para) + 2 <= target_size:
            current = (current + "\n\n" + para).strip()
        else:
            if current:
                chunks.append(current)
            # เริ่ม chunk ใหม่ พร้อม overlap จาก chunk ก่อน
            if overlap > 0 and chunks:
                tail = chunks[-1][-overlap:]
                current = (tail + "\n\n" + para).strip()
            else:
                current = para

    if current:
        chunks.append(current)

    # fallback: ไม่มี paragraph break
    if not chunks:
        chunks = [text[i:i + target_size] for i in range(0, len(text), target_size - overlap)]

    return [c for c in chunks if len(c.strip()) > 20]


# ── File collector ────────────────────────────────────────────────────────────

def collect_kb_files(kb_dir: Path):
    """
    รวบรวมไฟล์ทั้งหมดใน knowledge_base recursively
    คืนค่า list ของ (absolute_path, relative_key)
    ข้าม .disabled files และ hidden files
    """
    result = []
    for path in sorted(kb_dir.rglob("*")):
        if not path.is_file():
            continue
        name = path.name
        if name.startswith(".") or name.endswith(".disabled") or name.endswith(".backup"):
            continue
        if path.suffix.lower() not in SUPPORTED_EXTENSIONS:
            continue
        rel_key = path.relative_to(kb_dir).as_posix()
        result.append((path, rel_key))
    return result


# ── Index builder ─────────────────────────────────────────────────────────────

def build_index(kb_dir: Path, index_file: Path, model, force: bool = False) -> dict:
    """
    สร้างหรืออัพเดท vector index สำหรับ KB directory ที่กำหนด
    คืนค่า index dict (chunks + last_indexed)
    """
    # โหลด index เดิม
    if index_file.exists() and not force:
        with open(index_file, encoding="utf-8") as f:
            try:
                index = json.load(f)
            except json.JSONDecodeError:
                print(f"  ⚠️ Index file corrupt, rebuilding: {index_file}")
                index = {"chunks": [], "last_indexed": {}}
    else:
        index = {"chunks": [], "last_indexed": {}}

    existing_chunks = index.get("chunks", [])
    last_indexed = index.get("last_indexed", {})

    all_files = collect_kb_files(kb_dir)
    print(f"  พบไฟล์ทั้งหมด: {len(all_files)} ไฟล์")

    changed = False
    new_chunks = []

    for file_path, rel_key in all_files:
        mtime = int(file_path.stat().st_mtime)
        last_mtime = last_indexed.get(rel_key, 0)

        if not force and last_mtime >= mtime:
            old = [c for c in existing_chunks if c.get("file_name") == rel_key]
            new_chunks.extend(old)
            print(f"    SKIP (unchanged): {rel_key}  [{len(old)} chunks]")
            continue

        print(f"    INDEXING: {rel_key}")
        try:
            text = file_path.read_text(encoding="utf-8", errors="replace")
        except Exception as e:
            print(f"       WARNING: อ่านไฟล์ไม่ได้: {e}")
            continue

        text_chunks = chunk_text(text)
        print(f"       -> {len(text_chunks)} chunks")

        embeddings = model.encode(text_chunks, show_progress_bar=False, batch_size=16)

        for chunk_str, emb in zip(text_chunks, embeddings):
            new_chunks.append({
                "file_name": rel_key,
                "content": chunk_str,
                "embedding": emb.tolist()
            })

        last_indexed[rel_key] = mtime
        changed = True

    # ลบ chunks ของไฟล์ที่ถูกลบออกไปแล้ว
    active_keys = {rel_key for _, rel_key in all_files}
    filtered_chunks = [c for c in new_chunks if c.get("file_name") in active_keys]
    if len(filtered_chunks) != len(new_chunks):
        print(f"  ลบ chunks ของไฟล์ที่ถูกลบ: {len(new_chunks) - len(filtered_chunks)} chunks")
        changed = True

    return {
        "chunks": filtered_chunks,
        "last_indexed": {k: v for k, v in last_indexed.items() if k in active_keys},
        "changed": changed,
    }


def save_index(index_file: Path, index_data: dict):
    """บันทึก index ลงไฟล์ (atomic write ผ่าน temp file)"""
    output = {
        "chunks": index_data["chunks"],
        "last_indexed": index_data["last_indexed"],
    }
    # Atomic write: เขียนไป .tmp ก่อน แล้วค่อย rename
    tmp_file = index_file.with_suffix(".json.tmp")
    with open(tmp_file, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)
    tmp_file.replace(index_file)

    size_kb = index_file.stat().st_size / 1024
    print(f"    บันทึกที่: {index_file}")
    print(f"    ขนาดไฟล์: {size_kb:.1f} KB")
    print(f"    {len(output['chunks'])} chunks จาก {len(output['last_indexed'])} ไฟล์")
    if output["chunks"]:
        print(f"    Embedding dimension: {len(output['chunks'][0]['embedding'])}")


def sync_index(src_index_file: Path, dst_kb_dir: Path, dst_index_file: Path):
    """
    Sync .embeddings.json จาก dev KB ไปยัง resources/knowledge_base/
    เฉพาะ entries ของไฟล์ที่มีใน dst เท่านั้น
    """
    if not src_index_file.exists():
        print(f"  SYNC: ไม่พบ source index {src_index_file} — ข้าม")
        return

    with open(src_index_file, encoding="utf-8") as f:
        src_index = json.load(f)

    # หาไฟล์ที่มีใน dst KB
    dst_files_set = {
        path.relative_to(dst_kb_dir).as_posix()
        for path in dst_kb_dir.rglob("*")
        if path.is_file() and not path.name.startswith(".")
        and not path.name.endswith(".disabled")
        and path.suffix.lower() in SUPPORTED_EXTENSIONS
    } if dst_kb_dir.exists() else set()

    filtered_chunks = [c for c in src_index.get("chunks", []) if c.get("file_name") in dst_files_set]
    filtered_indexed = {k: v for k, v in src_index.get("last_indexed", {}).items() if k in dst_files_set}

    dst_index_file.parent.mkdir(parents=True, exist_ok=True)
    tmp_file = dst_index_file.with_suffix(".json.tmp")
    with open(tmp_file, "w", encoding="utf-8") as f:
        json.dump({"chunks": filtered_chunks, "last_indexed": filtered_indexed}, f, ensure_ascii=False, indent=2)
    tmp_file.replace(dst_index_file)

    size_kb = dst_index_file.stat().st_size / 1024
    print(f"  SYNC -> {dst_index_file} ({len(filtered_chunks)} chunks, {size_kb:.1f} KB)")


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    args = parse_args()

    # ── กำหนด KB directories ที่จะ index ──────────────────────────────────────
    if args.kb_dir:
        # User-specified path
        kb_targets = [(args.kb_dir.resolve(), args.kb_dir.resolve() / ".embeddings.json")]
    else:
        # Auto-detect: index DEV KB dir (source of truth)
        kb_targets = []
        if DEV_KB_DIR.exists():
            kb_targets.append((DEV_KB_DIR, DEV_KB_DIR / ".embeddings.json"))
        else:
            print(f"⚠️ ไม่พบ DEV KB dir: {DEV_KB_DIR}")

    if not kb_targets:
        print("ERROR: ไม่พบ knowledge_base directory ใดเลย")
        sys.exit(1)

    # ── โหลด model ────────────────────────────────────────────────────────────
    try:
        from sentence_transformers import SentenceTransformer
    except ImportError:
        print("ERROR: ไม่พบ sentence-transformers กรุณารัน:")
        print("   pip install sentence-transformers")
        sys.exit(1)

    print(f"กำลังโหลด model: {MODEL_NAME}")
    print("(ครั้งแรกจะดาวน์โหลด ~80 MB อัตโนมัติ)")
    model = SentenceTransformer(MODEL_NAME)
    print(f"โหลด model สำเร็จ\n")

    # ── Build index สำหรับแต่ละ KB ────────────────────────────────────────────
    dev_index_file = None
    for kb_dir, index_file in kb_targets:
        print(f"\n{'='*60}")
        print(f"KB Directory: {kb_dir}")
        print(f"Index file:   {index_file}")
        print(f"Force:        {args.force}")
        print(f"{'='*60}")

        result = build_index(kb_dir, index_file, model, force=args.force)

        if result["changed"]:
            save_index(index_file, result)
            print(f"\n✅ สร้าง/อัพเดท embedding สำเร็จ!")
        else:
            print(f"\n✅ ทุกไฟล์ถูก index ไว้แล้ว ({len(result['chunks'])} chunks, ไม่มีการเปลี่ยนแปลง)")

        if kb_dir == DEV_KB_DIR:
            dev_index_file = index_file

    # ── Sync ไปยัง resources/knowledge_base/ ─────────────────────────────────
    if not args.no_sync and dev_index_file and not args.kb_dir:
        print(f"\n{'='*60}")
        print(f"Syncing index to resources/knowledge_base/...")
        print(f"{'='*60}")

        if RESOURCES_KB_DIR.exists():
            sync_index(
                src_index_file=dev_index_file,
                dst_kb_dir=RESOURCES_KB_DIR,
                dst_index_file=RESOURCES_KB_DIR / ".embeddings.json",
            )
        else:
            # สร้าง resources/knowledge_base/ และ copy ไฟล์จาก dev KB ไป
            print(f"  สร้าง resources/knowledge_base/ ใหม่ และ copy files...")
            RESOURCES_KB_DIR.mkdir(parents=True, exist_ok=True)
            # Copy KB files from dev to resources
            import shutil
            for src_path, rel_key in collect_kb_files(DEV_KB_DIR):
                dst_path = RESOURCES_KB_DIR / rel_key
                dst_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src_path, dst_path)
            # Sync index
            sync_index(
                src_index_file=dev_index_file,
                dst_kb_dir=RESOURCES_KB_DIR,
                dst_index_file=RESOURCES_KB_DIR / ".embeddings.json",
            )
            print(f"  สร้าง resources/knowledge_base/ สำเร็จ")

    print(f"\n{'='*60}")
    print("เสร็จสิ้น! ผลลัพธ์:")
    print(f"  - knowledge_base/.embeddings.json")
    if not args.no_sync and RESOURCES_KB_DIR.exists():
        print(f"  - resources/knowledge_base/.embeddings.json")
    print("แนะนำ: รัน script นี้ทุกครั้งที่แก้ไขไฟล์ใน knowledge_base/")


if __name__ == "__main__":
    start = time.time()
    main()
    elapsed = time.time() - start
    print(f"\nใช้เวลา: {elapsed:.1f} วินาที")
