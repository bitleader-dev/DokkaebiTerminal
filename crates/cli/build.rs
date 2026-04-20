#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
use std::process::Command;

fn main() {
    if std::env::var("ZED_UPDATE_EXPLANATION").is_ok() {
        println!(r#"cargo:rustc-cfg=feature="no-bundled-uninstall""#);
    }

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15.7");
    }

    // Populate git sha environment variable if git is available
    println!("cargo:rerun-if-changed=../../.git/logs/HEAD");
    if let Some(output) = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let git_sha = String::from_utf8_lossy(&output.stdout);
        let git_sha = git_sha.trim();

        println!("cargo:rustc-env=ZED_COMMIT_SHA={git_sha}");
    }
    if let Some(build_identifier) = option_env!("GITHUB_RUN_NUMBER") {
        println!("cargo:rustc-env=ZED_BUILD_ID={build_identifier}");
    }

    // Windows 전용: dokkaebi-cli.exe 바이너리에만 메인 dokkaebi.exe와 동일한
    // 파일 아이콘 및 제품 메타데이터를 임베드한다. 아이콘 원본은 zed 크레이트와
    // 공유한다.
    //
    // cli 크레이트는 [lib] + [[bin]] 이중 구조이고 zed 크레이트가 cli lib를
    // 의존하므로, 일반적인 `rustc-link-lib` 방식은 리소스가 dokkaebi.exe에까지
    // 전파되어 zed의 VERSION 리소스와 CVT1100 충돌을 일으킨다. 대신
    // `embed_resource::compile_for`는 내부적으로 `rustc-link-arg-bin=<bin>=<lib>`
    // 를 emit하여 지정된 바이너리 타겟에만 리소스를 연결한다.
    if cfg!(windows) {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let icon_abs = std::path::PathBuf::from(&manifest_dir)
            .join("..")
            .join("zed")
            .join("resources")
            .join("windows")
            .join("app-icon-dokkaebi.ico");
        // rc.exe 파서 호환을 위해 파일 경로는 forward slash로 통일(이스케이프 회피).
        let icon_for_rc = icon_abs.display().to_string().replace('\\', "/");

        println!("cargo:rerun-if-changed={}", icon_abs.display());

        let out_dir = std::env::var("OUT_DIR").unwrap();
        let rc_path = std::path::PathBuf::from(&out_dir).join("dokkaebi_cli.rc");

        // FileVersion/ProductVersion은 CARGO_PKG_VERSION에서 파생.
        let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
        let ver_parts: Vec<u16> = pkg_version
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .take(4)
            .map(|s| s.parse::<u16>().unwrap_or(0))
            .collect();
        let major = ver_parts.first().copied().unwrap_or(0);
        let minor = ver_parts.get(1).copied().unwrap_or(0);
        let patch = ver_parts.get(2).copied().unwrap_or(0);
        let build_num = ver_parts.get(3).copied().unwrap_or(0);

        // `1 VERSIONINFO`의 리소스 ID 1은 Windows 탐색기/파일 속성이 읽어가는
        // 표준 위치. `1 ICON`도 동일한 관례.
        let rc_content = format!(
            r#"1 ICON "{icon}"

1 VERSIONINFO
 FILEVERSION {maj},{min},{pat},{bld}
 PRODUCTVERSION {maj},{min},{pat},{bld}
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x40004L
 FILETYPE 0x1L
 FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "FileDescription", "Dokkaebi CLI"
            VALUE "ProductName", "Dokkaebi"
            VALUE "FileVersion", "{maj}.{min}.{pat}.{bld}"
            VALUE "ProductVersion", "{maj}.{min}.{pat}.{bld}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#,
            icon = icon_for_rc,
            maj = major,
            min = minor,
            pat = patch,
            bld = build_num,
        );

        // 동일 내용이면 write 를 생략해 mtime 이 올라가지 않도록 한다.
        // embed_resource::compile_for 는 rc 의 mtime 을 근거로 재컴파일을
        // 결정하므로, 아이콘·버전·패키지 버전이 그대로인 incremental 빌드
        // 에서 리소스 재컴파일이 반복되는 것을 방지한다.
        let needs_write = std::fs::read_to_string(&rc_path)
            .map(|existing| existing != rc_content)
            .unwrap_or(true);
        if needs_write {
            if let Err(e) = std::fs::write(&rc_path, &rc_content) {
                eprintln!("cli build.rs: failed to write {}: {e}", rc_path.display());
                std::process::exit(1);
            }
        }

        // compile_for: 지정된 bin 타겟에만 리소스를 링크한다.
        // 반환값 CompilationResult가 실패를 나타내면 빌드 중단.
        let result =
            embed_resource::compile_for(&rc_path, ["dokkaebi-cli"], embed_resource::NONE);
        if let Err(e) = result.manifest_required() {
            eprintln!("cli build.rs: embed_resource::compile_for failed: {e}");
            std::process::exit(1);
        }
    }
}
