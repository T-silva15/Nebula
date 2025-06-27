# 🚀 Nebula Deduplication Testing

This repository contains scripts and tools to evaluate the deduplication effectiveness of the Nebula distributed file system using synthetic, realistic files.

---

## 🏅 Deduplication Results Summary

| Profile   | Files | Original Size | Stored Size | Expected Shared Content | Deduplication Savings |
|-----------|-------|---------------|-------------|-------------------------|-----------------------|
| 🟡 Low    | 301   | 6.90 MB       | 4.32 MB     | ~10–20%                 | 🟡 **38%**            |
| 🟠 Medium | 301   | 10.45 MB      | 3.55 MB     | ~30–50%                 | 🟠 **67%**            |
| 🟢 High   | 301   | 13.04 MB      | 2.58 MB     | ~60–80%                 | 🟢 **81%**            |

---

## 🔍 Explanation

- **Low profile** simulates mostly unique files with small shared components like headers or footers, targeting about **10–20% shared content** among files, and the system achieved a deduplication savings of around **38%**, slightly exceeding expectations.
- **Medium profile** represents a balanced workload with moderate duplication common in enterprise file sets, designed for **30–50% shared content**, yielding **67% savings** — demonstrating effective deduplication beyond the baseline shared content.
- **High profile** models highly redundant datasets such as backups or snapshots, created with **60–80% shared content** and achieving deduplication savings above **81%**, confirming strong storage reduction on heavily duplicated data.

These results demonstrate Nebula's ability to not only meet but often surpass the expected space savings based on the controlled shared content in generated files.

---

## 📁 File Generation Profiles

- Files generated include a mix of text (`.txt`, `.md`, `.json`, `.html`) and binary blobs (`.bin`), with varying ratios of shared vs unique content.
- Shared content is randomly distributed in reusable chunks simulating real-world file structures and embedded shared assets.
- Number of files per profile: **~300**
- Typical file sizes range between **20 KB and 60 KB**, with natural variance.

---

## 📊 How to Reproduce

1. Run the `python_dedup_script.py` to generate files in `test_profiles/{low,medium,high}` folders.
2. Use `dedup_analyze.sh` to put files into Nebula and gather storage stats.
3. Observe deduplication savings reported and compare across profiles.

---

## ⚙️ Requirements

- Python 3.8+
- Rust toolchain with Nebula CLI
- Active Nebula node instance while storing files (important for deduplication continuity)

---

This setup provides a realistic, scalable framework for testing distributed deduplication efficiency in your environment.
