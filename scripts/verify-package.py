"""验证官方安装包的清单、文件摘要及 Linux x86_64 可执行文件，不解压至磁盘。"""
import argparse
import hashlib
import json
import pathlib
import struct
import tarfile


def verify(path):
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    expected = pathlib.Path(str(path) + ".sha256").read_text(encoding="utf-8-sig").split()[0]
    assert digest == expected, "安装包 SHA-256 不匹配"
    with tarfile.open(path, "r:gz") as archive:
        members = archive.getmembers()
        names = [m.name for m in members]
        assert len(names) == len(set(names)), "包内路径重复"
        assert all(m.isfile() and not pathlib.PurePosixPath(m.name).is_absolute() and ".." not in pathlib.PurePosixPath(m.name).parts for m in members), "包内文件类型或路径无效"
        manifest = json.load(archive.extractfile("plugin.json"))
        package = manifest["package"]
        assert package["protocolVersion"] == 7
        assert package["target"] == {"os": "linux", "architecture": "x86_64"}
        assert manifest["publisher"] + "." + manifest["name"] == "belofr.enterprise-ledger"
        assert set(names) == set(package["files"]) | {"plugin.json"}
        for name, file_digest in package["files"].items():
            data = archive.extractfile(name).read()
            assert hashlib.sha256(data).hexdigest() == file_digest, f"文件摘要不匹配：{name}"
        main = manifest["main"]
        binary = archive.extractfile(main).read()
        assert archive.getmember(main).mode & 0o111, "缺少可执行权限"
        assert binary[:6] == b"\x7fELF\x02\x01" and struct.unpack_from("<H", binary, 18)[0] == 62, "不是 ELF64 x86_64"
    print(json.dumps({"package": str(path), "sha256": digest, "bytes": path.stat().st_size, "files": len(names), "protocol": 7, "elf": "Linux x86_64", "verified": True}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("package", type=pathlib.Path)
    verify(parser.parse_args().package)
