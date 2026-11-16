"""生成源码 ZIP，按允许的目录收集，排除工具、缓存、依赖、构建输出与仓库元数据。"""
import hashlib
import json
import os
import pathlib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
EXCLUDED = {".git", ".tools", ".runtime", "target", "target-linux", "node_modules", "dist", "__pycache__"}
FILES = [".gitattributes", ".gitignore", "LICENSE", "THIRD_PARTY_NOTICES.md", "README.md", "README.en.md", "plugin.json"]
DIRECTORIES = ["backend", "frontend", "scripts", "docs", "examples"]


def main():
    version = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))["version"]
    output = ROOT / "dist" / f"enterprise-ledger-{version}-source.zip"
    output.parent.mkdir(parents=True, exist_ok=True)
    selected = [ROOT / name for name in FILES]
    for directory in DIRECTORIES:
        for folder, subdirectories, filenames in os.walk(ROOT / directory):
            subdirectories[:] = [name for name in subdirectories if name not in EXCLUDED]
            selected.extend(pathlib.Path(folder) / name for name in filenames if not name.endswith((".tsbuildinfo", ".pyc")))
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(selected):
            archive.write(path, (pathlib.Path("enterprise-ledger") / path.relative_to(ROOT)).as_posix())
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    pathlib.Path(str(output) + ".sha256").write_text(f"{digest}  {output.name}\n", encoding="utf-8")
    print(f"{output}\n{len(selected)} source files\nSHA-256 {digest}")


if __name__ == "__main__":
    main()
