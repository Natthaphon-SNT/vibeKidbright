"""
build_kb_embeddings.py
======================
สร้าง RAG vector index จากไฟล์ใน resources/knowledge_base/
โดยใช้ sentence-transformers (local model, ไม่ต้องการ API key)

Output: resources/knowledge_base/.embeddings.json
Format: ตรงกับที่ Rust / ai_chat.rs อ่านได้ (VectorIndex struct)

ติดตั้ง dependencies ก่อน:
    pip install sentence-transformers

รัน (จาก root ของ project):
    python build_kb_embeddings.py

จะ index ไฟล์ที่ถูกเปลี่ยนแปลงเท่านั้น (ตรวจจาก mtime)
"""

import json
import os
import time
import re
import sys
from pathlib import Path

if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')
if hasattr(sys.stderr, 'reconfigure'):
    sys.stderr.reconfigure(encoding='utf-8')

# ── Config ────────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent
KB_DIR = SCRIPT_DIR / "resources" / "knowledge_base"
INDEX_FILE = KB_DIR / ".embeddings.json"

MODEL_NAME = "sentence-transformers/all-MiniLM-L6-v2"   # 80 MB, ดีที่สุดสำหรับ general text
CHUNK_TARGET = 600   # ตัวอักษรต่อ chunk (ใกล้เคียง ~800 ใน Rust)
CHUNK_OVERLAP = 80   # ตัวอักษร overlap ระหว่าง chunk

SUPPORTED_EXTENSIONS = {".md", ".txt", ".c", ".h"}

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
    """
    result = []
    for path in sorted(kb_dir.rglob("*")):
        if not path.is_file():
            continue
        name = path.name
        if name.startswith(".") or name.endswith(".disabled"):
            continue
        if path.suffix.lower() not in SUPPORTED_EXTENSIONS:
            continue
        rel_key = path.relative_to(kb_dir).as_posix()
        result.append((path, rel_key))
    return result


# ── Main indexer ──────────────────────────────────────────────────────────────

def main():
    if not KB_DIR.exists():
        print(f"ERROR: ไม่พบ knowledge_base directory: {KB_DIR}")
        sys.exit(1)

    print(f"Knowledge Base: {KB_DIR}")
    print(f"Index file:     {INDEX_FILE}\n")

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

    # โหลด index เดิม
    if INDEX_FILE.exists():
        with open(INDEX_FILE, encoding="utf-8") as f:
            try:
                index = json.load(f)
            except json.JSONDecodeError:
                index = {"chunks": [], "last_indexed": {}}
    else:
        index = {"chunks": [], "last_indexed": {}}

    existing_chunks = index.get("chunks", [])
    last_indexed = index.get("last_indexed", {})

    all_files = collect_kb_files(KB_DIR)
    print(f"พบไฟล์ทั้งหมด: {len(all_files)} ไฟล์\n")

    changed = False
    new_chunks = []

    for file_path, rel_key in all_files:
        mtime = int(file_path.stat().st_mtime)
        last_mtime = last_indexed.get(rel_key, 0)

        if last_mtime >= mtime:
            old = [c for c in existing_chunks if c.get("file_name") == rel_key]
            new_chunks.extend(old)
            print(f"  SKIP (unchanged): {rel_key}  [{len(old)} chunks]")
            continue

        print(f"  INDEXING: {rel_key}")
        try:
            text = file_path.read_text(encoding="utf-8", errors="replace")
        except Exception as e:
            print(f"     WARNING: อ่านไฟล์ไม่ได้: {e}")
            continue

        text_chunks = chunk_text(text)
        print(f"     -> {len(text_chunks)} chunks")

        embeddings = model.encode(text_chunks, show_progress_bar=False, batch_size=16)

        for chunk_str, emb in zip(text_chunks, embeddings):
            new_chunks.append({
                "file_name": rel_key,
                "content": chunk_str,
                "embedding": emb.tolist()
            })

        last_indexed[rel_key] = mtime
        changed = True

    if not changed:
        print("\nทุกไฟล์ถูก index ไว้แล้ว ไม่มีการเปลี่ยนแปลง")
        print(f"Vector index มี {len(new_chunks)} chunks")
        return

    output = {
        "chunks": new_chunks,
        "last_indexed": last_indexed
    }

    print(f"\nกำลังเขียน index...")
    with open(INDEX_FILE, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=2)

    size_kb = INDEX_FILE.stat().st_size / 1024
    print(f"\nสร้าง embedding สำเร็จ!")
    print(f"   {len(new_chunks)} chunks จาก {len(all_files)} ไฟล์")
    print(f"   ขนาดไฟล์: {size_kb:.1f} KB")
    print(f"   บันทึกที่: {INDEX_FILE}")
    if new_chunks:
        print(f"   Embedding dimension: {len(new_chunks[0]['embedding'])}")


if __name__ == "__main__":
    start = time.time()
    main()
    elapsed = time.time() - start
    print(f"\nใช้เวลา: {elapsed:.1f} วินาที")
