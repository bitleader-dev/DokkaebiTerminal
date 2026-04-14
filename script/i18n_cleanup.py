"""로케일 키 불일치 분석 — 미사용 키 식별 + 번역 누락 식별."""
import re
import subprocess
from pathlib import Path


def load_keys(path):
    text = Path(path).read_text(encoding="utf-8")
    return text, set(re.findall(r'^\s*"([^"]+)":', text, re.MULTILINE))


def is_used(key, project_root=Path("crates")):
    """정확한 따옴표 포함 문자열이 Rust 코드에서 참조되는지 검사"""
    pattern = f'"{key}"'
    try:
        r = subprocess.run(
            ["grep", "-rl", "--include=*.rs", pattern, str(project_root)],
            capture_output=True, text=True, encoding="utf-8", errors="replace"
        )
        return r.returncode == 0 and r.stdout.strip() != ""
    except Exception:
        return True


def main():
    _, en_keys = load_keys("assets/locales/en.json")
    _, ko_keys = load_keys("assets/locales/ko.json")

    ko_only = sorted(ko_keys - en_keys)
    en_only = sorted(en_keys - ko_keys)

    ko_used, ko_unused = [], []
    for k in ko_only:
        (ko_used if is_used(k) else ko_unused).append(k)

    en_used, en_unused = [], []
    for k in en_only:
        (en_used if is_used(k) else en_unused).append(k)

    print(f"# KO only: {len(ko_only)} (used {len(ko_used)}, unused {len(ko_unused)})")
    print(f"# EN only: {len(en_only)} (used {len(en_used)}, unused {len(en_unused)})")
    print()
    print("## KO used (keep + need en translation):")
    for k in ko_used:
        print(f"  {k}")
    print("\n## EN used (keep + need ko translation):")
    for k in en_used:
        print(f"  {k}")
    print("\n## KO unused sample (removable):")
    for k in ko_unused[:5]:
        print(f"  {k}")
    print(f"  ... ({len(ko_unused)} total)")
    print("\n## EN unused (removable):")
    for k in en_unused:
        print(f"  {k}")


if __name__ == "__main__":
    main()
